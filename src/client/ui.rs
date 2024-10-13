use ratatui::{
    crossterm::terminal,
    layout::{Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &mut App) {
    // Create the layout sections.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title/Header
            Constraint::Min(1),    // Messages List
            Constraint::Length(3), // Message Input Field
        ])
        .split(frame.area());

    // Create the title and key hints in the header (chunks[0])
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    // Get terminal size
    let terminal_size = frame.area();
    let total_width = terminal_size.width as usize;

    // Calculate lengths of title and key hint texts
    let title_len = "TUI Messenger".len();
    let key_kint_len = "(q) quit / (h) help".len();

    // Calculate length of space that should be inserted between title and key hints, + 2 to
    // accomidate title block
    let spaces_len = total_width.saturating_sub(title_len + key_kint_len + 2);

    // Left side of the header: "TUI Messenger"
    let title_text = Span::styled("TUI Messenger", Style::default().fg(Color::Green));

    // Right side of the header: "(q) to quit / (e) to compose message"
    let key_hint_text = Span::styled("(q) quit / (h) help", Style::default().fg(Color::Red));

    // Combine the two into one line
    let header_line = Line::from(vec![
        title_text,                                             // Title on the left
        Span::styled(" ".repeat(spaces_len), Style::default()), // Add spacing between title and key hints
        key_hint_text,                                          // Key hints on the right
    ]);

    let title_paragraph = Paragraph::new(header_line).block(title_block);
    frame.render_widget(title_paragraph, chunks[0]);

    // Display the list of messages in the main area (chunks[1])
    let mut list_items = Vec::<ListItem>::new();

    let max_width = chunks[1].width as usize - 4; // The width of messages List minus padding and borders.
    let max_height = chunks[1].height as usize - 2;

    app.messages_offset = app.messages.len().saturating_sub(max_height);

    for message in &app.messages[app.messages_offset..] {
        // Wrap each message so it fits within the widget's width.
        let wrapped_message_lines = wrap_text(message, max_width);

        // Create a ListItem for each line produced by wrapping and add them to list_items vector.
        for line in wrapped_message_lines {
            list_items.push(ListItem::new(Span::styled(
                line,
                Style::default().fg(Color::Green),
            )));
        }
    }

    let list = List::new(list_items).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, chunks[1]);

    // Show message input at the bottom if the user is composing a message
    let typing_block = Block::default()
        .title("Compose Message")
        .borders(Borders::ALL);
    let typing_paragraph = Paragraph::new(app.message_input.as_str())
        .block(typing_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(typing_paragraph, chunks[2]); // Use chunks[2] for the input field at the bottom

    if let CurrentScreen::ComposingMessage = app.current_screen {
        let x_position = chunks[2].x + app.message_input.len() as u16 + 1;
        let y_position = chunks[2].y + 1;
        let position = Position::new(x_position, y_position);
        frame.set_cursor_position(position);
    }

    // Show help menu if 'e' is pressed
    if let CurrentScreen::HelpMenu = app.current_screen {
        frame.render_widget(Clear, frame.area()); // Clears the entire screen and anything already drawn

        let help_menu_block = Block::default()
            .title("Help Menu")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

        let help_menu_text = Text::styled(
            "Here is some helpful information", // replace with actual help text
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
