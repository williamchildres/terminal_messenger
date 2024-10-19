// ui/disconnected.rs
use crate::ui::utils::centered_rect;
use ratatui::{
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
