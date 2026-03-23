use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Status};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // header
    let (status_text, status_color) = match &app.status {
        Status::Connecting => ("● connecting…", Color::DarkGray),
        Status::Ready => ("● ready", Color::Green),
        Status::Failed(e) => (e.as_str(), Color::Red),
        Status::Closing => ("● closing…", Color::Yellow),
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled("intersect", Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(status_text, Style::default().fg(status_color)),
    ]));
    frame.render_widget(header, chunks[0]);

    // log: scroll so the latest entry is always visible
    let log_lines: Vec<Line> = app.log.iter().map(|l| Line::from(l.as_str())).collect();
    let log_height = chunks[1].height;
    let scroll = (log_lines.len() as u16).saturating_sub(log_height);
    frame.render_widget(
        Paragraph::new(log_lines)
            .scroll((scroll, 0))
            .wrap(ratatui::widgets::Wrap { trim: false }),
        chunks[1],
    );

    // input
    frame.render_widget(Paragraph::new(format!("> {}", app.input)), chunks[2]);
    frame.set_cursor_position((chunks[2].x + 2 + app.input.len() as u16, chunks[2].y));
}
