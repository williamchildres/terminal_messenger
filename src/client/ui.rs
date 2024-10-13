use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, CurrentScreen};

pub fn ui(frame: &mut Frame, app: &App) {
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

    // Left side of the header: "TUI Messenger"
    let title_text = Span::styled("TUI Messenger", Style::default().fg(Color::Green));

    // Right side of the header: "(q) to quit / (e) to compose message"
    let key_hint_text = Span::styled(
        "(q) to quit / (e) to compose message",
        Style::default().fg(Color::Red),
    );

    // Combine the two into one line
    let header_line = Line::from(vec![
        title_text,                                     // Title on the left
        Span::styled(" ".repeat(30), Style::default()), // Add spacing between title and key hints
        key_hint_text,                                  // Key hints on the right
    ]);

    let title_paragraph = Paragraph::new(header_line).block(title_block);
    frame.render_widget(title_paragraph, chunks[0]);

    // Display the list of messages in the main area (chunks[1])
    let mut list_items = Vec::<ListItem>::new();
    for message in &app.messages {
        list_items.push(ListItem::new(Span::styled(
            message,
            Style::default().fg(Color::Green), // Customize color and style here
        )));
    }

    let list = List::new(list_items).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, chunks[1]);

    // Show message input at the bottom if the user is composing a message
    if let CurrentScreen::ComposingMessage = app.current_screen {
        let typing_block = Block::default()
            .title("Compose Message")
            .borders(Borders::ALL);
        let typing_paragraph = Paragraph::new(app.message_input.as_str())
            .block(typing_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(typing_paragraph, chunks[2]); // Use chunks[2] for the input field at the bottom
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
