// ui/server_selection.rs
use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_server_selection(frame: &mut Frame, app: &mut App) {
    frame.render_widget(ratatui::widgets::Clear, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(1),    // Server list
            Constraint::Length(3), // Input for adding new server
        ])
        .split(frame.area());

    // Title block
    let title = Paragraph::new("Select a Server").block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Render the server list
    let server_list: Vec<ListItem> = app
        .servers
        .iter()
        .map(|(name, _url)| ListItem::new(name.clone()))
        .collect();

    let server_list_widget = List::new(server_list).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Available Servers"),
    );
    frame.render_widget(server_list_widget, chunks[1]);

    // Render the input for adding new servers
    let input_widget = Paragraph::new(app.message_input.clone()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Add New Server (name:url)"),
    );
    frame.render_widget(input_widget, chunks[2]);

    // Set the cursor at the end of the message input field
    let cursor_x = chunks[2].x + app.message_input.len() as u16 + 1;
    let cursor_y = chunks[2].y + 1;
    frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}
