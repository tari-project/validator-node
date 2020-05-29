use std::{
    fmt::Display,
    io::{self, Stdout},
    iter::Iterator,
};
use tokio::sync::Mutex;
use tui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    widgets::{Row, Table},
    Terminal,
};

type CrosstermRawTerminal = Terminal<CrosstermBackend<Stdout>>;

lazy_static::lazy_static! {
    static ref TERMINAL: Mutex<CrosstermRawTerminal> = {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend).unwrap();
        let size = terminal.size().unwrap();
        println!("{}", "\n".repeat(size.height as usize));
        Mutex::new(terminal)
    };
}

pub async fn render_table<'a, H, D, R>(table: Table<'a, H, R>)
where
    H: Iterator,
    H::Item: Display,
    D: Iterator,
    D::Item: Display,
    R: Iterator<Item = Row<D>>,
{
    TERMINAL
        .lock()
        .await
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

use tui::{layout::Constraint, widgets::Block};

pub async fn render_value_as_table(name: &str, value: serde_json::Value, fields: &[&str], sizes: &[u16]) {
    if value.is_object() {
        return render_object_as_table(name, value).await;
    } else if !value.is_array() {
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

    TERMINAL
        .lock()
        .await
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

pub async fn render_object_as_table(name: &str, value: serde_json::Value) {
    let mut rows = vec![];
    for (field, value) in value.as_object().unwrap().iter() {
        rows.push([field.to_string(), value.to_string()]);
    }
    let table = Table::new(
        ["Field", "Value"].iter(),
        rows.iter().map(move |row| Row::Data(row.into_iter())),
    )
    .block(Block::default().title(name))
    .widths(&[Constraint::Length(25), Constraint::Length(55)]);

    TERMINAL
        .lock()
        .await
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
