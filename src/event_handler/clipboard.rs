use std::sync::Arc;

use crate::background_op;
use crate::context::{Context, UiState};
use crate::file_operation;

pub fn handle_copy_or_cut(context: &mut Context, mode: crate::context::ClipboardMode) {
    let label = if mode == crate::context::ClipboardMode::Cut { "Cut" } else { "Copied" };

    context.copy_source_backend = Some(Arc::clone(&context.active().backend));

    if context.active().has_selection() {
        context.copy_paths = context.active().get_selected_paths();
        context.copy_path = String::default();
        context.clipboard_mode = mode;
        context.set_status_message(&format!("{} {} items to clipboard", label, context.copy_paths.len()));
    } else {
        context.set_copy_path();
        context.copy_paths.clear();
        context.clipboard_mode = mode;
        context.set_status_message(&format!("{} to clipboard", label));
    }
}

pub fn handle_paste(context: &mut Context) {
    if context.is_operation_running() {
        context.set_status_message("Operation already in progress");
        return;
    }

    let is_cut = context.clipboard_mode == crate::context::ClipboardMode::Cut;
    let src_backend = context.copy_source_backend.clone()
        .unwrap_or_else(|| Arc::clone(&context.active().backend));
    let dst_backend = Arc::clone(&context.active().backend);

    // Multi-file paste
    if !context.copy_paths.is_empty() {
        let dest_dir = match resolve_paste_dest_dir(context) {
            Some(d) => d,
            None => return,
        };
        let items: Vec<(String, String)> = context.copy_paths.iter()
            .map(|from| {
                let name = src_backend.file_name(from).unwrap_or_default();
                let to = dst_backend.join_path(&dest_dir, &name);
                (from.clone(), to)
            })
            .collect();
        let count = items.len();
        let action = if is_cut { "Moving" } else { "Copying" };
        let desc = format!("{} {} items...", action, count);
        let progress = Arc::new(crate::background_op::TransferProgress::new());
        context.transfer_progress = Some(Arc::clone(&progress));
        let rx = background_op::spawn_copy_batch_with_backend(
            src_backend, dst_backend, items, desc.clone(), is_cut, Some(progress),
        );
        context.start_operation(rx, desc);
        return;
    }

    // Single file paste
    if context.get_copy_path().is_empty() {
        context.set_status_message("Nothing to paste — copy a file first");
        return;
    }
    match file_operation::resolve_paste_paths(context) {
        Ok((from, to)) => {
            if context.active().backend.exists(&to).unwrap_or(false) {
                context.pending_paste = Some((from, to, is_cut));
                context.confirm_popup_size = true;
                context.set_ui_state(UiState::ConfirmOverwrite);
            } else {
                spawn_paste_operation(context, from, to, is_cut);
            }
        }
        Err(e) => {
            context.set_status_message(&format!("Paste failed: {}", e));
        }
    }
}

pub(crate) fn execute_pending_paste(context: &mut Context) {
    if let Some((from, to, is_cut)) = context.pending_paste.take() {
        if context.active().backend.exists(&to).unwrap_or(false) {
            if let Err(e) = file_operation::delete_with_backend(&context.active().backend, &to) {
                context.set_status_message(&format!("Cannot remove existing file: {}", e));
                return;
            }
        }
        spawn_paste_operation(context, from, to, is_cut);
    }
}

fn resolve_paste_dest_dir(context: &mut Context) -> Option<String> {
    let panel = context.active();
    let base = panel.path.clone();
    if let Some(item) = panel.get_selected_item() {
        if panel.get_state() != 0 {
            let full = panel.backend.join_path(&base, item.trim_end_matches('/'));
            if let Ok(info) = panel.backend.metadata(&full) {
                if info.is_dir {
                    return Some(full);
                }
            }
        }
    }
    Some(base)
}

fn spawn_paste_operation(context: &mut Context, from: String, to: String, is_cut: bool) {
    let src_backend = context.copy_source_backend.clone()
        .unwrap_or_else(|| Arc::clone(&context.active().backend));
    let dst_backend = Arc::clone(&context.active().backend);

    let name = src_backend.file_name(&from)
        .unwrap_or_else(|| "item".to_string());

    let progress = Arc::new(crate::background_op::TransferProgress::new());
    context.transfer_progress = Some(Arc::clone(&progress));
    let (desc, rx) = if is_cut {
        let desc = format!("Moving {}...", name);
        let rx = background_op::spawn_move_with_backend(
            src_backend, dst_backend, from, to, desc.clone(), Some(progress),
        );
        (desc, rx)
    } else {
        let desc = format!("Copying {}...", name);
        let rx = background_op::spawn_copy_with_backend(
            src_backend, dst_backend, from, to, desc.clone(), Some(progress),
        );
        (desc, rx)
    };
    context.start_operation(rx, desc);
}

pub(crate) fn spawn_delete_operation(context: &mut Context) {
    if context.is_operation_running() {
        context.set_status_message("Operation already in progress");
        return;
    }

    let panel = context.active();
    let backend = Arc::clone(&panel.backend);

    if panel.has_selection() {
        let paths = panel.get_selected_paths();
        let count = paths.len();
        let desc = format!("Deleting {} items...", count);
        let rx = background_op::spawn_delete_batch_with_backend(backend, paths, desc.clone());
        context.start_operation(rx, desc);
        context.active_mut().clear_selection();
        return;
    }

    let selected = match panel.get_selected_item() {
        Some(item) => item.clone(),
        None => return,
    };

    let path = panel.backend.join_path(&panel.path, &selected);
    let desc = format!("Deleting {}...", selected);
    let rx = background_op::spawn_delete_with_backend(backend, path, desc.clone());
    context.start_operation(rx, desc);
}
