use std::{rc::Rc, str::FromStr};
use ratatui::{prelude::*, widgets::*};

use crate::file_operation;
use crate::context::Context;

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
        Paragraph::new("Q - Quit | C - Copy | P - Paste | R - Rename | Enter - Open")
            .bold()
            .alignment(Alignment::Left),
        main_layout[2],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(main_layout[1]);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(path_left()),
        inner_layout[0],
    );

    render_right_directory(frame, inner_layout, context);
}

fn render_right_directory(frame: &mut Frame, inner_layout: Rc<[Rect]>, context: &mut Context) {
    let items = file_operation::list_children(context).unwrap();

    let list = List::new(items)
        .block(
            Block::default()
                .title(file_operation::directory_path("."))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom);

    let mut state = ListState::default().with_selected(Some(context.state));

    frame.render_stateful_widget(list, inner_layout[1], &mut state);
}

fn path_left() -> String {
    file_operation::directory_path("./")
}

pub fn selected_item_right(context: &mut Context) -> String {
    let item = String::from_str(context.items.get(context.state).unwrap());
    item.ok().unwrap()
}