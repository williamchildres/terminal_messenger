// Define `centered_rect`
use crate::app::MessageType;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

// Define `wrap_text` (example)
pub fn wrap_text(
    messages: &[MessageType],
    max_width: usize,
    current_username: Option<&str>,
) -> Vec<Span<'static>> {
    let mut lines = Vec::new();

    for message in messages {
        match message {
            MessageType::ChatMessage { sender, content } => {
                let wrapped_lines = wrap_single_line(content, max_width);
                if Some(sender.as_str()) == current_username {
                    // Right-align the current user's messages with Cyan color
                    for line in wrapped_lines {
                        let padding = " ".repeat(max_width.saturating_sub(line.len()));
                        lines.push(Span::styled(
                            format!("{}{}", padding, line),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                } else {
                    // Left-align other users' messages with Green color
                    for line in wrapped_lines {
                        lines.push(Span::styled(
                            format!("{}: {}", sender, line),
                            Style::default().fg(Color::Green),
                        ));
                    }
                }
            }
            MessageType::SystemMessage(system_message) => {
                let wrapped_lines = wrap_single_line(system_message, max_width);
                for line in wrapped_lines {
                    lines.push(Span::styled(line, Style::default().fg(Color::Yellow)));
                }
            }
            _ => {}
        }
    }

    lines
}

pub fn wrap_single_line(line: &str, max_width: usize) -> Vec<String> {
    let max_width = std::cmp::max(max_width, 10); // Avoid subtracting below a reasonable minimum width
    let mut wrapped_lines = Vec::new();

    for line in line.split('\n') {
        let words = line.split_whitespace();
        let mut new_line = String::new();

        for word in words {
            if new_line.len() + word.len() > max_width {
                wrapped_lines.push(new_line.trim().to_string());
                new_line.clear();
            }

            if !new_line.is_empty() {
                new_line.push(' ');
            }

            new_line.push_str(word);
        }

        wrapped_lines.push(new_line.trim().to_string());
    }

    wrapped_lines
}
