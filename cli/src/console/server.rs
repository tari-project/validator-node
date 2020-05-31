use super::{dashboard::*, Terminal};
use actix::Addr;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tari_validator_node::metrics::{GetMetrics, Metrics, MetricsConfig};
use tokio::{
    sync::{oneshot, Mutex},
    time::timeout,
};

const REFRESH_INTERVAL_MS: u64 = 500;

lazy_static::lazy_static! {
    static ref INITIALIZED: Mutex<bool> = Mutex::new(false);
}

pub struct ServerConsole {
    metrics: Addr<Metrics>,
    terminal: Terminal,
    dashboard: Dashboard,
    kill_signal: oneshot::Receiver<()>,
}

impl ServerConsole {
    /// Interactive Console for running server showing stats and logs
    ///
    /// Returns oneshot channel for kill message
    ///
    /// # Panics
    /// Should be called once during lifetime of program, otherwise will panic
    pub async fn init(metrics: Addr<Metrics>) -> oneshot::Sender<()> {
        if *INITIALIZED.lock().await {
            panic!("Tried to initialize ServerConsole when one already initalized");
        }
        let (kill_sender, kill_signal) = oneshot::channel();
        actix_rt::spawn(
            Self {
                terminal: Terminal::alternate(),
                metrics,
                dashboard: Dashboard::default(),
                kill_signal,
            }
            .run(),
        );
        kill_sender
    }

    async fn run(mut self) {
        if let Ok(tui::layout::Rect { width, .. }) = self.terminal.size() {
            self.metrics
                .send(MetricsConfig {
                    instructions_spark_sizes: width as usize,
                })
                .await
                .expect("Failed to configure terminal size");
        }
        let mut events = self
            .terminal
            .events_receiver()
            .expect("Terminal events receiver failed to setup");
        const WAIT: Duration = Duration::from_millis(REFRESH_INTERVAL_MS);
        loop {
            if self.kill_signal.try_recv().is_ok() {
                // got kill signal
                break;
            };
            if let Ok(metrics) = self.metrics.send(GetMetrics).await {
                self.dashboard.update_metrics(metrics);
            }
            self.dashboard.draw(&mut self.terminal);

            // Wait timeout or for event from terminal
            match timeout(WAIT, events.recv()).await {
                Ok(Some(Event::Key(key))) => {
                    self.process_key(key);
                },
                Ok(Some(Event::Resize(width, ..))) => {
                    if let Err(err) = self
                        .metrics
                        .send(MetricsConfig {
                            instructions_spark_sizes: width as usize,
                        })
                        .await
                    {
                        log::warn!("Failed to reconfigure Metrics actor for new terminal size: {}", err);
                    }
                },
                _ => {},
            };
        }
        events.close();
    }

    fn process_key(&mut self, KeyEvent { code, modifiers }: KeyEvent) {
        match (code, modifiers) {
            // TODO: send proper kill signal back to server
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.kill_signal.close();
                // std::process::exit(1)
            },
            _ => {},
        }
    }
}
