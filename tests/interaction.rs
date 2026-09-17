use winpie::geometry::{GeometryConfig, Point, Sector};
use winpie::interaction::{InteractionEffect, InteractionEvent, InteractionFsm, State};

#[test]
fn test_at_001_activation() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    assert_eq!(fsm.state, State::Idle);

    let anchor = Point::new(300, 400);
    let effect = fsm.transition(InteractionEvent::WinEscDown(anchor));
    assert_eq!(effect, InteractionEffect::Activated { anchor });
    assert_eq!(fsm.state, State::Active { anchor, hover: None });
}

#[test]
fn test_at_003_and_004_hover_and_deadzone() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(500, 500);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Initially in deadzone
    let eff = fsm.transition(InteractionEvent::MouseMove(anchor));
    assert_eq!(eff, InteractionEffect::None);
    assert_eq!(fsm.hover_selection(), None);

    // Move to North (outside deadzone 32, within radius 180, e.g. y = 450)
    let eff = fsm.transition(InteractionEvent::MouseMove(Point::new(500, 450)));
    assert_eq!(eff, InteractionEffect::HoverChanged { from: None, to: Some(Sector::N) });
    assert_eq!(fsm.hover_selection(), Some(Sector::N));

    // Move slightly within North sector: no hover change event
    let eff = fsm.transition(InteractionEvent::MouseMove(Point::new(505, 440)));
    assert_eq!(eff, InteractionEffect::None);

    // Move to East
    let eff = fsm.transition(InteractionEvent::MouseMove(Point::new(560, 500)));
    assert_eq!(eff, InteractionEffect::HoverChanged { from: Some(Sector::N), to: Some(Sector::E) });
    assert_eq!(fsm.hover_selection(), Some(Sector::E));

    // Return to deadzone
    let eff = fsm.transition(InteractionEvent::MouseMove(anchor));
    assert_eq!(eff, InteractionEffect::HoverChanged { from: Some(Sector::E), to: None });
    assert_eq!(fsm.hover_selection(), None);
    assert!(fsm.is_active());
}

#[test]
fn test_at_005a_win_release_commit() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Release Win while cursor is in valid sector (e.g. East at 260, 200) with no keys remaining -> IDLE
    let eff = fsm.transition(InteractionEvent::WinUp(Point::new(260, 200), false));
    assert_eq!(eff, InteractionEffect::Committed(Sector::E));
    assert_eq!(fsm.state, State::Idle);

    // Release Win while cursor in valid sector with Esc still held -> WAIT_RELEASE
    fsm.transition(InteractionEvent::WinEscDown(anchor));
    let eff = fsm.transition(InteractionEvent::WinUp(Point::new(260, 200), true));
    assert_eq!(eff, InteractionEffect::Committed(Sector::E));
    assert_eq!(fsm.state, State::WaitRelease);
}

#[test]
fn test_at_005b_esc_release_noop() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Esc release produces no FSM event / leaves state unchanged
    assert_eq!(fsm.state, State::Active { anchor, hover: None });
}

#[test]
fn test_at_006_left_click_commit() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Click in valid sector S (y = 150, distance 50: within [32, 180])
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 150), false));
    assert_eq!(eff, InteractionEffect::Committed(Sector::S));
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_007_out_of_bounds_click() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Left click outside radius (5000px away) -> Unified Cancel
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 5000), false));
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_008_deadzone_click() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Left click inside deadzone (distance 10 < 32) -> Unified Cancel
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 110), false));
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_009_right_click_cancels() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Right click anywhere cancels
    let eff = fsm.transition(InteractionEvent::RButtonDown(Point::new(200, 200), false));
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_011_repeated_activation() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Repeated WinEsc while active does nothing
    let eff = fsm.transition(InteractionEvent::WinEscDown(Point::new(400, 400)));
    assert_eq!(eff, InteractionEffect::None);
    assert_eq!(fsm.active_anchor(), Some(anchor));
}

