// ui/disconnected.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render_disconnected(frame: &mut Frame) {
    let block = Block::default()
        .title("Disconnected")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let paragraph =
        Paragraph::new("Connection lost. Press 'r' to attempt to reconnect or press 'q' to quit.")
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow));
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, frame.area());
    frame.render_widget(paragraph, area);
    return;
}

// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
