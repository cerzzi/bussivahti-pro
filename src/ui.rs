use crate::models::StopData;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};
use std::collections::HashMap;

pub fn render(f: &mut Frame, data: &HashMap<String, StopData>, order: &[String]) {
    // Luodaan layout dynaamisesti pysäkkien määrän mukaan
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            order.iter().map(|_| Constraint::Length(8)).collect::<Vec<_>>()
        )
        .split(f.size());

    for (i, stop_id) in order.iter().enumerate() {
        if i >= chunks.len() { break; } 
        
        if let Some(stop) = data.get(stop_id) {
            render_stop_table(f, chunks[i], stop);
        } else {
            let p = Paragraph::new(format!("Haetaan dataa pysäkille {}...", stop_id))
                .block(Block::default().borders(Borders::ALL).title(stop_id.as_str()));
            f.render_widget(p, chunks[i]);
        }
    }
}

fn render_stop_table(f: &mut Frame, area: Rect, stop: &StopData) {
    let header_cells = ["Linja", "Suunta", "Min", "Klo", "Lähtöpylväs"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows = stop.departures.iter().map(|item| {
        // Värikoodaus minuuttien mukaan
        let color = if item.minutes_left <= 2 { Color::Red } 
                   else if item.minutes_left <= 5 { Color::Yellow } 
                   else { Color::Green };

        // ASCII-palkki
        let bar = create_ascii_bar(item.seconds_left, 900, 15, color);
        let rt_mark = if item.is_realtime { "" } else { "~" };

        let cells = vec![
            Cell::from(Span::styled(item.line.clone(), Style::default().add_modifier(Modifier::BOLD))),
            Cell::from(item.headsign.clone()),
            Cell::from(Span::styled(format!("{} min", item.minutes_left), Style::default().fg(color))),
            Cell::from(format!("{}{}", rt_mark, item.time_str)),
            Cell::from(bar),
        ];
        Row::new(cells).height(1)
    });

    let title = format!(" {} ({}) - Päivitetty {} ", 
        stop.stop_name, 
        stop.stop_id.split(':').nth(1).unwrap_or(""), 
        stop.last_updated.format("%H:%M:%S")
    );
    
    let table = Table::new(
        rows,
        [
            Constraint::Length(6), Constraint::Fill(1), Constraint::Length(8), 
            Constraint::Length(8), Constraint::Length(17),
        ]
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(table, area);
}

fn create_ascii_bar(val: i64, max: i64, width: usize, color: Color) -> Line<'static> {
    let ratio = 1.0 - (val as f64 / max as f64).clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty = width - filled;
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
    ])
}