#[test]
fn test_at_015_fatal_error() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    let eff = fsm.transition(InteractionEvent::FatalError);
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_019_wait_release_and_rearm() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Commit while activation keys held -> WaitRelease
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(260, 200), true));
    assert_eq!(eff, InteractionEffect::Committed(Sector::E));
    assert_eq!(fsm.state, State::WaitRelease);
    assert!(fsm.is_waiting_release());

    // All keys released -> re-arms to Idle
    let eff = fsm.transition(InteractionEvent::AllKeysUp);
    assert_eq!(eff, InteractionEffect::Rearmed);
    assert_eq!(fsm.state, State::Idle);

    // Now activation works again
    let eff = fsm.transition(InteractionEvent::WinEscDown(Point::new(300, 300)));
    assert_eq!(eff, InteractionEffect::Activated { anchor: Point::new(300, 300) });
    assert_eq!(fsm.state, State::Active { anchor: Point::new(300, 300), hover: None });

    // Direct activation from WaitRelease (self-healing)
    let _ = fsm.transition(InteractionEvent::LButtonDown(Point::new(300, 400), true));
    assert_eq!(fsm.state, State::WaitRelease);
    let eff = fsm.transition(InteractionEvent::WinEscDown(Point::new(400, 400)));
    assert_eq!(eff, InteractionEffect::Activated { anchor: Point::new(400, 400) });
    assert_eq!(fsm.state, State::Active { anchor: Point::new(400, 400), hover: None });
}

#[test]
fn test_modal_menu_spawn_and_keyup_execution() {
    use std::collections::HashMap;
    use winpie::config::{MenuDefinition, MenuItem};

    let mut items = HashMap::new();
    items.insert('a', MenuItem {
        label: "VS Code".to_string(),
        tooltip: Some("Launch VS Code".to_string()),
        command: Some("code.exe".to_string()),
        menu: None,
    });
    items.insert('s', MenuItem {
        label: "Sublime".to_string(),
        tooltip: None,
        command: Some("sublime.exe".to_string()),
        menu: None,
    });

    let menu = MenuDefinition {
        title: "Dev Tools".to_string(),
        tooltip: None,
        items,
    };

    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);

    // 1. Spawn menu
    let eff = fsm.transition(InteractionEvent::MenuSpawn { anchor, menu: menu.clone() });
    assert_eq!(eff, InteractionEffect::MenuSpawned { anchor, menu: menu.clone() });
    assert!(fsm.is_modal_menu());
    assert_eq!(fsm.active_anchor(), Some(anchor));
    assert_eq!(fsm.highlighted_menu_key(), None);

    // 2. KeyDown 'a' -> Highlight changed
    let eff = fsm.transition(InteractionEvent::MenuKeyDown('a'));
    assert!(matches!(eff, InteractionEffect::MenuHighlightChanged { key: Some('a'), .. }));
    assert_eq!(fsm.highlighted_menu_key(), Some('a'));

    // 3. KeyUp 'a' -> Executes code.exe and transitions to Idle
    let eff = fsm.transition(InteractionEvent::MenuKeyUp('a', false));
    assert_eq!(eff, InteractionEffect::MenuExecuted {
        command: "code.exe".to_string(),
        label: "VS Code".to_string(),
    });
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_modal_menu_drill_down_tab_loop_and_shift_tab() {
    use std::collections::HashMap;
    use winpie::config::{MenuDefinition, MenuItem};

    let mut child_items = HashMap::new();
    child_items.insert('c', MenuItem {
        label: "Cargo Clean".to_string(),
        tooltip: None,
        command: Some("cargo clean".to_string()),
        menu: None,
    });

    let child_menu = MenuDefinition {
        title: "Build Submenu".to_string(),
        tooltip: None,
        items: child_items,
    };

    let mut root_items = HashMap::new();
    root_items.insert('d', MenuItem {
        label: "Build Tools".to_string(),
        tooltip: None,
        command: None,
        menu: Some(child_menu.clone()),
    });

    let root_menu = MenuDefinition {
        title: "Dev Tools".to_string(),
        tooltip: None,
        items: root_items,
    };

    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(300, 300);
    fsm.transition(InteractionEvent::MenuSpawn { anchor, menu: root_menu.clone() });

    // KeyDown 'd' then KeyUp 'd' -> Drills down into child menu
    fsm.transition(InteractionEvent::MenuKeyDown('d'));
    let eff = fsm.transition(InteractionEvent::MenuKeyUp('d', false));
    assert_eq!(eff, InteractionEffect::MenuDrillDown { menu: child_menu.clone() });
    assert_eq!(fsm.current_menu(), Some(&child_menu));
    assert_eq!(fsm.menu_nav_stack().unwrap().len(), 2);

    // Shift+Tab -> Backtracks to root menu
    let eff = fsm.transition(InteractionEvent::MenuTab { shift: true });
    assert_eq!(eff, InteractionEffect::MenuBacktracked { current_menu: root_menu.clone() });
    assert_eq!(fsm.current_menu(), Some(&root_menu));
    assert_eq!(fsm.menu_nav_stack().unwrap().len(), 1);

    // Drill down again
    fsm.transition(InteractionEvent::MenuKeyDown('d'));
    fsm.transition(InteractionEvent::MenuKeyUp('d', false));
    assert_eq!(fsm.menu_nav_stack().unwrap().len(), 2);

    // Tab -> Loops/Resets back to root menu
    let eff = fsm.transition(InteractionEvent::MenuTab { shift: false });
    assert_eq!(eff, InteractionEffect::MenuResetToRoot { current_menu: root_menu.clone() });
    assert_eq!(fsm.current_menu(), Some(&root_menu));
    assert_eq!(fsm.menu_nav_stack().unwrap().len(), 1);
}

