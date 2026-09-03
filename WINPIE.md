# WinPie — POC Implementation Specification

**Status:** Implementation-ready
**Platform:** Windows 10/11 x64
**Language:** Rust-first
**Configuration/specification:** YAML
**Python:** Optional external helper only; never in the interaction hot path

---

## 1. Purpose

WinPie is a lightweight radial interaction primitive activated by a keyboard gesture:

```text
Win + Esc
    │
    ▼
┌──────────┐
│  ACTIVE  │
└────┬─────┘
     │
     ├── cursor movement ──► hover sector
     │
     ├── release Win ──────► commit (if in valid sector) or cancel (if out of bounds)
     │
     ├── left click ───────► commit (if in valid sector) or cancel (if out of bounds)
     │
     └── right click ──────► cancel
                              │
                              ▼
                         ┌─────────┐
                         │  IDLE   │
                         └─────────┘
```

The POC is successful when it can be invoked from an ordinary foreground Windows application, display a radial menu anchored to the current cursor position, track one of eight directions, commit via Win-key release or left-click within valid sector bounds, cancel on right-click or out-of-bounds click, and restore the system to its prior interaction state without focus theft, stuck modifiers, or leaked mouse-button input.

---

# 2. Non-Goals

The POC does **not** implement:

* application actions;
* persistent menu definitions;
* nested pie menus;
* animation;
* keyboard navigation;
* accessibility UI;
* DPI-independent visual scaling;
* Python integration in the interaction path;
* plugin architecture;
* configuration hot-reload;
* sophisticated rendering effects.

The POC proves the **interaction primitive**, not the eventual action system.

---

# 3. Architecture

```text
                    ┌──────────────────────────┐
                    │       Windows OS         │
                    └────────────┬─────────────┘
                                 │
                 ┌───────────────┴────────────────┐
                 │                                │
          WH_KEYBOARD_LL                    WH_MOUSE_LL
                 │                                │
                 ▼                                ▼
        ┌────────────────┐              ┌────────────────┐
        │ Keyboard Input │              │  Mouse Input   │
        └───────┬────────┘              └───────┬────────┘
                │                               │
                └──────────────┬────────────────┘
                               ▼
                    ┌────────────────────┐
                    │   Interaction FSM  │
                    │                    │
                    │ IDLE ↔ ACTIVE      │
                    └─────────┬──────────┘
                              │
             ┌────────────────┼────────────────┐
             ▼                ▼                ▼
        ┌──────────┐    ┌──────────┐    ┌────────────┐
        │ Geometry │    │  Overlay │    │ Diagnostics│
        └──────────┘    └──────────┘    └────────────┘
             │                │
             └───────┬────────┘
                     ▼
              ┌─────────────┐
              │   Renderer  │
              └─────────────┘
```

There is **one GUI/message-pump thread**.

The low-level hooks are deliberately thin. They observe/intercept input and hand the resulting event to the interaction machinery. They must not perform rendering, logging, allocation-heavy work, or action execution.

---

# 4. Repository Layout

```text
winpie/
├── Cargo.toml
├── config/
│   └── default.yaml
├── spec/
│   ├── INVARIANTS.yaml
│   ├── STATE_MACHINE.yaml
│   └── GEOMETRY.yaml
├── src/
│   ├── main.rs
│   ├── app.rs
│   ├── input/
│   │   ├── mod.rs
│   │   ├── keyboard.rs
│   │   └── mouse.rs
│   ├── interaction/
│   │   ├── mod.rs
│   │   └── state.rs
│   ├── geometry/
│   │   └── mod.rs
│   ├── overlay/
│   │   ├── mod.rs
│   │   ├── window.rs
│   │   └── render.rs
│   └── diagnostics.rs
└── tests/
    ├── geometry.rs
    └── interaction.rs
```

---

# 5. Runtime Configuration

`config/default.yaml`

