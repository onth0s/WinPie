use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use winpie::geometry::Sector;

#[derive(Debug, Deserialize)]
struct StateMachineSpec {
    states: Vec<String>,
    transitions: Vec<TransitionSpec>,
}

#[derive(Debug, Deserialize)]
struct TransitionSpec {
    from: String,
    event: String,
    to: String,
    effect: String,
}

#[derive(Debug, Deserialize)]
struct InvariantsSpec {
    invariants: Vec<InvariantEntry>,
}

#[derive(Debug, Deserialize)]
struct InvariantEntry {
    id: String,
    category: String,
    severity: String,
    statement: String,
}

#[derive(Debug, Deserialize)]
struct GeometrySpec {
    geometry: GeometryInnerSpec,
}

#[derive(Debug, Deserialize)]
struct GeometryInnerSpec {
    sectors: usize,
    sector_angle_degrees: f64,
    rotation_degrees: f64,
    deadzone_default: f64,
    radius_default: f64,
    sectors_definition: HashMap<usize, SectorDefSpec>,
}

#[derive(Debug, Deserialize)]
struct SectorDefSpec {
    name: String,
    range: [f64; 2],
}

#[test]
fn test_state_machine_spec_isomorphism() {
    let content = fs::read_to_string("spec/STATE_MACHINE.yaml")
        .expect("spec/STATE_MACHINE.yaml must exist");
    let sm: StateMachineSpec = serde_yaml::from_str(&content)
        .expect("spec/STATE_MACHINE.yaml must deserialize cleanly");

    // All core runtime states must be formally declared in the specification
    let declared_states = &sm.states;
    assert!(declared_states.contains(&"IDLE".to_string()));
    assert!(declared_states.contains(&"ACTIVE".to_string()));
    assert!(declared_states.contains(&"MODAL_MENU".to_string()));
    assert!(declared_states.contains(&"WAIT_RELEASE".to_string()));

    // Verify all transitions have non-empty definitions
    assert!(!sm.transitions.is_empty());
    for t in &sm.transitions {
        assert!(!t.from.is_empty());
        assert!(!t.event.is_empty());
        assert!(!t.to.is_empty());
        assert!(!t.effect.is_empty());
    }
}

#[test]
fn test_invariants_spec_integrity() {
    let content = fs::read_to_string("spec/INVARIANTS.yaml")
        .expect("spec/INVARIANTS.yaml must exist");
    let inv: InvariantsSpec = serde_yaml::from_str(&content)
        .expect("spec/INVARIANTS.yaml must deserialize cleanly");

    assert!(inv.invariants.len() >= 20, "Must have comprehensive invariant catalog");

    let mut ids = std::collections::HashSet::new();
    for entry in &inv.invariants {
        assert!(ids.insert(entry.id.clone()), "Duplicate invariant ID: {}", entry.id);
        assert!(!entry.category.is_empty());
        assert!(!entry.severity.is_empty());
        assert!(!entry.statement.trim().is_empty());
        assert!(
            entry.id.starts_with("INV-"),
            "Invariant ID must follow INV-* format: {}",
            entry.id
        );
    }

    // Verify essential invariants are defined
    assert!(ids.contains("INV-INPUT-001"));
    assert!(ids.contains("INV-INPUT-008"));
    assert!(ids.contains("INV-INPUT-009"));
    assert!(ids.contains("INV-MENU-001"));
    assert!(ids.contains("INV-MENU-002"));
    assert!(ids.contains("INV-GEOMETRY-001"));
}

#[test]
fn test_geometry_spec_isomorphism() {
    let content = fs::read_to_string("spec/GEOMETRY.yaml")
        .expect("spec/GEOMETRY.yaml must exist");
    let geom: GeometrySpec = serde_yaml::from_str(&content)
        .expect("spec/GEOMETRY.yaml must deserialize cleanly");

    assert_eq!(geom.geometry.sectors, 8);
    assert_eq!(geom.geometry.sector_angle_degrees, 45.0);
    assert_eq!(geom.geometry.rotation_degrees, 0.0);
    assert_eq!(geom.geometry.deadzone_default, 32.0);
    assert_eq!(geom.geometry.radius_default, 180.0);

    for (idx, sector) in Sector::ALL.iter().enumerate() {
        let def = geom.geometry.sectors_definition.get(&idx)
            .unwrap_or_else(|| panic!("Sector {} missing in spec", idx));
        assert_eq!(def.name, sector.name(), "Sector {} name mismatch", idx);
        assert_eq!(def.range.len(), 2);
    }
}
