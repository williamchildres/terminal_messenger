// ui/login.rs
use crate::app::{App, CurrentScreen};
use ratatui::{
    layout::Position,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_login<B>(frame: &mut Frame<B>, app: &mut App)
where
    B: ratatui::backend::Backend,
{
    frame.render_widget(ratatui::widgets::Clear, frame.area());

    let block = Block::default()
        .title("Login")
        .borders(Borders::ALL)
        .style(ratatui::style::Style::default().bg(ratatui::style::Color::DarkGray));

    let prompt = if app.username.is_none() {
        "Enter your username:"
    } else {
        "Enter your password:"
    };

    let paragraph = Paragraph::new(format!("{} {}", prompt, app.message_input.as_str()))
        .block(block)
        .wrap(Wrap { trim: true })
        .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow));

    let area = crate::ui::centered_rect(60, 25, frame.area());
    frame.render_widget(paragraph, area);

    let cursor_x = area.x + prompt.len() as u16 + app.message_input.len() as u16 + 1;
    let cursor_y = area.y + 1;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}