#[test]
fn test_modal_menu_cancel_on_esc_and_rmb() {
    use std::collections::HashMap;
    use winpie::config::{MenuDefinition, MenuItem};

    let mut items = HashMap::new();
    items.insert('x', MenuItem {
        label: "Test".to_string(),
        tooltip: None,
        command: Some("test.exe".to_string()),
        menu: None,
    });

    let menu = MenuDefinition {
        title: "Test Menu".to_string(),
        tooltip: None,
        items,
    };

    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);

    // Cancel via Esc while key held down
    fsm.transition(InteractionEvent::MenuSpawn { anchor, menu: menu.clone() });
    fsm.transition(InteractionEvent::MenuKeyDown('x'));
    let eff = fsm.transition(InteractionEvent::MenuEsc(false));
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);

    // Cancel via RMB
    fsm.transition(InteractionEvent::MenuSpawn { anchor, menu });
    let eff = fsm.transition(InteractionEvent::RButtonDown(Point::new(100, 100), false));
    assert_eq!(eff, InteractionEffect::Cancelled);
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_modal_menu_mouse_hover_and_click() {
    use std::collections::HashMap;
    use winpie::config::{MenuDefinition, MenuItem};

    let mut items = HashMap::new();
    items.insert('a', MenuItem {
        label: "VS Code".to_string(),
        tooltip: Some("Launch VS Code".to_string()),
        command: Some("code.exe".to_string()),
        menu: None,
    });
    items.insert('t', MenuItem {
        label: "Terminal".to_string(),
        tooltip: Some("Open Terminal".to_string()),
        command: Some("wt.exe".to_string()),
        menu: None,
    });

    let menu = MenuDefinition {
        title: "Dev Tools".to_string(),
        tooltip: None,
        items,
    };

    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(400, 400);

    fsm.transition(InteractionEvent::MenuSpawn { anchor, menu });

    // 1. Mouse hover over 'a'
    let eff = fsm.transition(InteractionEvent::MenuHover(Some('a')));
    assert_eq!(fsm.highlighted_menu_key(), Some('a'));
    if let InteractionEffect::MenuHighlightChanged { key, item } = eff {
        assert_eq!(key, Some('a'));
        assert_eq!(item.unwrap().label, "VS Code");
    } else {
        panic!("Expected MenuHighlightChanged effect");
    }

    // 2. Mouse hover out of items
    let eff = fsm.transition(InteractionEvent::MenuHover(None));
    assert_eq!(fsm.highlighted_menu_key(), None);
    assert!(matches!(eff, InteractionEffect::MenuHighlightChanged { key: None, item: None }));

    // 3. Mouse click on 't' -> Direct execution
    let eff = fsm.transition(InteractionEvent::MenuSelect('t', false));
    assert_eq!(eff, InteractionEffect::MenuExecuted {
        command: "wt.exe".to_string(),
        label: "Terminal".to_string(),
    });
    assert_eq!(fsm.state, State::Idle);
}

