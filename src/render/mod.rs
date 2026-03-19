mod panels;
mod popups;

pub use panels::render_panel;
pub use popups::*;

use ratatui::layout::Alignment;
use ratatui::prelude::*;
use ratatui::style::Style;
use ratatui::widgets::*;

/// Error type for rendering operations
pub(crate) type RenderResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Creates a styled button with minimum width and optional selected state
pub(crate) fn create_sized_button(text: &str, selected: bool) -> Paragraph<'_> {
    let style = if selected {
        Style::default().bg(Color::Blue).fg(Color::Black)
    } else {
        Style::default().fg(Color::Blue)
    };

    let border_style = if selected {
        Style::default().fg(Color::Black)
    } else {
        Style::default().fg(Color::Blue)
    };

    Paragraph::new(format!(" {} ", text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_alignment(Alignment::Center)
                .padding(Padding::new(1, 1, 0, 0)),
        )
        .alignment(Alignment::Center)
        .style(style)
}

/// Helper function to create a centered rectangle
pub(crate) fn centered_rect_dialog(r: Rect, percent_width: u16, percent_height: u16) -> Rect {
    let height_percent = percent_height.clamp(10, 90);
    let width = (r.width * percent_width / 100).min(60);
    let calculated_height = r.height * height_percent / 100;
    let height = calculated_height.clamp(10, 16);
    let x = (r.width - width) / 2;
    let y = (r.height - height) / 2;
    Rect::new(r.x + x, r.y + y, width, height)
}
