# Phase 0: Native Radial Interaction Primitive

**Document ID:** `01__native_radial_interaction_primitive.md`  
**Roadmap Phase:** Phase 0  
**Status:** Completed & Verified (POC Baseline)  
**Dependencies:** Windows 10/11 OS APIs (`user32.dll`, `gdi32.dll`, `msimg32.dll`), Rust standard library  

---

## 1. Executive Summary & Strategic Intent

Phase 0 establishes the fundamental interaction substrate of WinPie. The primary design objective was to build a zero-compromise, native Windows radial interaction primitive that feels like a natural OS capability rather than a third-party application.

The core challenge in Windows radial interfaces is **interaction transparency**:
1. Activating the interface must not steal focus from the active foreground window.
2. Intercepting user input must not leak stray clicks or key events to background applications upon dismissal.
3. Modifiers (particularly the Windows key) must not trigger native shell interactions (e.g., the Windows Start Menu).
4. Geometry tracking and overlay presentation must remain jitter-free and performant across multi-monitor setups with non-uniform physical coordinates.

Phase 0 successfully proved this substrate, yielding a stable, verified foundation for downstream context-aware and hierarchical capabilities.

---

## 2. High-Level Architectural Model

The Phase 0 architecture couples low-level Win32 hooks to a deterministic finite state machine (FSM) and an unbuffered layered window renderer.

```text
                                Windows OS Subsystem
                                         │
             ┌───────────────────────────┴───────────────────────────┐
             │                                                       │
             ▼                                                       ▼
     WH_KEYBOARD_LL                                             WH_MOUSE_LL
(Win/Esc gesture detection,                                (Cursor tracking,
 Start Menu suppression)                                    LMB/RMB intercept)
             │                                                       │
             └───────────────────────────┬───────────────────────────┘
                                         │ Input Event
                                         ▼
                             ┌───────────────────────┐
                             │    Interaction FSM    │
                             │  IDLE ↔ ACTIVE ↔ WAIT │
                             └───────────┬───────────┘
                                         │
                 ┌───────────────────────┼───────────────────────┐
                 │ State / Hover         │ Dimensions / Angle    │ Action
                 ▼                       ▼                       ▼
      ┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
      │   Layered Overlay   │ │  Radial Geometry    │ │   Process Launcher  │
      │ WS_EX_LAYERED       │ │ 8 Sectors, Deadzone │ │ Spawn Target via    │
      │ WS_EX_NOACTIVATE    │ │ Physical Px Mapping │ │ CreateProcessW      │
      └─────────────────────┘ └─────────────────────┘ └─────────────────────┘
```

---

## 3. Subsystem Specifications

### 3.1. Hook Subsystem & Input Swallowing (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`)
- **Activation Gesture:** Global chord `Win + Esc`.
- **Shell Suppression:** When `Win` is physically held and `Esc` transitions to down, the hook swallows `VK_ESCAPE` by returning non-zero (`1`) from `LowLevelKeyboardProc`. To prevent the Windows shell from interpreting the subsequent `Win` key release as a Start Menu invocation, dummy key strokes (`VK_NONAME` or intercepted keyups) or hook swallowing ensure the shell modifier state machine is safely suppressed.
- **Mouse Hook Transparency:**
  - In `IDLE` state, `WH_MOUSE_LL` immediately delegates to `CallNextHookEx` with zero overhead.
  - In `ACTIVE` state, `WM_MOUSEMOVE` coordinates are intercepted and pushed to the FSM for angle calculation while passing through to the OS to maintain visual cursor fluidity.
  - Mouse click events (`WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_RBUTTONDOWN`, `WM_RBUTTONUP`) are swallowed completely, preventing target background windows from receiving clicks during radial navigation.

### 3.2. Focus-Preserving Layered Overlay (`overlay.rs`)
- **Window Hierarchy:** Unowned popup window initialized with extended styles:
  ```c
  WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_TOOLWINDOW
  ```
- **Focus Neutrality:** The overlay is positioned via `SetWindowPos` with `SWP_NOACTIVATE | SWP_SHOWWINDOW`. Active foreground application window focus (`GetForegroundWindow`) remains entirely undisturbed throughout the interaction lifecycle.
- **Alpha-Blended Double Buffering:** Renders directly to a 32-bit ARGB DIB section, updating the screen atomically via `UpdateLayeredWindow`.

