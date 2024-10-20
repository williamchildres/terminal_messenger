// ui/login.rs
use crate::app::{App, LoginField, MessageType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_login(frame: &mut Frame, app: &mut App) {
    frame.render_widget(ratatui::widgets::Clear, frame.area());

    // Layout for the login screen
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Username input
            Constraint::Length(3), // Password input
            Constraint::Length(2), // System message
            Constraint::Min(0),    // Filler (remaining space)
        ])
        .split(frame.area());

    // Username input block
    let username_block = Block::default()
        .title("Username")
        .borders(Borders::ALL)
        .style(if let LoginField::Username = app.current_login_field {
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow) // Highlight active input
        } else {
            ratatui::style::Style::default()
        });

    let username_input = Paragraph::new(app.username.clone().unwrap_or_default())
        .block(username_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(username_input, chunks[1]);

    // Password input block
    let password_block = Block::default()
        .title("Password")
        .borders(Borders::ALL)
        .style(if let LoginField::Password = app.current_login_field {
            ratatui::style::Style::default().fg(ratatui::style::Color::Yellow) // Highlight active input
        } else {
            ratatui::style::Style::default()
        });

    let password_input = Paragraph::new(if app.password.is_some() {
        "*".repeat(app.password.as_ref().unwrap().len()) // Mask the password input
    } else {
        String::new()
    })
    .block(password_block)
    .wrap(Wrap { trim: true });

    frame.render_widget(password_input, chunks[2]);

    // Display the most recent system message (e.g., authentication failure)
    let system_message = if let Some(last_message) = app.messages.last() {
        match last_message {
            MessageType::SystemMessage(msg) => msg.clone(),
            _ => "".to_string(),
        }
    } else {
        "".to_string()
    };

    let message_block = Block::default()
        .borders(Borders::ALL)
        .title("System Message");
    let message_paragraph = Paragraph::new(system_message)
        .block(message_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(message_paragraph, chunks[3]);

    // Set cursor position based on the active field
    let cursor_x = match app.current_login_field {
        LoginField::Username => chunks[1].x + app.message_input.len() as u16 + 1,
        LoginField::Password => chunks[2].x + app.message_input.len() as u16 + 1,
    };
    let cursor_y = match app.current_login_field {
        LoginField::Username => chunks[1].y + 1,
        LoginField::Password => chunks[2].y + 1,
    };

    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}
