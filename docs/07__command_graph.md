# Phase 6: Command Graph

**Document ID:** `07__command_graph.md`  
**Roadmap Phase:** Phase 6  
**Status:** Planned  
**Dependencies:** Phase 0 (`01__native_radial_interaction_primitive.md`), Phase 1 (`02__formalize_interaction_contract.md`), Phase 5 (`06__native_svg_icon_pipeline.md`)  

---

## 1. Executive Summary & Strategic Intent

Phase 6 marks the architectural inflection point where WinPie transforms from a flat, single-tier 8-item wheel into a **recursive, arbitrary-depth command graph**.

Traditional radial tools suffer from rigid structural constraints: menus are hardcoded to "level 1", "level 2", or "submenus" with ad-hoc transition rules. In WinPie, every node in the graph possesses the **exact same conceptual shape**:
- A node may have an executable action.
- A node may have child nodes.
- Both may exist simultaneously (e.g. click to execute default action, flick/gesture further to navigate deeper).

This recursive topology supports arbitrary depth navigation through compound spatial gestures (e.g., `North-East -> North -> West -> Execute`) while maintaining the same physical interaction grammar at every level.

---

## 2. Command Graph Data Model

The command topology is modeled as a directed acyclic graph (DAG) of unified `CommandNode` structures:

```rust
pub type NodeId = String;

pub struct CommandNode {
    pub id: NodeId,
    pub label: String,
    pub icon: Option<String>,
    pub shortcut_key: Option<char>,
    
    /// Optional executable payload (Phase 10)
    pub action: Option<ActionSpec>,
    
    /// Child nodes indexed radially or alphabetically
    pub children: Vec<CommandNode>,
    
    /// Metadata tags used for contextual matching in Phase 7
    pub metadata: NodeMetadata,
}
```

```text
                        ┌───────────────────────────────┐
                        │          CommandNode          │
                        ├───────────────────────────────┤
                        │ id: "edit.transform"          │
                        │ label: "Transform"            │
                        │ icon: "icons/transform.svg"   │
                        │ action: Option<ActionSpec>    │
                        │ children: Vec<CommandNode>    │
                        └───────────────┬───────────────┘
                                        │
                         ┌──────────────┴──────────────┐
                         ▼                             ▼
                 [ action: Some ]              [ children: NonEmpty ]
                 Leaf Execution                Child Surface Expansion
                 (Runs command)                (Displays next radial tier)
```

---

## 3. Spatial Navigation & Gesture Mechanics

### 3.1. Compound Gesture Pathing
A gesture is represented as a sequence of spatial directional tokens:
$$\text{Path} = [D_0, D_1, D_2, \dots, D_k], \quad D_i \in \{\text{N}, \text{NE}, \text{E}, \text{SE}, \text{S}, \text{SW}, \text{W}, \text{NW}\}$$

```text
Gesture Vector: [ NE ──► N ──► E ]
      │
      ├── (NE) Root Sector: "Code / Refactor"
      │     └── (N) Sub-Sector: "Extract"
      │           └── (E) Leaf: "Extract Function" ──► EXECUTE
```

### 3.2. Sub-Surface Transition Physics
When the cursor enters a slice containing child nodes, WinPie offers two distinct transition modes:
1. **Continuous Marking Mode (Gesture Flick):** As the cursor exceeds a threshold distance or lingers in the slice, the center origin smoothly repins to the transition boundary, projecting the next 8-way ring instantly.
2. **Step-Wise Confirmation Mode:** The user clicks LMB or releases a key chord to drill into the sub-menu, centering the new sub-wheel at the current cursor position.

### 3.3. Breadcrumb & Backtracking Semantics
- Returning the cursor to the center deadzone of a child ring steps back one level in the hierarchy.
- Pressing `Esc` or a dedicated backtrack key steps up one parent tier without canceling the entire interaction.
- Clicking `RMB` or moving out-of-bounds cancels the interaction completely, terminating back to `IDLE` per Phase 0/1 invariants.

---

## 4. Declarative Graph Configuration

Command topologies are declared hierarchically in YAML (`config/commands.yaml`):

```yaml
root:
  id: "root"
  children:
    - direction: "N"
      id: "nav"
      label: "Navigate"
      children:
        - direction: "N"
          id: "nav.top"
          label: "Go to Top"
          action: { exec: "cursor_top" }
        - direction: "S"
          id: "nav.bottom"
          label: "Go to Bottom"
          action: { exec: "cursor_bottom" }

    - direction: "NE"
      id: "build"
      label: "Build & Test"
      children:
        - direction: "E"
          id: "build.test"
          label: "Run Tests"
          action: { exec: "cargo test" }
        - direction: "N"
          id: "build.release"
          label: "Build Release"
          action: { exec: "cargo build --release" }

    - direction: "E"
      id: "quick_editor"
      label: "Sublime"
      action: { exec: "sublime.exe" }
```

---

## 5. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-GRAPH-001` | Topology | The command graph must be a strict directed acyclic graph (cycles are rejected during schema validation). |
| `INV-GRAPH-002` | Topology | Any node that does not possess children must possess an executable action (no dead-end leaves). |
| `INV-GRAPH-003` | Navigation | Traversal depth has no theoretical limit; stack depth is bounded only by system memory. |
| `INV-GRAPH-004` | Navigation | Backtracking to a parent node must preserve the parent's contextual state without re-evaluating the graph. |
| `INV-GRAPH-005` | Geometry | Child radial surfaces maintain the identical sector classification rules defined in `GEOMETRY.yaml`. |

---

## 6. Edge Cases & Resilience Strategy

1. **Circular References in Config:** If user config defines recursive references (`A -> B -> A`), the configuration loader detects graph cycles using Tarjan's strongly connected components algorithm at parse time, logging a fatal validation error before startup.
2. **Missing Action and Children:** Nodes defined without actions or children are pruned automatically during graph compilation.
3. **Rapid Direction Oscillations:** Rapid jitter across sector boundaries does not flood the breadcrumb stack; hysteresis deadbands prevent rapid push/pop thrashing.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Deep Tree Navigation Test:** Construct a 5-level deep command hierarchy; simulate synthetic gesture sequence `[N, NE, S, W, E]`; assert execution fires on leaf node with exact payload.
2. [ ] **Cycle Detection Test:** Feed cyclic YAML configuration to the parser; assert clean rejection with descriptive line error.
3. [ ] **Backtracking Invariant Test:** Navigate 3 levels deep; simulate deadzone return gesture; verify system returns cleanly to parent level 2 without UI artifacting.
