// ui/chat.rs
use crate::app::{App, CurrentScreen};
use crate::ui::utils::{wrap_single_line, wrap_text};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_chat(frame: &mut Frame, app: &mut App) {
    // Compose message scrolling management
    let input_lines = wrap_single_line(&app.message_input, frame.area().width as usize - 4); // Subtracting borders

    let available_height = frame.area().height as usize; // u16 to usize value
    let max_input_height = std::cmp::min(available_height.saturating_sub(4), 5); // Prevent overflow
    let input_height = std::cmp::min(input_lines.len(), max_input_height);

    // Scroll offset for input (manages scrolling when the input is longer than the view)
    let input_start_line = app.compose_scroll_offset;
    let visible_input_lines = input_lines
        .iter()
        .skip(input_start_line)
        .take(max_input_height)
        .cloned()
        .collect::<Vec<String>>();

    // Layout based on dynamic input box height
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                         // Title/Header
            Constraint::Min(1),                            // Messages List
            Constraint::Length((input_height + 2) as u16), // Message Input Field
        ])
        .split(frame.area());

    // Header block (Title and Help)
    const TITLE: &str = "TUI Messenger";
    const KEY_HINT: &str = "(h) help";
    let total_width = frame.area().width as usize;

    // Ensure that we don't subtract too much and cause a crash
    let space_padding = total_width.saturating_sub(TITLE.len() + KEY_HINT.len() + 2); // Avoid negative values

    let header = Paragraph::new(Line::from(vec![
        Span::styled(TITLE, Style::default().fg(Color::Green)),
        Span::raw(" ".repeat(space_padding)), // Safely repeat spaces
        Span::styled(KEY_HINT, Style::default().fg(Color::Red)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Messages area with left/right alignment for sent/received messages
    let messages_area = chunks[1];
    let max_width = messages_area.width.checked_sub(4).unwrap_or(0) as usize;
    let available_lines = (messages_area.height as usize).saturating_sub(2);

    // Wrap messages, and calculate total lines
    let wrapped_lines = wrap_text(&app.messages, max_width, app.username.as_deref());
    let total_lines = wrapped_lines.len();

    // Calculate starting line based on the scroll offset and total lines
    let start_line = total_lines
        .saturating_sub(available_lines)
        .saturating_sub(app.scroll_offset);

    // Render the visible lines
    let visible_lines = wrapped_lines
        .into_iter()
        .skip(start_line)
        .take(available_lines)
        .map(|line| {
            ListItem::new(line) // The line is already a Span with styling
        })
        .collect::<Vec<ListItem>>();

    let list = List::new(visible_lines).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, messages_area);

    // Message input block
    let typing = Paragraph::new(visible_input_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Compose Message"),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(typing, chunks[2]);

    // Set cursor position if composing a message
    if let CurrentScreen::ComposingMessage = app.current_screen {
        let cursor_x = chunks[2].x + app.message_input.len() as u16 + 1;
        let cursor_y = chunks[2].y + visible_input_lines.len() as u16;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}
