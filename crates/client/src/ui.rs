use crate::app::{App, CurrentScreen, MessageType};
use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // Compose message scrolling management
    let input_lines = wrap_text(&app.message_input, frame.area().width as usize - 4); // Subtracting borders
    let max_input_height = 5; // Maximum height for input box
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

    // Handle disconnected state
    if let CurrentScreen::Disconnected = app.current_screen {
        let block = Block::default()
            .title("Disconnected")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));
        let paragraph = Paragraph::new(
            "Connection lost. Press 'r' to attempt to reconnect or press 'q' to quit.",
        )
        .block(block)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::Yellow));
        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(Clear, frame.area());
        frame.render_widget(paragraph, area);
        return;
    }

    // Handle login screen
    if let CurrentScreen::LoggingIn = app.current_screen {
        frame.render_widget(Clear, frame.area());
        let block = Block::default()
            .title("Login")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));
        let prompt = if app.username.is_none() {
            "Enter your username:"
        } else {
            "Enter your password:"
        };
        let paragraph = Paragraph::new(format!("{} {}", prompt, app.message_input.as_str()))
            .block(block)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow));
        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(paragraph, area);
        let cursor_x = area.x + prompt.len() as u16 + app.message_input.len() as u16 + 1;
        let cursor_y = area.y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        return;
    }

    // Header block (Title and Help)
    const TITLE: &str = "TUI Messenger";
    const KEY_HINT: &str = "(h) help";
    let header = Paragraph::new(Line::from(vec![
        Span::styled(TITLE, Style::default().fg(Color::Green)),
        Span::raw(" ".repeat(frame.area().width as usize - TITLE.len() - KEY_HINT.len())),
        Span::styled(KEY_HINT, Style::default().fg(Color::Red)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Messages area with left/right alignment for sent/received messages
    let messages_area = chunks[1];
    let max_width = messages_area.width.checked_sub(4).unwrap_or(0) as usize;
    let available_lines = messages_area.height as usize - 2;

    let wrapped_messages = app
        .messages
        .iter()
        .flat_map(|msg| {
            if let MessageType::ChatMessage { sender, content } = msg {
                wrap_text(&format!("{}: {}", sender, content), max_width)
            } else {
                vec![]
            }
        })
        .collect::<Vec<String>>();

    let start_line = wrapped_messages
        .len()
        .saturating_sub(available_lines + app.scroll_offset);

    let visible_lines = app
        .messages
        .iter()
        .skip(start_line)
        .take(available_lines)
        .map(|msg| {
            match msg {
                MessageType::ChatMessage { sender, content } => {
                    let alignment = if Some(sender) == app.username.as_ref() {
                        // Align to the right if the message is from the current user
                        let padding = " ".repeat(max_width.checked_sub(content.len()).unwrap_or(0));
                        ListItem::new(Span::styled(
                            format!("{}{}", padding, content),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else {
                        // Align to the left for other users
                        ListItem::new(Span::styled(
                            format!("{}: {}", sender, content),
                            Style::default().fg(Color::Green),
                        ))
                    };
                    Some(alignment)
                }
                MessageType::SystemMessage(system_message) => Some(ListItem::new(Span::styled(
                    system_message.to_string(),
                    Style::default().fg(Color::Yellow),
                ))),
                _ => None,
            }
        })
        .filter_map(|x| x)
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

    // Help menu popup
    if let CurrentScreen::HelpMenu = app.current_screen {
        frame.render_widget(Clear, frame.area());
        let help_menu_block = Block::default()
            .title("Help Menu")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));
        let help_menu_text = Text::styled(
            "(q) to quit\n(n) to set username",
            Style::default().fg(Color::Red),
        );
        let help_menu_paragraph = Paragraph::new(help_menu_text)
            .block(help_menu_block)
            .wrap(Wrap { trim: false });
        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(help_menu_paragraph, area);
    }

    // Exiting confirmation popup
    if let CurrentScreen::Exiting = app.current_screen {
        frame.render_widget(Clear, frame.area());
        let popup_block = Block::default()
            .title("y/n")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));
        let exit_text = Text::styled(
            "Are you sure you want to quit?",
            Style::default().fg(Color::Red),
        );
        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false });
        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(exit_paragraph, area);
    }

    // Set Username Popup
    if let CurrentScreen::SetUser = app.current_screen {
        frame.render_widget(Clear, frame.area());
        let block = Block::default()
            .title("Set Username")
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

// Helper to wrap text
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let words = line.split_whitespace();
        let mut new_line = String::new();
        for word in words {
            if new_line.len() + word.len() > max_width {
                lines.push(new_line);
                new_line = String::from(word);
            } else {
                if !new_line.is_empty() {
                    new_line.push(' ');
                }
                new_line.push_str(word);
            }
        }
        lines.push(new_line);
    }
    lines
}
