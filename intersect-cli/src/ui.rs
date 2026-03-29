use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Status};

const HEADER:       usize = 0;
// index 1 = top gap
const LOG_TITLE:    usize = 2;
const LOG_CONTENT:  usize = 3;
const OUTPUT_LABEL: usize = 4;
const OUTPUT:       usize = 5;
// index 6 = bottom gap
const INPUT:        usize = 7;

pub fn render(frame: &mut Frame, app: &mut App) {
    let log_content_height = if app.log_expanded { 8 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                   // HEADER
            Constraint::Length(1),                   // top gap
            Constraint::Length(1),                   // LOG_TITLE
            Constraint::Length(log_content_height),  // LOG_CONTENT
            Constraint::Length(1),                   // OUTPUT_LABEL
            Constraint::Min(0),                      // OUTPUT
            Constraint::Length(1),                   // bottom gap
            Constraint::Length(1),                   // INPUT
        ])
        .split(frame.area());

    app.log_area = chunks[LOG_CONTENT];

    // header
    let (status_text, status_color) = match &app.status {
        Status::Connecting => ("● connecting…", Color::DarkGray),
        Status::Ready => ("● ready", Color::Green),
        Status::Failed(e) => (e.as_str(), Color::Red),
        Status::Closing => ("● closing…", Color::Yellow),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("intersect", Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(status_text, Style::default().fg(status_color)),
        ])),
        chunks[HEADER],
    );

    // log title — always visible, toggle with ` or ~
    let log_label = if app.log_expanded { "[log ▾]" } else { "[log ▸]" };
    frame.render_widget(Paragraph::new(log_label), chunks[LOG_TITLE]);

    // log content — only rendered when expanded
    if app.log_expanded {
        let log_lines: Vec<Line> = app.log.iter().map(|l| Line::from(l.as_str())).collect();
        let inner_height = chunks[LOG_CONTENT].height;
        let para = Paragraph::new(log_lines).wrap(Wrap { trim: false });
        let max_scroll = (para.line_count(chunks[LOG_CONTENT].width) as u16).saturating_sub(inner_height);
        let scroll = max_scroll.saturating_sub(app.log_scroll.min(max_scroll));
        frame.render_widget(para.scroll((scroll, 0)), chunks[LOG_CONTENT]);
    }

    // output label + bordered panel
    frame.render_widget(Paragraph::new("[output]"), chunks[OUTPUT_LABEL]);

    let output_lines: Vec<Line> = app.output.iter().map(|l| Line::from(l.as_str())).collect();
    let output_inner_height = chunks[OUTPUT].height.saturating_sub(2); // top + bottom border
    let output_scroll = (output_lines.len() as u16).saturating_sub(output_inner_height);
    frame.render_widget(
        Paragraph::new(output_lines)
            .block(Block::bordered())
            .scroll((output_scroll, 0))
            .wrap(Wrap { trim: false }),
        chunks[OUTPUT],
    );

    // input
    frame.render_widget(Paragraph::new(format!("> {}", app.input)), chunks[INPUT]);
    frame.set_cursor_position((chunks[INPUT].x + 2 + app.input.len() as u16, chunks[INPUT].y));
}
