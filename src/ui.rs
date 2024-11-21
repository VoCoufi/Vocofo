use ratatui::{prelude::*, widgets::*};

use crate::context::Context;
use crate::render;

pub fn ui(frame: &mut Frame, context: &mut Context) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    )
    .split(frame.size());
    frame.render_widget(
        Block::new()
            .borders(Borders::NONE)
            .title("Vocofo")
            .title_alignment(Alignment::Center),
        main_layout[0],
    );
    frame.render_widget(
        Paragraph::new("Q - Quit | C - Copy | P - Paste | R - Rename | Enter - Open | O - Options")
            .bold()
            .alignment(Alignment::Left),
        main_layout[2],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(main_layout[1]);

    let binding = String::default();
    let filename_right = context.get_selected_item().unwrap_or(&binding);

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(filename_right.clone()),
        inner_layout[1],
    );

    render::render_left_directory(frame, inner_layout.clone(), context);
    render::render_right_directory(frame, inner_layout.clone(), context);

    if context.get_popup().unwrap() {
        render::popup_window(frame, context);
    }
}