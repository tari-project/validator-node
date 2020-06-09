use super::Terminal;
use tari_validator_node::metrics::{Metrics, MetricsSnapshot};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Text},
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
    pub const COUNTER_COLUMN_WIDTH: u16 = 25;

    pub fn sparkline_width(terminal: &Terminal) -> u16 {
        terminal
            .size()
            .map(|s| s.width)
            .unwrap_or(80)
            .saturating_sub(Self::COUNTER_COLUMN_WIDTH)
    }

    pub fn update_metrics(&mut self, metrics: MetricsSnapshot) {
        self.metrics = metrics;
    }

    pub fn draw(&self, terminal: &mut Terminal) {
        terminal.draw(|mut f| {
            let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(17),
                    Constraint::Length(7),
                ]
                .as_ref(),
            )
            .split(f.size());

            let r1_columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(60),
                    Constraint::Length(Self::COUNTER_COLUMN_WIDTH),
                ]
                .as_ref(),
            )
            .split(rows[0]);

            let counters_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(9),
                    Constraint::Length(9),
                ]
                .as_ref(),
            )
            .split(r1_columns[1]);

            self.draw_instruction_sparklines(&mut f, r1_columns[0]);
            self.draw_counters_info(&mut f, counters_area[0]);
            self.draw_pool_status(&mut f, counters_area[1]);
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
            ("Scheduled", &self.metrics.instructions_scheduled_spark, Color::Yellow),
            ("Processing", &self.metrics.instructions_processing_spark, Color::Blue),
            ("Pending", &self.metrics.instructions_pending_spark, Color::Gray),
            ("Invalid", &self.metrics.instructions_invalid_spark, Color::Red),
            ("Commit", &self.metrics.instructions_commit_spark, Color::Green),
        ];

        for (chunk, (title, data, color)) in chunks.into_iter().zip(&data) {
            let sparkline = Sparkline::default()
                .block(Block::default().title(title))
                .data(data.as_slice())
                .style(Style::default().fg(*color));
            f.render_widget(sparkline, chunk);
        }
    }

    fn draw_counters_info<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3), Constraint::Length(3)].as_ref())
            .margin(0)
            .split(area);

        let total = [Text::raw(self.metrics.total_unique_instructions.to_string())];
        let total = Paragraph::new(total.iter())
            .block(Block::default().borders(Borders::ALL).title("Total instructions"))
            .style(Style::default().fg(Color::Green));
        f.render_widget(total, chunks[0]);

        let processing = [Text::raw(self.metrics.current_processing_instructions.to_string())];
        let proessing = Paragraph::new(processing.iter())
            .block(Block::default().borders(Borders::ALL).title("Processing"))
            .style(Style::default().fg(Color::Blue));
        f.render_widget(proessing, chunks[1]);

        let pending = [Text::raw(self.metrics.current_pending_instructions.to_string())];
        let pending = Paragraph::new(pending.iter())
            .block(Block::default().borders(Borders::ALL).title("Pending consensus"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(pending, chunks[2]);
    }

    fn draw_pool_status<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        if self.metrics.pool_status.is_none() {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
            .margin(1)
            .split(area);

        let block = Block::default().borders(Borders::ALL).title("DB Pool status");
        f.render_widget(block, area);

        let status = self.metrics.pool_status.as_ref().unwrap();

        let available = if status.available > 0 { status.available } else { 0 };
        let available_ratio = available as f64 / status.max_size as f64;
        let title = format!("Available {}/{}", available, status.max_size);
        let connections = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(title.as_str()))
            .style(Style::default().fg(Color::Green).bg(Color::Gray))
            .ratio(available_ratio);
        f.render_widget(connections, chunks[0]);

        let waiting = if status.available < 0 { -status.available } else { 0 };
        let mut waiting_ratio = waiting as f64 / status.max_size as f64 * 2f64;
        if waiting_ratio > 1f64 {
            waiting_ratio = 1f64;
        }
        let title = format!("Waiting {}", waiting);
        let connections = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(title.as_str()))
            .style(Style::default().fg(Color::Red).bg(Color::Gray))
            .ratio(waiting_ratio);
        f.render_widget(connections, chunks[1]);
    }
}