```yaml
activation:
  modifier: win
  trigger: escape

  # Qualifying Escape DOWN is consumed.
  swallow_trigger: true

  # Which Windows modifier(s) qualify.
  left_win: true
  right_win: true

overlay:
  radius: 180
  deadzone: 32

  # Radius/deadzone are physical screen pixels.
  coordinate_space: physical_screen_pixels

  window:
    popup: true
    toolwindow: true
    topmost: true
    no_activate: true
    layered: true
    transparent_input: true

wheel:
  slices: 8
  rotation_degrees: 0.0

rendering:
  show_stubs: true
  show_deadzone: true
  highlight_hovered: true

diagnostics:
  enabled: true
  log_selection: true
  log_cancel: true
  log_fatal_errors: true
```

---

# 6. Coordinate Semantics

All geometry uses **physical screen coordinates**.

The implementation must use a DPI-awareness mode that makes the cursor coordinates returned to the geometry layer and the overlay placement coordinates refer to the same physical coordinate space.

For the POC:

```text
screen origin = Windows virtual-screen origin
units        = physical pixels
```

Negative coordinates are valid.

This explicitly supports multi-monitor layouts such as:

```text
             Monitor A
        (-1920,0) ┌──────────┐
                  │          │
                  └──────────┘
                              ┌──────────────┐
                              │  Monitor B   │
                              │   (0,0)      │
                              └──────────────┘
```

No conversion between logical pixels and physical pixels may occur between:

* cursor acquisition;
* geometry calculations;
* overlay positioning.

---

# 7. Interaction State Machine

`spec/STATE_MACHINE.yaml`

```yaml
states:
  - IDLE
  - ACTIVE

transitions:

  - from: IDLE
    event: WIN_ESC_DOWN
    to: ACTIVE
    effect: ACTIVATE

  - from: ACTIVE
    event: LBUTTON_DOWN
    to: IDLE
    effect: COMMIT_IF_VALID_ELSE_CANCEL

  - from: ACTIVE
    event: WIN_UP
    to: IDLE
    effect: COMMIT_IF_VALID_ELSE_CANCEL

  - from: ACTIVE
    event: RBUTTON_DOWN
    to: IDLE
    effect: CANCEL

  - from: ACTIVE
    event: FATAL_ERROR
    to: IDLE
    effect: CANCEL
```

### Semantic resolution rules

- **Esc UP** is a no-op while ACTIVE (releasing Esc does not dismiss the menu while Win remains held).
- **Win UP** resolves the wheel: commits the hovered sector if within valid slice bounds ($\text{deadzone} < r \le \text{radius}$), or cancels if in deadzone/out-of-bounds.
- **Left-Click** resolves the wheel: commits the hovered sector if within valid slice bounds, or cancels if in deadzone/out-of-bounds.
- **Right-Click** unconditionally cancels anywhere.
- **Re-press Requirement**: When cancelled or committed while activation keys remain held, WinPie silently waits until keys are released before allowing another activation.

---

# 8. Activation

Activation sequence:

```text
IDLE
 │
 │ LWin/RWin DOWN
 │
 │ Escape DOWN
 ▼
ACTIVE
```

The activation gesture is:

```text
Win + Esc
```

The qualifying Escape `DOWN` event is swallowed.

The implementation must prevent the activation gesture from producing unintended Windows-shell behavior such as accidentally opening Start.

Therefore the implementation must explicitly establish, through the actual input handling and acceptance tests, that:

> Completing the Win+Esc activation gesture does not cause Start, a shell shortcut, or another unintended foreground action.

The exact minimum set of Win-key events swallowed to achieve that behavior is an implementation detail, but the observable behavior is not.

---

# 9. Windows Modifier Tracking

The input layer maintains:

```rust
struct ModifierState {
    left_win_down: bool,
    right_win_down: bool,
}
```

with:

```text
win_held =
    left_win_down || right_win_down
```

The implementation must distinguish:

* LWin DOWN;
* LWin UP;
* RWin DOWN;
* RWin UP;
* Escape DOWN;
* Escape UP.

Keyboard auto-repeat must not repeatedly activate the wheel.

Activation is generated only from the qualifying transition:

```text
win_held == true
AND
Escape DOWN
```

while currently `IDLE`.

---

# 10. Keyboard Hook

Use:

```text
WH_KEYBOARD_LL
```

The hook callback must be minimal.

Conceptually:

```rust
fn keyboard_hook(event: KeyboardEvent) -> HookResult
```

