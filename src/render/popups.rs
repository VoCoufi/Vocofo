use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::Style,
    widgets::*,
};

use crate::context::Context;

use super::{RenderResult, centered_rect_dialog, create_sized_button};

pub fn popup_confirm_delete(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let selected_item = context
        .active()
        .get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?;

    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Confirm Deletion ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let warning = Paragraph::new("⚠️ Warning: This action cannot be undone!")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let message = Paragraph::new(format!(
        "Are you sure you want to delete \"{}\"?",
        selected_item
    ))
    .alignment(Alignment::Center);

    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
        ])
        .split(chunks[4]);

    let selected = context.get_confirm_button_selected().unwrap_or(false);
    frame.render_widget(warning, chunks[0]);
    frame.render_widget(message, chunks[2]);
    frame.render_widget(create_sized_button("Yes", selected), button_chunks[1]);
    frame.render_widget(create_sized_button("No", !selected), button_chunks[3]);

    Ok(())
}

pub fn popup_name_creation(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    render_input_popup(
        frame,
        context,
        " Create folder ",
        "Write the name of the folder.",
        Color::Red,
    )
}

pub fn popup_create_file(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    render_input_popup(
        frame,
        context,
        " Create file ",
        "Write the name of the file.",
        Color::Green,
    )
}

pub fn popup_rename(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    render_input_popup(frame, context, " Rename ", "Enter new name:", Color::Cyan)
}

pub fn popup_chmod(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 50, 8);

    let dialog_block = Block::default()
        .title(" chmod ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let label = Paragraph::new("Enter octal mode (e.g. 755):")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let empty = String::new();
    let input_text = context.get_input().unwrap_or(&empty);
    let para = Paragraph::new(input_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .padding(Padding::new(1, 1, 0, 0)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(label, chunks[1]);
    frame.render_widget(para, chunks[3]);
    Ok(())
}

pub fn popup_confirm_overwrite(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let file_name = context
        .pending_paste
        .as_ref()
        .and_then(|(_, to, _)| {
            std::path::Path::new(to)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "file".to_string());

    let area = centered_rect_dialog(frame.area(), 80, 10);
    let dialog_block = Block::default()
        .title(" Confirm Overwrite ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let warning = Paragraph::new("⚠️ File already exists!")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    let message =
        Paragraph::new(format!("Overwrite \"{}\"?", file_name)).alignment(Alignment::Center);

    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
        ])
        .split(chunks[4]);

    let selected = context.get_confirm_button_selected().unwrap_or(false);
    frame.render_widget(warning, chunks[0]);
    frame.render_widget(message, chunks[2]);
    frame.render_widget(create_sized_button("Yes", selected), button_chunks[1]);
    frame.render_widget(create_sized_button("No", !selected), button_chunks[3]);
    Ok(())
}

pub fn popup_bookmark_list(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let connections = &context.config.connections;
    let height = (connections.len() as u16 + 4).min(20);
    let area = centered_rect_dialog(frame.area(), 60, height);

    let dialog_block = Block::default()
        .title(" Bookmarks ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);
    let inner_area = dialog_block.inner(area);

    let items: Vec<Line> = connections
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let text = format!(
                " {} ({}://{}@{}:{}) ",
                profile.name, profile.protocol, profile.username, profile.host, profile.port
            );
            let style = if i == context.bookmark_selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(text).style(style)
        })
        .collect();

    frame.render_widget(Paragraph::new(items), inner_area);
    Ok(())
}

pub fn popup_bookmark_name(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    render_input_popup(
        frame,
        context,
        " Save Bookmark ",
        "Bookmark name:",
        Color::Cyan,
    )
}

pub fn popup_command_palette(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    use crate::event_handler::PALETTE_ACTIONS;

    let state = match context.command_palette.as_ref() {
        Some(s) => s,
        None => return Ok(()),
    };

    let visible_count = state.filtered_indices.len().min(15);
    let height = (visible_count as u16 + 5).max(6);
    let area = centered_rect_dialog(frame.area(), 50, height);

    let dialog_block = Block::default()
        .title(" Command Palette ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);
    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner_area);

    let cursor = if state.filter.is_empty() { "▎" } else { "" };
    let input_line = Line::from(vec![
        Span::styled(
            " > ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", state.filter, cursor),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(input_line), chunks[0]);
    frame.render_widget(
        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    // Build all lines with section headers inserted
    let label_width = chunks[2].width.saturating_sub(10) as usize;
    let mut all_lines: Vec<Line> = Vec::new();
    let mut selected_line_idx: usize = 0;
    let mut last_section = "";
    let is_filtered = !state.filter.is_empty();

    for (sel_idx, &action_idx) in state.filtered_indices.iter().enumerate() {
        let action = &PALETTE_ACTIONS[action_idx];

        // Insert section header when section changes (only when not filtering)
        if !is_filtered && action.section != last_section {
            last_section = action.section;
            all_lines.push(Line::from(vec![Span::styled(
                format!(" {}", action.section),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )]));
        }

        if sel_idx == state.selected {
            selected_line_idx = all_lines.len();
        }

        let padded_label = format!(
            "   {:<width$}",
            action.label,
            width = label_width.saturating_sub(2)
        );
        let is_selected = sel_idx == state.selected;
        let (style, shortcut_style) = if is_selected {
            (
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
                Style::default().bg(Color::Blue).fg(Color::Gray),
            )
        } else {
            (
                Style::default().fg(Color::White),
                Style::default().fg(Color::DarkGray),
            )
        };
        all_lines.push(Line::from(vec![
            Span::styled(padded_label, style),
            Span::styled(format!("{:>6} ", action.shortcut), shortcut_style),
        ]));
    }

    // Scroll to keep selected visible
    let max_visible = chunks[2].height as usize;
    let scroll_offset = if selected_line_idx >= max_visible {
        selected_line_idx - max_visible + 1
    } else {
        0
    };

    let items: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll_offset)
        .take(max_visible)
        .collect();

    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(" No matching actions").style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
    } else {
        frame.render_widget(Paragraph::new(items), chunks[2]);
    }
    Ok(())
}

pub fn popup_settings(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 55, 14);

    let dialog_block = Block::default()
        .title(" Settings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);
    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner_area);

    let state = context.settings_state.as_ref();
    let focused = state.map(|s| s.focused_field).unwrap_or(0);
    let editing_path = state.map(|s| s.editing_path).unwrap_or(false);

    let field_style = |idx: usize| -> Style {
        if idx == focused {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    let layout_val = context.config.general.panel_layout.as_str();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Panel Layout   ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("◂ {:^12} ▸", layout_val), field_style(0)),
        ])),
        chunks[1],
    );

    let hidden_val = if context.config.general.show_hidden {
        " On "
    } else {
        " Off"
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Show Hidden    ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("  {:^12}  ", hidden_val), field_style(1)),
        ])),
        chunks[3],
    );

    let preview_val = if context.config.general.show_preview_on_start {
        " On "
    } else {
        " Off"
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Show Preview   ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("  {:^12}  ", preview_val), field_style(2)),
        ])),
        chunks[5],
    );

    let path_val = if editing_path {
        state.map(|s| s.path_input.as_str()).unwrap_or("")
    } else {
        &context.config.general.default_path
    };
    let path_suffix = if editing_path && focused == 3 {
        "▎"
    } else {
        ""
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Default Path   ", Style::default().fg(Color::Yellow)),
            Span::styled(format!(" {}{} ", path_val, path_suffix), field_style(3)),
        ])),
        chunks[7],
    );

    let hint = if editing_path {
        "Enter: confirm  Esc: cancel"
    } else {
        "↑↓: navigate  ◂▸/Space: change  Enter: edit path  Esc: save & close"
    };
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[9],
    );
    Ok(())
}

