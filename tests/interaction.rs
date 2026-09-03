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

    // Move to North (outside deadzone 32, e.g. y = 450)
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
fn test_at_006_and_007_commit_left_click() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Click in sector S (y = 150)
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 150)));
    assert_eq!(eff, InteractionEffect::Committed(Sector::S));
    assert_eq!(fsm.state, State::Idle);

    // Test far click beyond radius
    fsm.transition(InteractionEvent::WinEscDown(anchor));
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 5000)));
    assert_eq!(eff, InteractionEffect::Committed(Sector::S));
    assert_eq!(fsm.state, State::Idle);
}

#[test]
fn test_at_008_deadzone_left_click_noop() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(100, 100);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Click inside deadzone (distance = 10 < 32)
    let eff = fsm.transition(InteractionEvent::LButtonDown(Point::new(100, 110)));
    assert_eq!(eff, InteractionEffect::NoOpInDeadzone);
    assert!(fsm.is_active(), "Must remain active after deadzone click");
}

#[test]
fn test_at_009_right_click_cancels() {
    let mut fsm = InteractionFsm::new(GeometryConfig::default());
    let anchor = Point::new(200, 200);
    fsm.transition(InteractionEvent::WinEscDown(anchor));

    // Right click anywhere (even deadzone or far away) cancels
    let eff = fsm.transition(InteractionEvent::RButtonDown(Point::new(200, 200)));
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
