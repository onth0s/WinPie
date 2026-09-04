# Phase 11: Unified Contextual Command Surface

**Document ID:** `12__unified_contextual_command_surface.md`  
**Roadmap Phase:** Phase 11  
**Status:** Planned  
**Dependencies:** All Prior Phases (Phases 0 through 10)  

---

## 1. Executive Summary & Strategic Intent

Phase 11 represents the ultimate architectural convergence of WinPie. All preceding subsystems—input interception, environmental introspection, capability hydration, SVG icon rasterization, hierarchical graphs, and execution backends—merge into a cohesive, decoupled whole.

At this phase, WinPie transcends being merely a "pie menu program." It becomes a **Unified Contextual Capability Engine**, where the radial interface is simply **one view** projected over that engine.

### The Four Pillars of WinPie
> - **Context supplies Meaning** (What is the user doing?)  
> - **Geometry supplies Syntax** (How does the user express intent?)  
> - **Command Graph supplies Structure** (What actions are possible?)  
> - **Executor supplies Action** (How is the work carried out?)  

---

## 2. Global Architectural Synthesis

```text
                                  ┌───────────────────┐
                                  │    INPUT LAYER    │
                                  │ (Gesture / Chord) │
                                  └─────────┬─────────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │ INVOCATION CONTEXT│
                                  │(Atomic OS Snapshot│
                                  └─────────┬─────────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │CAPABILITY PROVIDER│
                                  │(Hydrated Semantics│
                                  └─────────┬─────────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │   COMMAND GRAPH   │
                                  │(Master Capabilities│
                                  └─────────┬─────────┘
                                            │
                                            ▼
                                  ┌───────────────────┐
                                  │ RESOLUTION ENGINE │
                                  │ (Context Matching)│
                                  └─────────┬─────────┘
                                            │
                     ┌──────────────────────┴──────────────────────┐
                     │                                             │
                     ▼                                             ▼
          ┌─────────────────────┐                       ┌─────────────────────┐
          │     RADIAL VIEW     │                       │     HEADLESS /      │
          │ (Layered Window UI) │                       │     HOTKEY VIEW     │
          └──────────┬──────────┘                       └──────────┬──────────┘
                     │                                             │
                     └──────────────────────┬──────────────────────┘
                                            │ Selected Action
                                            ▼
                                  ┌───────────────────┐
                                  │ EXECUTOR BACKENDS │
                                  │ (Native / PS / Py)│
                                  └───────────────────┘
```

---

## 3. Decoupling the View from the Engine

Because the pipeline is cleanly partitioned, WinPie supports multiple concurrent presentations of the resolved capability graph:

1. **Radial Overlay View (Default):** The native 8-way layered radial wheel rendered directly on screen via Win32 DIB blitting.
2. **Keyboard Chord HUD View:** A compact, non-intrusive bottom-screen status HUD displaying active left-hand keybindings and mnemonic letters.
3. **Headless / Blind Mode:** For expert users, executing compound marking gestures or known hotkey sequences bypasses overlay window creation entirely, executing commands with sub-millisecond latency.
4. **CLI / IPC Inspection View:** An external query tool (e.g. `winpie.exe query --context active`) that outputs the currently resolved command graph as structured JSON for testing, debugging, or external automation.

---

## 4. End-to-End Execution Trace

To illustrate the complete synthesis in action, consider a user in Blender with a 3D model path on the clipboard:

```text
[T = 0.00ms] User presses Win+Esc. WH_KEYBOARD_LL swallows trigger. FSM -> ACTIVE.
[T = 0.40ms] Context Detection captures HWND, PID, process ("blender.exe"), Monitor (144 DPI).
[T = 1.20ms] Context Hydration queries clipboard; identifies CF_HDROP containing "scene.blend".
[T = 1.80ms] Context Appearance matches blender.yaml -> Accent: #E87D0D, Icon: "blender.svg".
[T = 2.40ms] Command Graph evaluates predicates:
             - Matches "blender.import_clipboard_mesh" -> Assigned to North slice.
             - Matches "blender.render_viewport"      -> Assigned to East slice.
[T = 3.50ms] Overlay double-buffer draws radial sectors with Blender theme and pre-rasterized SVGs.
[T = 4.00ms] UpdateLayeredWindow blits overlay to physical display. Cursor at (0, 0).
[T = 85.0ms] User flicks mouse North (angle = 2.4°, distance = 110px). North sector highlighted.
[T = 110ms]  User releases Win key.
[T = 110.2ms] FSM commits North. Overlay dismissed. WinPie returns to IDLE.
[T = 111.0ms] Executor Manager builds environment:
             WINPIE_TARGET_PROCESS = "blender.exe"
             WINPIE_SELECTED_PATHS = "C:\Assets\scene.blend"
[T = 111.5ms] Spawns python.exe with script "scripts/blender/import_mesh.py" (detached).
[T = 112.0ms] Displays subtle corner toast: "Imported scene.blend (N)". UI loop fully idle.
```

---

## 5. Formal System Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-SYS-001` | System | End-to-end latency from trigger qualification to overlay presentation must remain $\le 8.0\,\text{ms}$. |
| `INV-SYS-002` | System | Failure in any single subsystem (e.g. SVG rasterizer, context provider, executor) must isolate cleanly. |
| `INV-SYS-003` | Architecture| The Command Graph and Resolution Engine must remain completely agnostic of whether input was mouse or keyboard. |
| `INV-SYS-004` | Architecture| The interaction loop must never allocate unbounded memory or leak OS handles across invocation cycles. |

---

## 6. Verification & Acceptance Criteria

1. [ ] **Full-Loop Automated Test:** Trigger WinPie via synthetic Win32 injection; resolve contextual graph; commit sector; verify mock action executes with complete environment variables in $< 10\,\text{ms}$.
2. [ ] **Headless Query Test:** Run `winpie.exe --inspect-context`; verify output is valid JSON matching current foreground application capabilities.
3. [ ] **Multi-Context Switching Stress Test:** Rapidly alternate focus between 5 different applications while invoking WinPie; assert visual theme, resolved commands, and launched actions match the focused target 100% of the time.