Responsibilities:

1. identify relevant key transition;
2. update modifier state;
3. recognize activation;
4. consume only events required by the interaction contract;
5. return immediately.

The callback must **not**:

* render;
* allocate unnecessarily;
* execute actions;
* perform blocking I/O;
* perform expensive logging;
* perform arbitrary application logic.

State transitions and window-management work should occur outside the hook callback where practical.

---

# 11. Mouse Capture Model

While `ACTIVE`, WinPie observes global mouse input using:

```text
WH_MOUSE_LL
```

The mouse hook handles:

```text
WM_MOUSEMOVE
WM_LBUTTONDOWN
WM_RBUTTONDOWN
```

The crucial distinction is:

> WinPie observes mouse movement globally but consumes only the button transitions belonging to the pie-menu interaction.

Therefore:

```text
Mouse movement
      │
      ├── WinPie observes ──► hover geometry
      │
      └── normal Windows processing continues

Left/right button DOWN
      │
      └── WinPie consumes while ACTIVE
```

---

# 12. Mouse Consumption Ordering

This is an explicit correctness requirement.

For a resolving button event:

```text
ACTIVE + LBUTTON_DOWN
```

the event must be consumed **before the foreground application can receive it**.

Likewise:

```text
ACTIVE + RBUTTON_DOWN
```

must be consumed before delivery to the foreground application.

The sequence is therefore logically:

```text
physical button event
        │
        ▼
 WH_MOUSE_LL
        │
        ├── determine ACTIVE
        │
        ├── determine LBUTTON/RBUTTON
        │
        ├── resolve interaction
        │
        ├── transition to IDLE
        │
        └── return non-zero
                │
                ▼
        event is suppressed
```

There is **no synthetic re-emission** of the resolving button event after returning to `IDLE`.

### Required invariants

```yaml
- id: INV-INPUT-004
  category: input
  severity: critical
  statement: >
    While ACTIVE, the left-button-down event that resolves the interaction
    is swallowed before it can reach the foreground application.

- id: INV-INPUT-005
  category: input
  severity: critical
  statement: >
    Returning to IDLE as a consequence of a resolving mouse event does not
    re-emit or otherwise forward that event.

- id: INV-INPUT-006
  category: input
  severity: critical
  statement: >
    Once ACTIVE has been entered, there is no input-processing gap during
    which a resolving left/right button-down event can escape to the
    foreground application.
```

The implementation may keep the mouse hook installed for the lifetime of the process or dynamically install/remove it around ACTIVE, but **the observable invariant above must hold**.

---

# 13. Mouse Movement

Mouse movement does not terminate the wheel.

While ACTIVE:

```text
WM_MOUSEMOVE
      │
      ▼
cursor position
      │
      ▼
geometry evaluation
      │
      ▼
hover sector
      │
      ▼
redraw only if hover changed
```

No continuous rendering loop is required.

The implementation should use mouse movement events rather than polling the cursor at a high-frequency timer.

---

# 14. Overlay Window

The wheel is rendered by a native Win32 popup window.

Required window characteristics:

```text
WS_POPUP
WS_EX_TOOLWINDOW
WS_EX_TOPMOST
WS_EX_NOACTIVATE
WS_EX_LAYERED
```

The overlay is positioned with its center at the cursor position captured at activation.

```text
anchor = cursor_position_at_activation
```

The anchor remains fixed for the lifetime of the interaction.

---

# 15. Visual Transparency vs Input Transparency

These are explicitly separate requirements.

### Visual transparency

The overlay must visually contain only the radial UI.

Pixels outside the rendered wheel must remain transparent.

### Input transparency

The overlay must **not become the effective mouse target for pointer movement**.

`WS_EX_TRANSPARENT` is **not** accepted as a specification-level definition of click-through behavior.

The implementation must establish actual hit-testing behavior.

Required observable property:

```yaml
- id: INV-OVERLAY-003
  category: overlay
  severity: critical
  statement: >
    The overlay must not become the effective mouse target for pointer
    movement. Mouse movement must continue to behave as normal foreground
    application mouse movement while WinPie observes the movement globally.
```

