
use std::rc::Rc;
use ratatui::{prelude::*, widgets::*};

use crate::file_operation;
use crate::context::Context;

pub fn render_left_directory(frame: &mut Frame, inner_layout: Rc<[Rect]>, context: &mut Context) {
    let items = file_operation::list_children(context).unwrap();

    let list = List::new(items)
        .block(
            Block::default()
                .title(context.path.clone())
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom);

    let mut state = ListState::default().with_selected(Some(context.state));

    frame.render_stateful_widget(list, inner_layout[0], &mut state);
}

pub fn render_right_directory(frame: &mut Frame, inner_layout: Rc<[Rect]>, context: &mut Context) {
    let items = file_operation::list_children(context).unwrap();

    let list = List::default()
        .block(
            Block::default()
                .title(context.path.clone())
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

pub fn popup_window(frame: &mut Frame, context: &mut Context) {
    let block = Block::default().title("Create folder").borders(Borders::ALL);
    let para = Paragraph::new(context.get_input().unwrap().clone()).block(block);
    let area = centered_rect(frame.size());

    frame.render_widget(Clear, area); //this clears out the background
    frame.render_widget(para, area);
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(r: Rect) -> Rect {
    Layout::new(
        Direction::Vertical,
        [Constraint::Length(3), Constraint::Length(5)])
        .split(r)[0]
}