// ui/set_add.server.rs
use crate::app::App;
use crate::ui::utils::centered_rect;
use ratatui::{
    layout::Position,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
}; // Import the utility functions

pub fn render_add_server(frame: &mut Frame, app: &mut App) {
    frame.render_widget(Clear, frame.area());
    let block = Block::default()
        .title("Add New Server (name:url)")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    let paragraph = Paragraph::new(app.message_input.as_str())
        .block(block)
        .wrap(Wrap { trim: true });
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(paragraph, area);
    let cursor_x = area.x + app.message_input.len() as u16 + 1;
    let cursor_y = area.y + 1;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}