The exact implementation may use appropriate Win32 hit-testing behavior, but the POC must test the resulting behavior rather than merely checking for a particular window style.

---

# 16. Overlay Lifetime

Activation:

```text
IDLE
  │
  ├── capture cursor
  ├── create/show overlay
  ├── initialize hover = NONE
  └── ACTIVE
```

Resolution/cancellation:

```text
ACTIVE
  │
  ├── resolve/cancel
  ├── hide overlay
  ├── restore IDLE
  └── reset transient state
```

The overlay must not activate or steal foreground focus.

---

# 17. Geometry

The wheel contains exactly eight directional sectors.

```text
        N
        │
   NW   │   NE
        │
W ──────●────── E
        │
   SW   │   SE
        │
        S
```

Each sector spans:

```text
360° / 8 = 45°
```

The geometry coordinate system is:

```text
0°   = North
90°  = East
180° = South
270° = West
```

Angles increase clockwise.

---

# 18. Rotation

`rotation_degrees` rotates the sector boundaries.

Let:

```text
θ = normalized_clockwise_angle(cursor - center)
r = distance(cursor, center)
φ = rotation_degrees
```

The effective classification angle is:

```text
θ' = normalize(θ - φ)
```

The sector is then selected from `θ'`.

With:

```yaml
rotation_degrees: 0
```

North is the centerline of the north sector.

The rotation parameter must therefore affect **actual geometry**, not merely exist in configuration.

---

# 19. Deadzone

The deadzone is an inner circular region.

```text
             outer radius
          ┌───────────────┐
        /                   \
       /       sector        \
      |        ╲   ╱         |
      |         ╲ ╱          |
      |          ●           |
      |       deadzone       |
       \                     /
        \                   /
          └───────────────┘
```

Let:

```text
r² = dx² + dy²
```

A hover evaluation is:

```text
r² <= deadzone²
    → NONE

r² > deadzone²
    → directional sector
```

Squared distance should be used where practical to avoid unnecessary square roots.

---

# 20. Hover vs Commit

These are deliberately separate concepts.

## Hover

Hover is purely visual/preselection state.

```rust
hover_selection: Option<Sector>
```

Inside the deadzone:

```text
hover_selection = None
```

Moving into a sector:

```text
hover_selection = Some(sector)
```

Changing hover selection does **not** terminate ACTIVE.

---

## Commit & Unified Cancel

Resolution occurs within the valid sector ring:

```text
r <= deadzone
    → CANCEL

r > radius
    → CANCEL

deadzone < r <= radius
    → resolve by angle
    → Commit(sector)
```

Therefore:

```text
hover:
    inside deadzone or outside radius → NONE
    inside sector ring                → Some(sector)

commit (Left-Click or Win-Up):
    inside deadzone or outside radius → CANCEL
    inside sector ring                → Commit(sector)
```

Clicking or releasing outside valid slice bounds unifies with right-click cancellation.

---

# 21. Right-Click

Right-click is unconditional cancellation.

```text
ACTIVE + RBUTTON_DOWN
    → CANCEL
    → IDLE
```

This is true:

* inside deadzone;
* inside visual radius;
* outside visual radius.

Right-click never commits a sector.

---

# 22. Sector Classification

For eight sectors:

```text
sector_width = 45°
```

After normalization and rotation:

```text
sector_index = floor((θ' + 22.5°) / 45°) mod 8
```

This produces North-centered sectors:

```text
N   = [337.5°, 22.5°)
NE  = [22.5°, 67.5°)
E   = [67.5°, 112.5°)
SE  = [112.5°, 157.5°)
S   = [157.5°, 202.5°)
SW  = [202.5°, 247.5°)
W   = [247.5°, 292.5°)
NW  = [292.5°, 337.5°)
```

Boundary handling must be deterministic.

---

# 23. Rendering

The renderer only needs to communicate the interaction state.

POC visual requirements:

* transparent background;
* eight visible stubs/sectors;
* deadzone visualization;
* hovered sector visibly distinguished;
* cursor/center relationship visually apparent.

Rendering should be event-driven.

A redraw is required when:

```text
hover_selection changes
```

or when the overlay is initially shown.

No 60/120/240 Hz render loop is required.

---

# 24. Selection Model

