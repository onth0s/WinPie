use std::time::Instant;
use winpie::config::AppConfig;
use winpie::context::capture_invocation_context;
use winpie::geometry::{evaluate_hover, GeometryConfig, Point};
use winpie::interaction::{InteractionEvent, InteractionFsm};

#[test]
fn test_context_capture_latency_sub_2ms() {
    // Warm up
    let _ = capture_invocation_context(Point::new(100, 100));

    let iterations = 20;
    let start = Instant::now();
    for _ in 0..iterations {
        let ctx = capture_invocation_context(Point::new(500, 500));
        assert!(ctx.invocation_id > 0);
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_secs_f64() * 1000.0 / (iterations as f64);

    println!(
        "[PERF] Context capture avg latency: {:.3}ms over {} iterations",
        avg_ms, iterations
    );

    // INV-CTX-002: Context capture latency must be <= 2.0 ms on average
    assert!(
        avg_ms < 2.0,
        "Context capture avg latency {:.3}ms exceeded 2.0ms invariant threshold",
        avg_ms
    );
}

#[test]
fn test_sector_classification_throughput() {
    let config = GeometryConfig {
        radius: 200.0,
        deadzone: 40.0,
        rotation_degrees: 0.0,
        slices: 8,
    };
    let anchor = Point::new(500, 500);

    let points = [
        Point::new(500, 300), // N
        Point::new(700, 300), // NE
        Point::new(700, 500), // E
        Point::new(700, 700), // SE
        Point::new(500, 700), // S
        Point::new(300, 700), // SW
        Point::new(300, 500), // W
        Point::new(300, 300), // NW
        Point::new(510, 510), // deadzone
    ];

    let iterations = 100_000;
    let start = Instant::now();
    let mut dummy_count = 0usize;

    for i in 0..iterations {
        let pt = points[i % points.len()];
        if evaluate_hover(anchor, pt, &config).is_some() {
            dummy_count += 1;
        }
    }

    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / (iterations as f64);

    println!(
        "[PERF] Sector classification avg latency: {:.1}ns over {} classifications",
        avg_ns, iterations
    );

    assert!(dummy_count > 0);
    // Classification must be sub-microsecond (< 1000 ns)
    assert!(
        avg_ns < 1000.0,
        "Sector classification took {:.1}ns, expected < 1000ns",
        avg_ns
    );
}

#[test]
fn test_fsm_transition_throughput() {
    let geom_config = GeometryConfig {
        radius: 200.0,
        deadzone: 40.0,
        rotation_degrees: 0.0,
        slices: 8,
    };
    let mut fsm = InteractionFsm::new(geom_config);

    let anchor = Point::new(500, 500);
    let iterations = 50_000;

    let start = Instant::now();
    for _ in 0..iterations {
        // Activate -> Hover -> Commit -> KeyUp
        let _ = fsm.transition(InteractionEvent::WinEscDown(anchor));
        let _ = fsm.transition(InteractionEvent::MouseMove(Point::new(700, 500)));
        let _ = fsm.transition(InteractionEvent::WinUp(Point::new(700, 500), false));
        let _ = fsm.transition(InteractionEvent::AllKeysUp);
    }
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / ((iterations * 4) as f64);

    println!(
        "[PERF] FSM transition avg latency: {:.1}ns across {} transitions",
        avg_ns,
        iterations * 4
    );

    assert!(
        avg_ns < 500.0,
        "FSM transition took {:.1}ns, expected < 500ns",
        avg_ns
    );
}

#[test]
fn test_profile_resolution_throughput() {
    let yaml = r#"
wheel:
  labels:
    N: "Default N"
profiles:
  - name: "IDE Specific"
    when:
      process_name: "code.exe"
      window_title_contains: "WinPie"
    wheel:
      labels:
        N: "VS Code WinPie N"
  - name: "Browser General"
    when:
      process_name: "chrome.exe"
    wheel:
      labels:
        N: "Chrome N"
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).expect("Valid YAML");
    let ctx = capture_invocation_context(Point::new(100, 100));

    let iterations = 10_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = config.find_matching_profile_for_invocation(&ctx);
    }
    let elapsed = start.elapsed();
    let avg_micros = elapsed.as_micros() as f64 / (iterations as f64);

    println!(
        "[PERF] Profile matching avg latency: {:.2}µs over {} evaluations",
        avg_micros, iterations
    );

    assert!(
        avg_micros < 50.0,
        "Profile matching took {:.2}µs, expected < 50µs",
        avg_micros
    );
}
