# WinPie Architectural Roadmap — Master Specification Index

This index organizes the multi-phase engineering roadmap for **WinPie**, a high-performance, low-latency, context-aware radial interaction and command execution system for Windows 10/11.

Each phase is specified in its own sequential specification document containing high-level architectural models, component boundaries, formal invariants, edge cases, failure domains, and acceptance criteria.

---

## 1. Document Registry & Phase Mapping

| Doc ID | Roadmap Phase | Title | Focus & Strategic Mission | Status |
| :--- | :--- | :--- | :--- | :--- |
| [`00__index.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/00__index.md) | **Index** | **Master Specification Index** | Roadmap topology, phase mapping, dependency graph, and document governance. | **Active** |
| [`01__native_radial_interaction_primitive.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/01__native_radial_interaction_primitive.md) | **Phase 0** | **Native Radial Interaction Primitive** | Low-level OS hooks, zero-leak input swallowing, focus-preserving layered overlay, 8-way radial geometry, commit/cancel FSM. | **Completed (POC)** |
| [`02__formalize_interaction_contract.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/02__formalize_interaction_contract.md) | **Phase 1** | **Formalize Interaction Contract** | Complete specification-implementation isomorphism via YAML schemas, deterministic state machines, and quantifiable benchmark gates. | **Ready for Execution** |
| [`03__context_detection.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/03__context_detection.md) | **Phase 2** | **Context Detection** | Immutable invocation snapshot capture: foreground HWND, process image, monitor physical space, cursor position, and clipboard signatures. | **Planned** |
| [`04__context_hydration_and_capability_providers.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/04__context_hydration_and_capability_providers.md) | **Phase 3** | **Context Hydration & Capability Providers** | Extensible domain providers (Explorer, Terminal, VS Code, Blender, Clipboard) producing strongly-typed semantic capability models with isolated timeouts. | **Planned** |
| [`05__context_aware_visual_identity.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/05__context_aware_visual_identity.md) | **Phase 4** | **Context-Aware Visual Identity** | Semantic visual state: dynamic accent palettes, application badges, and typography adapting instantly to the detected foreground target. | **Planned** |
| [`06__native_svg_icon_pipeline.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/06__native_svg_icon_pipeline.md) | **Phase 5** | **Native SVG Icon Pipeline** | Pure Rust SVG asset ingestion, pre-rasterization, multi-DPI caching, and zero-allocation blitting into Win32 layered surfaces. | **Planned** |
| [`07__command_graph.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/07__command_graph.md) | **Phase 6** | **Command Graph** | Arbitrary-depth recursive command hierarchy replacing flat 8-item menus with unified executable/branching node topologies. | **Planned** |
| [`08__contextual_command_resolution.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/08__contextual_command_resolution.md) | **Phase 7** | **Contextual Command Resolution** | Declarative requirement predicates matching invocation capabilities to dynamic radial layouts; identical gestures yield context-specific actions. | **Planned** |
| [`09__massive_command_topology.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/09__massive_command_topology.md) | **Phase 8** | **Massive Command Topology** | Spatial mnemonics scaling to 160+ operations per application context via 8 primary directional families and rapid nested gestures. | **Planned** |
| [`10__left_hand_shortcut_chord_layer.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/10__left_hand_shortcut_chord_layer.md) | **Phase 9** | **Left-Hand Shortcut / Chord Layer** | Multi-modal convergence: keyboard chord maps resolving to the same underlying command graph nodes as spatial mouse gestures. | **Planned** |
| [`11__scriptable_execution_backends.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/11__scriptable_execution_backends.md) | **Phase 10** | **Scriptable Execution Backends** | Hardened, decoupled execution engine supporting native binaries, PowerShell scripts, and Python workers with rich contextual payloads. | **Planned** |
| [`12__unified_contextual_command_surface.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/12__unified_contextual_command_surface.md) | **Phase 11** | **Unified Contextual Command Surface** | Complete architectural synthesis: Context (Meaning) + Geometry (Syntax) + Graph (Structure) + Executor (Action). | **Planned** |
| [`13__hardening_and_performance.md`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/docs/13__hardening_and_performance.md) | **Phase 12** | **Hardening & Performance** | Long-running daemon stability: hook watchdog recovery, COM isolation, latency validation (<8ms invocation), and stress engineering. | **Planned** |

---

## 2. Global Architecture & Pipeline Flow

WinPie unifies input modalities, OS introspection, hierarchical graphs, and execution runtimes into a strictly unidirectional processing pipeline:

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                   INPUT INVOCATION                                      │
│               Keyboard Gesture (Win+Esc)  /  Left-Hand Chord  /  Mouse Macro            │
└───────────────────────────────────────────┬─────────────────────────────────────────────┘
                                            │
                                            ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              PHASE 2: CONTEXT DETECTION                                 │
│   Immutable Snapshot: HWND, Process Image, Monitor (DPI/Bounds), Cursor, Clipboard Sig  │
└───────────────────────────────────────────┬─────────────────────────────────────────────┘
                                            │
                                            ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                    PHASE 3: CONTEXT HYDRATION & CAPABILITY PROVIDERS                    │
│   Independent Async Providers: Explorer Selection, Terminal CWD, IDE Buffer, Doc State  │
└───────────────────────────────────────────┬─────────────────────────────────────────────┘
                                            │
                                            ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                    PHASE 7: CONTEXTUAL COMMAND RESOLUTION ENGINE                        │
│   Match Capabilities against Declarative Predicates ──► Resolve Subgraph for Context    │
└─────────────────────┬─────────────────────────────────────────────────────┬─────────────┘
                      │                                                     │
                      ▼                                                     ▼
┌───────────────────────────────────────────────┐   ┌─────────────────────────────────────┐
│       PHASES 4 & 5: PRESENTATION LAYER        │   │       PHASE 6 & 8: TOPOLOGY         │
│   Semantic Accent & App Branding              │   │   8 Directional Primary Families    │
│   Pre-rasterized Native SVG Glyphs            │   │   Arbitrary-Depth Spatial Navigation│
│   Layered Focus-Preserving Layered Window     │   │   Left-Hand Keyboard Chords (Ph. 9) │
└─────────────────────┬─────────────────────────┘   └───────────────────────┬─────────────┘
                      │                                                     │
                      └───────────────────────┬─────────────────────────────┘
                                              │
                                              ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                    PHASE 10: SCRIPTABLE EXECUTION DISPATCH LAYER                        │
│   Decoupled Action Runners: Native Process Spawn / PowerShell Script / Python Worker    │
│   Context Environment Variables & Structured Payloads Passed to Execution Target        │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Dependency & Precedence Topology

Phases must be implemented in a structured progression. While some phases (such as the SVG pipeline and Context Hydration) can be developed concurrently, they merge at key architectural integration milestones:

```text
Phase 0: Native Primitive (Done)
   │
   ▼
Phase 1: Formalize Contract
   │
   ▼
Phase 2: Context Detection
   ├──► Phase 3: Hydration & Providers ──┐
   │                                     ▼
   │                                Phase 7: Contextual Resolution
   │                                     │
   ├──► Phase 4: Visual Identity ◄───────┤
   │         ▲                           │
   │         │                           ▼
   └──► Phase 5: SVG Pipeline       Phase 6: Command Graph
                                         │
                                         ├──► Phase 8: Massive Topology
                                         ├──► Phase 9: Left-Hand Chords
                                         │
                                         ▼
                                    Phase 10: Scriptable Backends
                                         │
                                         ▼
                                    Phase 11: Unified Command Surface
                                         │
                                         ▼
                                    Phase 12: Hardening & Performance
```

---

## 4. Architectural Tenets & Invariants

All phases must adhere to these governing engineering principles:

1. **System Primitive, Not An Application**: WinPie acts as an extension of the Windows shell. It must never steal window focus, disrupt active OS foreground state, or leak intercepted input buttons.
2. **Deterministic Context Isolation**: Invocation context is captured **exactly once** at the trigger timestamp. Command resolution, geometry navigation, and execution dispatch operate strictly against this immutable snapshot.
3. **Bounded Latency in Hot Paths**:
   - Keyboard & mouse hook handlers must complete in $<0.5\,\text{ms}$.
   - Overlay presentation from trigger down-event must display in $<8\,\text{ms}$ (1 frame at 120Hz).
   - No filesystem I/O, COM waits, or memory reallocations inside hook callbacks.
4. **Independent Failure Domains**: A failure in an application-specific context provider (e.g. Photoshop or Blender COM automation hanging) must never abort radial invocation or degrade system responsiveness.
5. **Multi-Modal Symmetry**: Spatial radial mouse gestures and left-hand keyboard chording are distinct syntactic views over the exact same underlying Command Graph.
