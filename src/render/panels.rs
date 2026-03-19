use ratatui::{
    prelude::*,
    style::{Style, Stylize},
    widgets::*,
};

use crate::file_operation;
use crate::context::PanelState;

use super::RenderResult;

/// Renders a directory panel, refreshing the item list only when dirty
pub fn render_panel(frame: &mut Frame, area: Rect, panel: &mut PanelState, is_active: bool, is_searching: bool) -> RenderResult<()> {
    if panel.items_dirty {
        file_operation::list_children(panel)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    }

    let panel_width = area.width.saturating_sub(4) as usize;
    let items: Vec<ListItem> = panel.filtered_items.iter().map(|name| {
        if name == "../" {
            return ListItem::new(name.clone()).style(Style::new().blue());
        }

        let full_path = panel.backend.join_path(&panel.path, name.trim_end_matches('/'));
        let details = match panel.backend.metadata(&full_path) {
            Ok(info) => file_operation::format_item_details_from_info(&info),
            Err(_) => String::new(),
        };

        let is_selected = panel.selected.contains(name);
        let name_style = if is_selected {
            Style::new().yellow().bold()
        } else if name.ends_with('/') {
            Style::new().blue()
        } else {
            Style::new().green()
        };

        if details.is_empty() || panel_width < name.len() + details.len() + 2 {
            return ListItem::new(name.clone()).style(name_style);
        }

        let padding = panel_width.saturating_sub(name.len() + details.len());
        let line = Line::from(vec![
            Span::styled(name.clone(), name_style),
            Span::raw(" ".repeat(padding)),
            Span::styled(details, Style::new().dark_gray()),
        ]);
        ListItem::new(line)
    }).collect();

    panel.visible_rows = area.height.saturating_sub(2) as usize;

    let position = if panel.filtered_items.is_empty() {
        " [0/0]".to_string()
    } else {
        format!(" [{}/{}]", panel.state + 1, panel.filtered_items.len())
    };

    let title = if is_searching {
        format!("{} [/: {}]{}", panel.path, panel.filter, position)
    } else if !panel.filter.is_empty() {
        format!("{} [filter: {}]{}", panel.path, panel.filter, position)
    } else {
        format!("{}{}", panel.path, position)
    };

    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom);

    let mut state = ListState::default().with_selected(Some(panel.state));
    frame.render_stateful_widget(list, area, &mut state);

    Ok(())
}
