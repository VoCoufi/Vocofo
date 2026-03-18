use std::fs;
use tempfile::TempDir;
use vocofo::context::{Context, UiState};
use vocofo::file_operation;

fn create_search_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("apple.txt"), "").unwrap();
    fs::write(base.join("banana.txt"), "").unwrap();
    fs::write(base.join("cherry.log"), "").unwrap();
    fs::create_dir(base.join("documents")).unwrap();
    fs::create_dir(base.join("downloads")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    (context, temp_dir)
}

// ============================================================================
// Filter Tests
// ============================================================================

#[test]
fn test_apply_filter_empty() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = String::new();
    context.panels[0].apply_filter();

    // All items should be visible
    assert_eq!(context.panels[0].filtered_items.len(), context.panels[0].items.len());
}

#[test]
fn test_apply_filter_matches_files() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "txt".to_string();
    context.panels[0].apply_filter();

    // Should match apple.txt, banana.txt + ../
    assert!(context.panels[0].filtered_items.contains(&"../".to_string()));
    assert!(context.panels[0].filtered_items.contains(&"apple.txt".to_string()));
    assert!(context.panels[0].filtered_items.contains(&"banana.txt".to_string()));
    assert!(!context.panels[0].filtered_items.contains(&"cherry.log".to_string()));
}

#[test]
fn test_apply_filter_matches_folders() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "do".to_string();
    context.panels[0].apply_filter();

    // Should match documents/, downloads/ + ../
    assert!(context.panels[0].filtered_items.contains(&"documents/".to_string()));
    assert!(context.panels[0].filtered_items.contains(&"downloads/".to_string()));
    assert!(!context.panels[0].filtered_items.contains(&"apple.txt".to_string()));
}

#[test]
fn test_apply_filter_case_insensitive() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "APPLE".to_string();
    context.panels[0].apply_filter();

    assert!(context.panels[0].filtered_items.contains(&"apple.txt".to_string()));
}

#[test]
fn test_apply_filter_no_matches() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "zzzzz".to_string();
    context.panels[0].apply_filter();

    // Only ../ should remain
    assert_eq!(context.panels[0].filtered_items.len(), 1);
    assert_eq!(context.panels[0].filtered_items[0], "../");
}

#[test]
fn test_apply_filter_resets_state() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].state = 3;
    context.panels[0].filter = "txt".to_string();
    context.panels[0].apply_filter();

    assert_eq!(context.panels[0].state, 0);
}

#[test]
fn test_clear_filter() {
    let (mut context, _temp) = create_search_context();

    let total_items = context.panels[0].items.len();

    context.panels[0].filter = "txt".to_string();
    context.panels[0].apply_filter();
    assert!(context.panels[0].filtered_items.len() < total_items);

    context.panels[0].clear_filter();

    assert!(context.panels[0].filter.is_empty());
    assert_eq!(context.panels[0].filtered_items.len(), total_items);
    assert_eq!(context.panels[0].state, 0);
}

#[test]
fn test_get_selected_item_uses_filtered_items() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "banana".to_string();
    context.panels[0].apply_filter();

    // State 0 = ../, State 1 = banana.txt
    context.panels[0].state = 1;
    assert_eq!(context.panels[0].get_selected_item(), Some(&"banana.txt".to_string()));
}

#[test]
fn test_filter_parent_always_visible() {
    let (mut context, _temp) = create_search_context();

    context.panels[0].filter = "nonexistent_filter_term".to_string();
    context.panels[0].apply_filter();

    assert!(context.panels[0].filtered_items.contains(&"../".to_string()));
}

#[test]
fn test_list_children_applies_existing_filter() {
    let (mut context, _temp) = create_search_context();

    // Set a filter before refreshing
    context.panels[0].filter = "txt".to_string();
    context.panels[0].invalidate_directory_cache();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Filter should be applied to new items
    assert!(context.panels[0].filtered_items.iter().all(|i| i == "../" || i.contains("txt")));
}
