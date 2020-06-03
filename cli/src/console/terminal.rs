use crossterm::event::{read, Event};
use serde::Serialize;
use std::{
    io::{self, Stdout, Write},
    iter::Iterator,
    ops::{Deref, DerefMut},
    thread::JoinHandle,
};
use tokio::sync::mpsc::{channel, Receiver};
use tui::{
    backend::CrosstermBackend,
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Row, Table},
    Terminal as TUITerminal,
};

type CrosstermRawTerminal = TUITerminal<CrosstermBackend<Stdout>>;

const EVENTS_BUFFER_SIZE: usize = 100;

pub struct Terminal {
    inner: CrosstermRawTerminal,
    alternate: bool,
    events: Option<JoinHandle<()>>,
}

impl Default for Terminal {
    fn default() -> Self {
        let backend = CrosstermBackend::new(io::stdout());
        let inner = TUITerminal::new(backend).unwrap();
        Self {
            inner,
            alternate: false,
            events: None,
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        use crossterm::{execute, terminal::LeaveAlternateScreen};
        if self.alternate {
            if let Ok(_) = execute!(io::stdout(), LeaveAlternateScreen) {
                self.alternate = false;
            } else {
                log::warn!("Failed to leave alternate terminal screen");
            }
        }
        let _ = self.inner.show_cursor();
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

impl Deref for Terminal {
    type Target = CrosstermRawTerminal;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for Terminal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Terminal {
    /// Init main terminal screen, scroll existing content up to allow rendering
    pub fn basic() -> Self {
        let this: Terminal = Default::default();
        let size = this.inner.size().unwrap();
        println!("{}", "\n".repeat(size.height as usize));
        this
    }

    /// Init alternate terminal screen, which will be dropped when Terminal is dropped
    /// Current screen content will stay as is
    pub fn alternate() -> Self {
        use crossterm::{execute, terminal::EnterAlternateScreen};
        let mut this: Terminal = Default::default();
        crossterm::terminal::enable_raw_mode().unwrap();
        this.inner.hide_cursor().unwrap();
        if let Ok(_) = execute!(io::stdout(), EnterAlternateScreen) {
            this.alternate = true;
        } else {
            log::warn!("Failed to enter alternate terminal screen");
        }
        this
    }

    pub fn render_list<T: Serialize>(&mut self, name: &str, value: Vec<T>, fields: &[&str], sizes: &[u16]) {
        let value = serde_json::json!(value);
        if !value.is_array() {
            return println!("{:#}", value);
        }

        let values = value.as_array().unwrap();
        let mut headers = fields;
        let rows: Vec<Vec<String>> = if value[0].is_object() {
            values
                .iter()
                .map(|v| {
                    fields
                        .iter()
                        .map(|f| {
                            v.as_object()
                                .unwrap()
                                .get(*f)
                                .map(|v| v.to_string().trim_matches('"').to_string())
                                .unwrap_or("".to_string())
                        })
                        .collect()
                })
                .collect()
        } else {
            headers = &fields[0..1];
            values.iter().map(|v| vec![v.to_string()]).collect()
        };
        let constraints: Vec<_> = sizes.iter().map(|width| Constraint::Length(*width)).collect();
        let table = Table::new(headers.iter(), rows.iter().map(move |row| Row::Data(row.into_iter())))
            .block(Block::default().title(name))
            .widths(constraints.as_slice());

        self.inner
            .draw(|mut f| {
                let mut size = f.size();
                size.height -= 1;
                f.render_widget(
                    table
                        .header_style(Style::default().fg(Color::Yellow))
                        .style(Style::default().fg(Color::White))
                        .column_spacing(1),
                    size,
                );
            })
            .unwrap();
        println!("");
    }

    pub fn render_object<T: Serialize>(&mut self, name: &str, value: T) {
        let mut rows = vec![];
        let value = serde_json::json!(value);
        for (field, value) in value.as_object().unwrap().iter() {
            rows.push([field.to_string(), value.to_string()]);
        }
        let table = Table::new(
            ["Field", "Value"].iter(),
            rows.iter().map(move |row| Row::Data(row.into_iter())),
        )
        .block(Block::default().title(name))
        .widths(&[Constraint::Length(25), Constraint::Length(100)]);

        self.inner
            .draw(|mut f| {
                let size = f.size();
                f.render_widget(
                    table
                        .header_style(Style::default().fg(Color::Yellow))
                        .style(Style::default().fg(Color::White))
                        .column_spacing(1),
                    size,
                );
            })
            .unwrap();
        println!("");
    }

    pub fn events_receiver(&mut self) -> anyhow::Result<Receiver<Event>> {
        if self.events.is_some() {
            // TODO: or should we restart events thread to allow recovery after panics?
            anyhow::anyhow!("Events receiver was tried to call second time, can be called only once now");
        }

        // Setup input handling
        let (mut event_sender, event_receiver) = channel(EVENTS_BUFFER_SIZE);

        self.events = Some(std::thread::spawn(move || {
            loop {
                // poll for tick rate duration, if no events, sent tick event.
                match read() {
                    Ok(event) => {
                        if let Err(err) = event_sender.try_send(event) {
                            log::warn!("Received terminal event, but failed to send to ServerConsole: {}", err);
                        }
                    },
                    Err(err) => log::warn!("Failed to read events from Terminal: {}", err),
                }
            }
        }));
        Ok(event_receiver)
    }
}
