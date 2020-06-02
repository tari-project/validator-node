use super::Terminal;
use tari_validator_node::metrics::{Metrics, MetricsSnapshot};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Sparkline},
    Frame,
};

pub struct Dashboard {
    metrics: MetricsSnapshot,
}

impl Default for Dashboard {
    fn default() -> Self {
        Self {
            metrics: MetricsSnapshot::from(&Metrics::default()),
        }
    }
}

impl Dashboard {
    pub fn update_metrics(&mut self, metrics: MetricsSnapshot) {
        self.metrics = metrics;
    }

    pub fn draw(&self, terminal: &mut Terminal) {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(17),
                    Constraint::Length(7),
                ]
                .as_ref(),
            )
            .split(f.size());
            self.draw_instruction_sparklines(&mut f, chunks[0]);
        })
        // TODO: this should process errors - but ok for demo
        .unwrap();
    }

    fn draw_instruction_sparklines<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .margin(1)
            .split(area);

        let block = Block::default().borders(Borders::ALL).title("Instructions per second");
        f.render_widget(block, area);

        let data = [
            ("Scheduled", &self.metrics.instructions_scheduled_spark),
            ("Processing", &self.metrics.instructions_processing_spark),
            ("Pending", &self.metrics.instructions_pending_spark),
            ("Invalid", &self.metrics.instructions_invalid_spark),
            ("Commit", &self.metrics.instructions_commit_spark),
        ];

        for (chunk, (title, data)) in chunks.into_iter().zip(&data) {
            let sparkline = Sparkline::default()
                .block(Block::default().title(title))
                .data(data.as_slice())
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, chunk);
        }
    }
}
