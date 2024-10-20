// ui/exiting.rs
use crate::ui::utils::centered_rect;
use ratatui::{
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render_exiting(frame: &mut Frame) {
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