The interaction result is explicitly distinct from hover state.

```rust
enum Resolution {
    Commit(Sector),
    Cancel,
}
```

A left-click inside the deadzone produces:

```text
NoOp
```

and leaves the interaction `ACTIVE`.

Therefore the complete resolution model is:

```rust
enum MouseResolution {
    Commit(Sector),
    Cancel,
    NoOp,
}
```

`NoOp` is not equivalent to `Cancel`.

---

# 25. Input/State Invariants

`spec/INVARIANTS.yaml`

```yaml
invariants:

  - id: INV-INPUT-001
    category: input
    severity: critical
    statement: >
      Win+Esc activates the wheel exactly once per qualifying gesture.

  - id: INV-INPUT-002
    category: input
    severity: critical
    statement: >
      While ACTIVE, left/right mouse-button transitions are not forwarded
      to the foreground application.

  - id: INV-INPUT-003
    category: input
    severity: critical
    statement: >
      Mouse movement remains observable by WinPie while normal pointer
      movement continues through the operating system.

  - id: INV-INPUT-004
    category: input
    severity: critical
    statement: >
      The resolving left-button-down event is swallowed before foreground
      application delivery.

  - id: INV-INPUT-005
    category: input
    severity: critical
    statement: >
      A resolving button event is never re-emitted after returning to IDLE.

  - id: INV-INPUT-006
    category: input
    severity: critical
    statement: >
      No input-processing gap exists after ACTIVE begins in which a
      resolving button event can escape.

  - id: INV-STATE-001
    category: state
    severity: critical
    statement: >
      ACTIVE always terminates in IDLE through commit, cancel, or fatal error.

  - id: INV-STATE-002
    category: state
    severity: critical
    statement: >
      Esc key release is a no-op while ACTIVE. Win key release commits the
      hovered sector if cursor is within valid slice bounds, or cancels.

  - id: INV-STATE-003
    category: state
    severity: critical
    statement: >
      Left-click inside the deadzone or outside the wheel radius cancels
      and terminates ACTIVE (unified cancellation).

  - id: INV-STATE-004
    category: state
    severity: critical
    statement: >
      Hover selection is preselection only and cannot independently resolve
      or terminate the interaction.

  - id: INV-GEOMETRY-001
    category: geometry
    severity: critical
    statement: >
      The wheel contains exactly eight equal directional sectors.

  - id: INV-GEOMETRY-002
    category: geometry
    severity: critical
    statement: >
      Sector classification is deterministic for every finite cursor position.

  - id: INV-GEOMETRY-003
    category: geometry
    severity: critical
    statement: >
      Points inside the deadzone or outside the visual radius map to no directional hover sector.

  - id: INV-GEOMETRY-004
    category: geometry
    severity: critical
    statement: >
      A left-click within the valid ring (deadzone < r <= radius) resolves by angle;
      clicks outside this range cancel the interaction.

  - id: INV-GEOMETRY-005
    category: geometry
    severity: critical
    statement: >
      Right-click cancellation is independent of geometry.

  - id: INV-GEOMETRY-006
    category: geometry
    severity: critical
    statement: >
      Cursor coordinates and overlay coordinates use the same physical
      screen-coordinate space.

  - id: INV-GEOMETRY-007
    category: geometry
    severity: critical
    statement: >
      rotation_degrees changes the effective sector boundaries.

  - id: INV-OVERLAY-001
    category: overlay
    severity: critical
    statement: >
      Showing the overlay does not activate or focus its window.

  - id: INV-OVERLAY-002
    category: overlay
    severity: critical
    statement: >
      Pixels outside the rendered wheel remain visually transparent.

  - id: INV-OVERLAY-003
    category: overlay
    severity: critical
    statement: >
      The overlay does not become the effective mouse target for pointer
      movement.

  - id: INV-RECOVERY-001
    category: recovery
    severity: critical
    statement: >
      Every supported ACTIVE input sequence eventually returns to IDLE.
```

---

# 26. Recovery Property

Rather than the vague requirement “unexpected input must never leave the wheel permanently visible,” the POC defines the supported recovery domain explicitly.

The implementation must return to `IDLE` after any finite sequence containing:

