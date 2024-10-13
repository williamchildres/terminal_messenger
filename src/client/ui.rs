use ratatui::{
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title/Header
            Constraint::Min(1),    // Messages List
            Constraint::Length(3), // Message Input Field
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
        .wrap(Wrap { trim: true });

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(Clear, frame.area()); // Clear the screen
        frame.render_widget(paragraph, area); // Show disconnection message
        return; // Skip the rest of the UI rendering
    }

    const TITLE: &str = "TUI Messenger";
    const KEY_HINT: &str = "(q) quit";

    let title_length = TITLE.len();
    let key_hint_len = KEY_HINT.len();
    let terminal_size = frame.area();
    let total_width = terminal_size.width as usize;
    let spaces_len = total_width.saturating_sub(title_length + key_hint_len + 2);

    // Header block with title and key hints
    let header = Paragraph::new(Line::from(vec![
        Span::styled(TITLE, Style::default().fg(Color::Green)),
        Span::raw(" ".repeat(spaces_len)),
        Span::styled(KEY_HINT, Style::default().fg(Color::Red)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Messages list block (handles multi-line wrapping)
    let messages_area = chunks[1];
    let max_width = messages_area.width as usize - 4;
    let available_lines = messages_area.height as usize - 2;

    // Flatten all messages into individual wrapped lines
    let wrapped_messages = app
        .messages
        .iter()
        .flat_map(|msg| wrap_text(msg, max_width))
        .collect::<Vec<String>>();

    // Calculate the starting point based on available display lines and scrolling offset
    let start_line = wrapped_messages
        .len()
        .saturating_sub(available_lines + app.scroll_offset);

    // Create list items from wrapped lines
    let visible_lines = wrapped_messages
        .iter()
        .skip(start_line)
        .take(available_lines)
        .map(|line| ListItem::new(Span::styled(line, Style::default().fg(Color::Green))))
        .collect::<Vec<ListItem>>();

    let list = List::new(visible_lines).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, messages_area);

    // Message input block
    let typing = Paragraph::new(app.message_input.as_str())
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
        let cursor_y = chunks[2].y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }

    // Popup for setting the username
    if let CurrentScreen::SetUser = app.current_screen {
        frame.render_widget(Clear, frame.area()); // Clears the screen

        let block = Block::default()
            .title("Set Username")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray));

        let paragraph = Paragraph::new(app.message_input.as_str())
            .block(block)
            .wrap(Wrap { trim: true });

        // Center the popup
        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(paragraph, area);

        // Set cursor position in the popup for entering the username
        let cursor_x = area.x + app.message_input.len() as u16 + 1;
        let cursor_y = area.y + 1;
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }

    // Show help menu if 'e' is pressed
    if let CurrentScreen::HelpMenu = app.current_screen {
        frame.render_widget(Clear, frame.area()); // Clears the entire screen and anything already drawn

        let help_menu_block = Block::default()
            .title("Help Menu")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

        let help_menu_text = Text::styled(
            "(q) to quit\n(n) to set username", // replace with actual help text
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
        frame.render_widget(Clear, frame.area()); // Clears the entire screen and anything already drawn
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
}

/// helper function to create a centered rect using up a certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
}

/// Wrap text into lines with maximum width.
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

/// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text() {
        let text = "Hello, world! This is a long text that needs to be wrapped.";
        let max_width = 10;
        let expected_lines = vec![
            "Hello,",
            "world! This",
            "is a long",
            "text that",
            "needs to be",
            "wrapped.",
        ];

        let result = wrap_text(text, max_width);

        assert_eq!(result, expected_lines);
    }
    #[test]
    fn test_wrap_text_empty() {
        let text = "";
        let max_width = 10;
        let expected_lines = vec![""];

        let result = wrap_text(text, max_width);

        assert_eq!(result, expected_lines);
    }
}
