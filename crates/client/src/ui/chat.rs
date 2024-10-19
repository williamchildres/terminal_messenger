// ui/chat.rs
use crate::app::App;
use ratatui::{
    layout::Constraint,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_chat<B>(frame: &mut Frame<B>, app: &mut App)
where
    B: ratatui::backend::Backend,
{
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Chat messages
            Constraint::Length(3), // Message input
        ])
        .split(frame.area());

    // Messages area
    let messages = app
        .messages
        .iter()
        .map(|msg| {
            ListItem::new(msg.to_string()) // Example conversion, adjust as needed
        })
        .collect::<Vec<_>>();

    let list = List::new(messages).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, chunks[1]);

    // Compose message input
    let typing = ratatui::widgets::Paragraph::new(app.message_input.clone()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Compose Message"),
    );
    frame.render_widget(typing, chunks[2]);
}