* left-button down;
* right-button down;
* activation followed by either resolution path;
* fatal internal error.

Repeated:

```text
Win DOWN
Win UP
Esc DOWN
Esc UP
```

must not cause repeated activation while already ACTIVE.

No supported finite interaction sequence may leave the system permanently ACTIVE.

---

# 27. Fatal Error Handling

Fatal errors include failures such as:

* overlay creation failure;
* renderer initialization failure;
* unrecoverable Win32 resource failure;
* invariant violation detected at runtime.

On fatal error:

```text
ACTIVE
  │
  ▼
CANCEL
  │
  ├── hide overlay
  ├── release transient resources
  ├── reset input state
  └── IDLE
```

The application must fail closed with respect to the wheel UI.

It must not leave:

* a visible orphaned overlay;
* a permanently active interaction state;
* a permanently installed transient input state.

---

# 28. Hook Lifetime

The implementation must guarantee hook correctness rather than prescribing a particular lifetime strategy.

Two valid implementations are:

### Strategy A — persistent mouse hook

```text
process lifetime:
    WH_MOUSE_LL installed

ACTIVE:
    consume relevant buttons

IDLE:
    observe only
```

### Strategy B — ACTIVE-scoped mouse hook

```text
IDLE:
    no mouse hook

ACTIVATE:
    install hook before ACTIVE becomes externally observable

ACTIVE:
    consume relevant buttons

RESOLVE:
    consume resolving event
    hide overlay
    remove hook
    IDLE
```

For the POC, **Strategy A is preferable if it materially simplifies the no-gap guarantee**, but this is an implementation choice rather than a specification requirement.

---

# 29. Hook Callback Discipline

Low-level hook callbacks must remain extremely small.

They must not:

* render;
* block;
* sleep;
* execute arbitrary actions;
* perform expensive logging;
* perform heap-heavy work;
* call Python;
* perform filesystem operations.

The callback's job is essentially:

```text
receive OS input
      │
      ├── recognize relevant event
      ├── update minimal input state
      ├── determine whether event must be consumed
      └── return
```

The normal application/message-pump layer owns:

* overlay management;
* rendering;
* diagnostics;
* lifecycle cleanup.

---

# 30. Python Boundary

Python is explicitly **not** part of the POC interaction path.

If eventually required:

```text
Rust
 │
 └── external IPC/API boundary
          │
          ▼
       Python
```

Python may be used for high-level helper APIs, experimentation, or future action execution.

It must not participate in:

```text
mouse hook
keyboard hook
geometry
hover
commit
cancel
overlay rendering
```

---

# 31. Testing

## Geometry Unit Tests

Required:

```text
north
north-east
east
south-east
south
south-west
west
north-west
```

Test:

* center;
* deadzone boundary;
* just outside deadzone;
* exact sector boundaries;
* just either side of boundaries;
* negative screen coordinates;
* large coordinates;
* rotation;
* all eight sectors.

---

## Interaction Tests

### AT-001 — Activation

```text
IDLE
Win+Esc
→ ACTIVE
```

### AT-002 — Overlay anchoring

Cursor at arbitrary screen coordinate.

Expected:

```text
overlay_center == activation_cursor
```

### AT-003 — Hover

Move into each of eight sectors.

Expected:

```text
hover_selection == expected_sector
```

### AT-004 — Deadzone hover

Move cursor inside deadzone.

Expected:

```text
hover_selection == NONE
state == ACTIVE
```

### AT-005 — Keyboard release

Release Win and Esc.

Expected:

```text
state == ACTIVE
```

### AT-006 — Valid left click

Move outside deadzone and left-click.

Expected:

```text
Commit(expected_sector)
state == IDLE
overlay == hidden
```

### AT-007 — Far-radius left click

Left-click outside the visual outer radius along a valid angle.

Expected:

```text
Commit(expected_sector)
```

### AT-008 — Deadzone left click

Left-click inside deadzone.

Expected:

```text
NoOp
state == ACTIVE
overlay == visible
```

### AT-009 — Right-click

Right-click anywhere.

Expected:

```text
Cancel
state == IDLE
overlay == hidden
```

### AT-010 — Input swallowing

