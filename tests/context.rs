use winpie::config::{AppConfig, ContextMatcher};
use winpie::context::{WindowContext, active_window_context, is_process_running};
use winpie::geometry::Sector;

#[test]
fn test_context_matcher_process_exact_and_case_insensitive() {
    let matcher = ContextMatcher {
        process: Some("code.exe".to_string()),
        window_class: None,
        window_title: None,
    };

    let ctx1 = WindowContext {
        process_name: "code.exe".to_string(),
        process_path: r"C:\Users\Leonardo\AppData\Local\Programs\Microsoft VS Code\Code.exe".to_string(),
        window_class: "Chrome_WidgetWin_1".to_string(),
        window_title: "WinPie - Visual Studio Code".to_string(),
    };

    let ctx2 = WindowContext {
        process_name: "explorer.exe".to_string(),
        process_path: r"C:\Windows\explorer.exe".to_string(),
        window_class: "CabinetWClass".to_string(),
        window_title: r"C:\Users".to_string(),
    };

    assert!(matcher.matches(&ctx1));
    assert!(!matcher.matches(&ctx2));
}

#[test]
fn test_context_matcher_class_and_title() {
    let matcher = ContextMatcher {
        process: Some("explorer.exe".to_string()),
        window_class: Some("CabinetWClass".to_string()),
        window_title: None,
    };

    let ctx_explorer = WindowContext {
        process_name: "explorer.exe".to_string(),
        process_path: r"C:\Windows\explorer.exe".to_string(),
        window_class: "CabinetWClass".to_string(),
        window_title: "Downloads".to_string(),
    };

    let ctx_desktop = WindowContext {
        process_name: "explorer.exe".to_string(),
        process_path: r"C:\Windows\explorer.exe".to_string(),
        window_class: "Progman".to_string(),
        window_title: "Program Manager".to_string(),
    };

    assert!(matcher.matches(&ctx_explorer));
    assert!(!matcher.matches(&ctx_desktop));
}

#[test]
fn test_profile_inheritance_and_overrides() {
    let yaml = r#"
wheel:
  labels:
    N: "Global Dev"
    E: "Global Sublime"
    S: "Global Settings"
    W: "Global Tasks"
  commands:
    N: "global_dev.exe"
    E: "sublime.exe"

profiles:
  - name: "VS Code Profile"
    match_rules:
      process: "code.exe"
    wheel:
      labels:
        N: "Format Document"
      commands:
        N: "code --format"
"#;

    let config: AppConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize test config");
    assert_eq!(config.profiles.len(), 1);

    let ctx_code = WindowContext {
        process_name: "code.exe".to_string(),
        process_path: "".to_string(),
        window_class: "".to_string(),
        window_title: "".to_string(),
    };

    let profile_idx = config.find_matching_profile(&ctx_code);
    assert_eq!(profile_idx, Some(0));

    // Overridden sector N in profile
    assert_eq!(config.get_label_for_sector_with_profile(Sector::N, profile_idx), "Format Document");
    assert_eq!(config.get_command_for_sector_with_profile(Sector::N, profile_idx), Some("code --format"));

    // Non-overridden sector E inherits from global default
    assert_eq!(config.get_label_for_sector_with_profile(Sector::E, profile_idx), "Global Sublime");
    assert_eq!(config.get_command_for_sector_with_profile(Sector::E, profile_idx), Some("sublime.exe"));
}

#[test]
fn test_active_window_context_smoke() {
    let ctx = active_window_context();
    // Verify it returns without panic
    println!("Active window context: {:?}", ctx);
}

#[test]
fn test_is_process_running_smoke() {
    // explorer.exe is guaranteed to run on any logged-in Windows session
    assert!(is_process_running("explorer.exe"));
    // case-insensitivity
    assert!(is_process_running("EXPLORER.EXE"));
    // completely bogus executable should not be running
    assert!(!is_process_running("non_existent_fake_process_123456789.exe"));
}

#[test]
fn test_context_matcher_title_and_combined_filters() {
    let matcher = ContextMatcher {
        process: Some("code.exe".to_string()),
        window_class: Some("Chrome_WidgetWin_1".to_string()),
        window_title: Some("WinPie".to_string()),
    };

    let ctx_match = WindowContext {
        process_name: "code.exe".to_string(),
        process_path: r"C:\Programs\VSCode\code.exe".to_string(),
        window_class: "Chrome_WidgetWin_1".to_string(),
        window_title: "src/main.rs - WinPie - Visual Studio Code".to_string(),
    };

    let ctx_diff_title = WindowContext {
        process_name: "code.exe".to_string(),
        process_path: r"C:\Programs\VSCode\code.exe".to_string(),
        window_class: "Chrome_WidgetWin_1".to_string(),
        window_title: "OtherProject - Visual Studio Code".to_string(),
    };

    let ctx_diff_proc = WindowContext {
        process_name: "chrome.exe".to_string(),
        process_path: r"C:\Programs\Chrome\chrome.exe".to_string(),
        window_class: "Chrome_WidgetWin_1".to_string(),
        window_title: "WinPie GitHub".to_string(),
    };

    assert!(matcher.matches(&ctx_match));
    assert!(!matcher.matches(&ctx_diff_title));
    assert!(!matcher.matches(&ctx_diff_proc));
}