### 3.3. Deterministic 8-Way Radial Geometry (`geometry.rs`)
- **Coordinate Space:** Physical screen pixels directly matching OS desktop coordinates.
- **Center Origin:** Anchored to cursor location $(X_0, Y_0)$ at the exact instant of invocation trigger.
- **Deadzone Radius ($R_{\text{dead}}$):** Default $32\,\text{px}$. Coordinates inside this radius yield `None` (no hover sector).
- **Outer Radius ($R_{\text{max}}$):** Default $180\,\text{px}$. Coordinates beyond this radius map out of bounds.
- **Angular Sectors:** 8 equal directional slices of $45^\circ$ each:
  - North (`N`): $337.5^\circ \to 22.5^\circ$
  - North-East (`NE`): $22.5^\circ \to 67.5^\circ$
  - East (`E`): $67.5^\circ \to 112.5^\circ$
  - South-East (`SE`): $112.5^\circ \to 157.5^\circ$
  - South (`S`): $157.5^\circ \to 202.5^\circ$
  - South-West (`SW`): $202.5^\circ \to 247.5^\circ$
  - West (`W`): $247.5^\circ \to 292.5^\circ$
  - North-West (`NW`): $292.5^\circ \to 337.5^\circ$

---

## 4. Interaction State Machine & Transition Semantics

```text
                  ┌──────────────┐
                  │     IDLE     │◄──────────────────┐
                  └──────┬───────┘                   │
                         │                           │
                   [Win+Esc Down]               [All Keys Up]
                         │                           │
                         ▼                           │
                  ┌──────────────┐                   │
                  │    ACTIVE    │                   │
                  └──────┬───────┘                   │
                         │                           │
         ┌───────────────┼───────────────┐           │
         │               │               │           │
   [Win Up (Valid)] [LMB (Valid)]   [RMB / Out]      │
         │               │               │           │
         ▼               ▼               ▼           │
     (Commit)        (Commit)         (Cancel)       │
         │               │               │           │
         └───────────────┼───────────────┘           │
                         │                           │
                   [Keys Held?]                      │
                   ├── Yes ──────────────────►┌──────────────┐
                   │                          │ WAIT_RELEASE │
                   └── No ───────────────────►└──────────────┘
```

### 4.1. Resolution Semantics
- **Commit:** Occurs if either `Win` is released or `LMB` is pressed while the cursor resides within a valid sector ($R_{\text{dead}} < r \le R_{\text{max}}$). Launches target application command and dismisses overlay.
- **Cancel:** Occurs if `RMB` is clicked anywhere, or `LMB` is clicked inside the deadzone ($r \le R_{\text{dead}}$) or beyond outer bounds ($r > R_{\text{max}}$). Closes overlay with zero side-effects.
- **Re-press Guard (`WAIT_RELEASE`):** If a resolution occurs while physical modifier keys remain held down, the engine enters `WAIT_RELEASE`. It will not accept new activation triggers until all physical keys are released, preventing accidental cyclic re-triggers.

---

## 5. Formal Invariants Implemented

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-INPUT-001` | Input | `Win+Esc` activates the wheel exactly once per qualifying gesture. |
| `INV-INPUT-002` | Input | While `ACTIVE`, mouse button transitions are not forwarded to foreground applications. |
| `INV-INPUT-004` | Input | Resolving mouse clicks are swallowed before application delivery. |
| `INV-INPUT-007` | Input | After resolution, if keys remain held, WinPie enters `WAIT_RELEASE` until disarmed. |
| `INV-STATE-001` | FSM | `ACTIVE` always terminates in `IDLE` (or `WAIT_RELEASE`) via commit, cancel, or fatal error. |
| `INV-GEOMETRY-001`| Geometry | The wheel contains exactly 8 equal directional sectors of $45^\circ$. |
| `INV-GEOMETRY-003`| Geometry | Coordinates inside deadzone or outside radius map to no directional hover sector. |

---

## 6. Edge Cases & Verified Failure Modes

1. **Multi-Monitor Virtual Desktop Coordinates:** Cursor positions on secondary displays can possess negative coordinates $(x < 0, y < 0)$. Sector classification normalizes offsets relative to the invocation origin $(\Delta x = x - x_0, \Delta y = y - y_0)$, preserving perfect mathematical invariance across multi-display arrangements.
2. **Foreground Window Invalidation:** Target window exiting or crashing while WinPie is `ACTIVE` has zero effect on overlay stability.
3. **Ghost Modifiers:** Releasing `Esc` before `Win` maintains `ACTIVE` state cleanly. Releasing `Win` while hovering a sector triggers immediate commit.

---

## 7. Verification Baseline

- **Unit Test Suite:** [`tests/geometry.rs`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/tests/geometry.rs) verifies all boundary conditions, sector angles, negative coordinates, and deadzone transitions.
- **State Machine Verification:** [`tests/interaction.rs`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/tests/interaction.rs) proves exhaustive FSM transitions, re-press guards, and cancellation paths.
- **Physical Proof:** Tested on live Windows 10/11 environments against Explorer, Windows Terminal, Sublime Text, and Chrome.