During ACTIVE, left/right click must not produce a click in the foreground application.

### AT-011 — Activation repeat

Hold/repeat Win+Esc while ACTIVE.

Expected:

```text
exactly one ACTIVE instance
```

### AT-012 — Focus

Activating WinPie must not change foreground application focus.

### AT-013 — Mouse continuity

Move cursor across overlay.

Expected:

```text
WinPie receives movement
normal foreground mouse behavior is preserved
```

### AT-014 — Cleanup

After commit/cancel:

```text
no visible overlay
no stuck ACTIVE state
no stale transient interaction state
```

### AT-015 — Fatal error

Force overlay initialization failure.

Expected:

```text
ACTIVE → IDLE
overlay hidden/nonexistent
transient input state cleaned
```

### AT-016 — Multi-monitor coordinates

Test on a monitor with negative virtual-screen coordinates.

Expected:

```text
geometry and overlay anchoring remain correct
```

### AT-017 — Rotation

Set:

```yaml
rotation_degrees: 22.5
```

Verify sector boundaries rotate by exactly 22.5°.

### AT-018 — Win-shell behavior

Perform the activation gesture repeatedly from a normal foreground application.

Expected:

```text
WinPie activates
no unintended Start-menu invocation
no unintended shell shortcut
```

---

# 32. Performance Requirements

These are targets, not correctness invariants.

| Operation                    |     Target |
| ---------------------------- | ---------: |
| Geometry evaluation          |     < 1 µs |
| Hover update                 | negligible |
| Sector change redraw         |     < 1 ms |
| Hook callback                |    << 1 ms |
| Activation perceived latency |    < 20 ms |

The primary performance principle is:

> **No polling loop and no unnecessary work in the hook path.**

---

# 33. Definition of Done

The POC is complete when all of the following hold:

```text
                    Win + Esc
                        │
                        ▼
                 ┌────────────┐
                 │    ACTIVE  │
                 └─────┬──────┘
                       │
              cursor movement
                       │
                       ▼
                 hover sector
                       │
              ┌────────┴────────┐
              │                 │
          left-click        right-click
              │                 │
              ▼                 ▼
           commit             cancel
              │                 │
              └────────┬────────┘
                       ▼
                    IDLE
```

Specifically:

* [x] Win+Esc activates the interaction.
* [x] Escape activation input is consumed as specified.
* [x] No unintended Windows-shell activation occurs.
* [x] Overlay is anchored to activation cursor position.
* [x] Overlay does not steal focus.
* [x] Overlay is visually transparent outside the wheel.
* [x] Overlay does not become the effective pointer target.
* [x] Eight sectors are rendered with anti-aliasing and constant spoke widths.
* [x] Cursor movement updates hover state via precomputed 0ms bitmap cache.
* [x] Deadzone and outer bounds produce `NONE` hover.
* [x] Left-click or Win-up within slice bounds commits by angle.
* [x] Left-click in deadzone or out of bounds cancels (unified cancel).
* [x] Right-click cancels anywhere.
* [x] Resolving button events cannot leak to the foreground application.
* [x] Esc release is a no-op; Win release resolves the interaction.
* [x] Keys held after cancel require re-press before next activation.
* [x] Rotation changes sector boundaries.
* [x] Multi-monitor physical coordinates behave correctly.
* [x] Commit/cancel always tears down the interaction.
* [x] Fatal errors return to `IDLE`.
* [x] No supported interaction sequence leaves the wheel permanently active.
* [x] Geometry tests pass.
* [x] Interaction tests pass.

---

# 34. Implementation Principle

The POC should remain deliberately small:

```text
             INPUT
               │
               ▼
          STATE MACHINE
          /           \
         ▼             ▼
     GEOMETRY       OVERLAY
         │             │
         └──────┬──────┘
                ▼
             RESULT
```

No action framework, plugin system, ECS, reactive framework, async runtime, scripting engine, or general-purpose UI abstraction belongs in this implementation.

**The POC proves one thing:**

> A globally activated, focus-preserving, eight-way radial interaction can intercept its own resolution clicks, provide deterministic directional hover feedback, and cleanly return control to the surrounding Windows environment.

That is the implementation boundary.