pub fn popup_connect_dialog(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let dialog = match context.connect_dialog.as_ref() {
        Some(d) => d,
        None => return Ok(()),
    };

    let r = frame.area();
    let width = (r.width * 70 / 100).clamp(40, 70);
    let height = 22_u16.min(r.height.saturating_sub(2));
    let x = (r.width.saturating_sub(width)) / 2;
    let y = (r.height.saturating_sub(height)) / 2;
    let area = Rect::new(r.x + x, r.y + y, width, height);

    let dialog_block = Block::default()
        .title(" Connect to Remote Server ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);
    let inner = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    let focused = dialog.focused_field;

    let proto_text = match dialog.protocol {
        crate::context::ConnectionProtocol::Sftp => "< SFTP >",
        crate::context::ConnectionProtocol::Ftp => "< FTP  >",
    };
    let proto_style = if focused == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(
        Paragraph::new(format!("  Protocol: {}", proto_text)).style(proto_style),
        chunks[0],
    );

    let fields = [
        ("Host", &dialog.host, false),
        ("Port", &dialog.port, false),
        ("Username", &dialog.username, false),
        ("Password", &dialog.password, true),
        ("SSH Key", &dialog.key_path, false),
    ];

    for (i, (label, value, is_password)) in fields.iter().enumerate() {
        let field_idx = i + 1;
        let is_focused = focused == field_idx;
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        let display_value = if *is_password && !value.is_empty() {
            "*".repeat(value.len())
        } else {
            (*value).clone()
        };
        let cursor_suffix = if is_focused { "▎" } else { "" };

        let para = Paragraph::new(format!("{}{}", display_value, cursor_suffix))
            .block(
                Block::default()
                    .title(format!(" {} ", label))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
            .style(if is_focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            });
        frame.render_widget(para, chunks[i + 2]);
    }

    if let Some(ref err) = dialog.error_message {
        frame.render_widget(
            Paragraph::new(format!("  {}", err))
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: false }),
            chunks[7],
        );
    } else {
        frame.render_widget(
            Paragraph::new("  [Tab] Next  [↑↓] Protocol  [Enter] Connect  [Esc] Cancel")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[7],
        );
    }
    Ok(())
}

/// Common input popup renderer
fn render_input_popup(
    frame: &mut Frame,
    context: &mut Context,
    title: &str,
    label: &str,
    border_color: Color,
) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);
    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let label_widget = Paragraph::new(label).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let empty = String::new();
    let input_text = context.get_input().unwrap_or(&empty);
    let para = Paragraph::new(input_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .padding(Padding::new(1, 1, 0, 0)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(label_widget, chunks[1]);
    frame.render_widget(para, chunks[3]);
    Ok(())
}
