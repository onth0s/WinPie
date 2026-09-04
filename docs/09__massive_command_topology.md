# Phase 8: Massive Command Topology

**Document ID:** `09__massive_command_topology.md`  
**Roadmap Phase:** Phase 8  
**Status:** Planned  
**Dependencies:** Phase 6 (`07__command_graph.md`), Phase 7 (`08__contextual_command_resolution.md`), Phase 0 Radial Geometry  

---

## 1. Executive Summary & Strategic Intent

Phase 8 scales WinPie from a modest quick-launcher to an operating-system-level spatial control plane capable of organizing hundreds of domain actions without cognitive collapse.

Modern professional software (Blender, Unreal Engine, VS Code, Houdini) possesses thousands of distinct operations. Users typically memorize dozens of awkward, non-ergonomic modifier combinations (`Ctrl+Alt+Shift+F9`) or repeatedly search textual command palettes (`Ctrl+Shift+P`).

WinPie exploits **2D spatial memory**:
- 8 primary directional families act as high-level functional categories.
- Each direction branches into 8 to 20+ specialized contextual operations.
- This yields **$8 \times 20+ \approx 160+$ distinct operations available per context**, accessible via rapid, fluid spatial gestures without requiring a flat, overwhelming list of shortcuts.

> **The spatial hierarchy itself becomes the mnemonic.**

---

## 2. Spatial Directional Taxonomies

To build universal muscle memory across varied applications, WinPie establishes standardized directional semantics:

```text
                                [ NORTH ]
                            Navigation & View
                       (Jump, Focus, Zoom, Camera)
                                   │
              [ NORTH-WEST ]       │       [ NORTH-EAST ]
             History & State       │       Build & Run
          (Undo, Redo, Snapshot)   │     (Compile, Play, Deploy)
                       \           │           /
                        \          │          /
     [ WEST ]            \         │         /            [ EAST ]
   Edit & Modify          ─────────┼─────────          File & Assets
(Select, Duplicate, Cut)           │                 (Save, Export, Import)
                        /          │          \
                       /           │           \
              [ SOUTH-WEST ]       │       [ SOUTH-EAST ]
             Window & Layout       │      Terminal & Shell
          (Split, Float, Tile)     │      (CLI, Logs, Monitor)
                                   │
                                [ SOUTH ]
                            Tools & Transform
                       (Scale, Rotate, Extrude, Refactor)
```

---

## 3. Scale Topology & Gesture Progression

```text
Level 0: Context
└── 8 Primary Directions (Cardinal & Intercardinal)
    ├── North (Navigation)      ──► 20+ actions (Top, Bottom, Matching Brace, Next Issue...)
    ├── North-East (Build/Run)  ──► 20+ actions (Debug, Profile, Release, Run Selected...)
    ├── East (File/Assets)      ──► 20+ actions (Import Asset, Export FBX, Save Version...)
    ├── South-East (Terminal)   ──► 20+ actions (New Tab, Split Terminal, Git CLI...)
    ├── South (Transform/Tools) ──► 20+ actions (Extrude, Bevel, Subdivide, Knife...)
    ├── South-West (Layout)     ──► 20+ actions (Tile Left, Fullscreen, Zen Mode...)
    ├── West (Edit/Modify)      ──► 20+ actions (Select All, Multi-Cursor, Reformat...)
    └── North-West (History)    ──► 20+ actions (Stash, Revert, Git Blame, Local Diff...)
```

### 3.1. Novice to Expert Motor Learning Curve
1. **Novice (Visual Guidance):** User invokes `Win+Esc`, reads the radial slices, hovers East ("File"), reads the sub-ring, and clicks "Export FBX".
2. **Intermediate (Marking Gesture):** User knows the direction and quickly flicks East $\to$ South without waiting for the visual overlay to settle.
3. **Expert (Blind Compound Muscle Memory):** User executes a fluid compound stroke ($\approx 120\,\text{ms}$) that resolves immediately upon release, bypassing conscious visual search completely.

---

## 4. Radial Density & Sub-Layout Architecture

When expanding a primary direction containing 20+ operations, WinPie employs two complementary layout techniques to maintain high angular target size:

### 4.1. Concentric Radial Rings (Inner / Outer Rings)
- Rather than squeezing 20 slices into a single crowded $18^\circ$ wheel, WinPie layers them into concentric rings:
  - **Inner Ring ($r = 80\,\text{px} \dots 160\,\text{px}$):** 8 high-frequency primary sub-actions.
  - **Outer Ring ($r = 160\,\text{px} \dots 240\,\text{px}$):** 12 secondary/extended sub-actions.
- Angular resolution remains broad and fault-tolerant ($\ge 30^\circ$ per target).

### 4.2. Directional Wedge Cascades
- Selecting a primary direction projects a targeted $90^\circ$ fan (wedge) outward in that direction rather than occupying the entire $360^\circ$ circle, leaving the remainder of the screen unoccluded.

---

## 5. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-TOPO-001` | Topology | Primary directional semantics (e.g. North for Nav, East for Files) must remain consistent across all application profiles. |
| `INV-TOPO-002` | Geometry | Minimum angular slice width in any active layout must not fall below $15^\circ$ to prevent mis-flicks. |
| `INV-TOPO-003` | Performance | Scaling the command catalog to $2,000+$ total commands must not increase resolution latency by more than $0.2\,\text{ms}$. |
| `INV-TOPO-004` | Memory | Command tree indexing must use dense contiguous vectors or string interning to limit total memory footprint to $\le 10\,\text{MB}$. |

---

## 6. Edge Cases & Resilience Strategy

1. **Angular Drift on Fast Gestures:** Fast hand flicks often exhibit an angular curvature (e.g., pulling slightly downward when moving right). WinPie's directional classifier employs an inertia-weighted vector model rather than instantaneous point sampling to correctly detect intended sector trajectories.
2. **Screen Boundary Collision for Outer Rings:** If a multi-ring menu with radius $280\,\text{px}$ is opened near the edge of a display, the entire cluster shifts dynamically inwards (elastic repulsion) so no slices are clipped outside the monitor work area.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Capacity Benchmark:** Load a synthetic catalog of 1,500 commands across 10 application profiles; assert startup parse time $< 15\,\text{ms}$ and memory footprint $< 8\,\text{MB}$.
2. [ ] **Gesture Stroke Recognition Test:** Feed captured human mouse trajectories into the gesture classifier; verify $> 99\%$ classification accuracy matching intended multi-tier nodes.
3. [ ] **Elastic Screen Clamping Test:** Simulate invocation at $(0, 0)$ and $(1920, 1080)$; assert all 20+ sub-slices fall completely within valid screen bounds.
