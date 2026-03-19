use std::fs;
use tempfile::TempDir;
use vocofo::config::Config;
use vocofo::context::Context;
use vocofo::file_operation;

// ============================================================================
// Config Loading
// ============================================================================

#[test]
fn test_config_default() {
    let config = Config::default();
    assert!(!config.general.show_hidden);
    assert_eq!(config.general.default_path, ".");
    assert!(!config.general.show_preview_on_start);
}

#[test]
fn test_config_load_missing_file() {
    // Should return defaults when config file doesn't exist
    let nonexistent = std::path::PathBuf::from("/tmp/vocofo_test_nonexistent_config.toml");
    let config = Config::load_from(&nonexistent);
    assert!(!config.general.show_hidden);
    assert_eq!(config.general.default_path, ".");
}

#[test]
fn test_config_with_context() {
    let mut config = Config::default();
    config.general.show_preview_on_start = true;

    let context = Context::with_config(config).unwrap();
    assert!(context.show_preview);
}

#[test]
fn test_config_show_hidden_propagates() {
    let mut config = Config::default();
    config.general.show_hidden = true;

    let context = Context::with_config(config).unwrap();
    assert!(context.panels[0].show_hidden);
    assert!(context.panels[1].show_hidden);
}

// ============================================================================
// Hidden Files Filtering
// ============================================================================

#[test]
fn test_hidden_files_not_shown_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("visible.txt"), "").unwrap();
    fs::write(base.join(".hidden"), "").unwrap();
    fs::create_dir(base.join(".hidden_dir")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    context.panels[0].show_hidden = false;
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(context.panels[0].items.contains(&"visible.txt".to_string()));
    assert!(!context.panels[0].items.contains(&".hidden".to_string()));
    assert!(
        !context.panels[0]
            .items
            .contains(&".hidden_dir/".to_string())
    );
}

#[test]
fn test_hidden_files_shown_when_enabled() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("visible.txt"), "").unwrap();
    fs::write(base.join(".hidden"), "").unwrap();
    fs::create_dir(base.join(".hidden_dir")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    context.panels[0].show_hidden = true;
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(context.panels[0].items.contains(&"visible.txt".to_string()));
    assert!(context.panels[0].items.contains(&".hidden".to_string()));
    assert!(
        context.panels[0]
            .items
            .contains(&".hidden_dir/".to_string())
    );
}

#[test]
fn test_toggle_hidden_files() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("visible.txt"), "").unwrap();
    fs::write(base.join(".secret"), "").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    context.panels[0].show_hidden = false;
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(!context.panels[0].items.contains(&".secret".to_string()));

    // Toggle on
    context.panels[0].show_hidden = true;
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(context.panels[0].items.contains(&".secret".to_string()));

    // Toggle off
    context.panels[0].show_hidden = false;
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(!context.panels[0].items.contains(&".secret".to_string()));
}
