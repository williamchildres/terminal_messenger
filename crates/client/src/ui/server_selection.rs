// ui/server_selection.rs
use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
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
        ])
        .split(frame.area());

    // Title block
    let title = Paragraph::new("Select a Server").block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Render the server list
    let server_list: Vec<ListItem> = app
        .servers
        .iter()
        .map(|(name, _url)| {
            let style = if Some(name) == app.selected_server.as_ref() {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(name.clone()).style(style)
        })
        .collect();

    let server_list_widget = List::new(server_list).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Available Servers, add with (n)"),
    );
    frame.render_widget(server_list_widget, chunks[1]);
}
