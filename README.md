# WinPie

Lightweight, zero-latency 8-way radial interaction primitive for Windows 10/11 x64, built in Rust.

WinPie provides global, focus-preserving radial menu invocation anchored to the current cursor position, supporting both click-to-commit and gesture-release flick interactions with true per-pixel alpha transparency and zero input leakage.

---

## Features

- **Global Hotkey Activation**: Invoke anywhere via <kbd>Win</kbd> + <kbd>Esc</kbd>. Swallows the activation trigger and suppresses Windows Start Menu popups via non-intrusive modifier disarming.
- **Fast Flick or Click Commitment**:
  - **Flick Gesture**: Hold <kbd>Win</kbd>, press <kbd>Esc</kbd>, move cursor into target sector, and release <kbd>Win</kbd> to immediately commit.
  - **Click to Commit**: Left-click any sector slice to commit.
- **Unified Cancellation**:
  - Right-click anywhere cancels immediately.
  - Left-clicking inside the deadzone or outside the wheel radius cancels cleanly.
  - Releasing <kbd>Win</kbd> while inside the deadzone or out of bounds cancels without action.
- **No Stuck Modifiers / Re-Press Guard**: When cancelled or committed while keys remain held down, WinPie enters a silent waiting state until keys are physically released, preventing accidental re-activations or stuck modifier states.
- **True Per-Pixel Alpha & Constant Geometry**:
  - Rendered via Win32 layered popup (`WS_EX_LAYERED`, `WS_EX_NOACTIVATE`, `WS_EX_TOPMOST`, `WS_POPUP`).
  - Transparent input hit-testing (`HTTRANSPARENT`) so foreground applications preserve native hover and focus.
  - 4x Rotated Grid Supersampling (RGSS) anti-aliasing for buttery-smooth circular rims and constant-width Euclidean spoke dividers.
  - Pre-rasterized bitmap cache on startup for **< 0.05ms** instant hover state blits without cursor lag.
- **Multi-Monitor & DPI Aware**: Physical screen coordinate math across arbitrary multi-monitor topologies (including negative virtual screen bounds).

---

## Interaction Model

```text
               Hold Win + Press Esc
                         │
                         ▼
                  ┌─────────────┐
                  │   ACTIVE    │
                  └──────┬──────┘
                         │
            Move cursor into direction
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   Release Win       Left-Click       Right-Click
  (inside sector)  (inside sector)     (anywhere)
        │                │                │
        ▼                ▼                ▼
      COMMIT           COMMIT           CANCEL
        │                │                │
        └────────────────┴────────────────┘
                         │
          Any activation keys still held?
                     /       \
                Yes /         \ No
                   ▼           ▼
           ┌──────────────┐    │
           │ WAIT_RELEASE │    │
           └──────┬───────┘    │
                  │            │
            All keys up        │
                  │            │
                  ▼            ▼
               ┌──────────────────┐
               │       IDLE       │
               └──────────────────┘
```


---

## Quick Start

### Prerequisites
- Windows 10 or 11 (x64)
- [Rust toolchain](https://rustup.rs/) (1.75+)

### Launching

Use the provided PowerShell convenience runner:

```powershell
# Run release build
.\run.ps1 -Release

# Run debug build
.\run.ps1

# Build only without launching
.\run.ps1 -BuildOnly
```

Or via Cargo directly:

```powershell
cargo run --release
```

---

## Configuration

Configuration is defined in `config/default.yaml`:

```yaml
activation:
  modifier: win
  trigger: escape
  swallow_trigger: true
  left_win: true
  right_win: true

overlay:
  radius: 180           # Outer wheel radius in physical pixels
  deadzone: 32          # Inner deadzone radius in physical pixels
  coordinate_space: physical_screen_pixels

wheel:
  slices: 8
  rotation_degrees: 0.0 # 0.0 centers North on [337.5°, 22.5°)
  # Custom sector titles (fall back to direction name if omitted)
  labels:
    N: "Terminal"
    NE: "Browser"
    E: "Sublime"
    SE: "Files"
    S: "Settings"
    SW: "Music"
    W: "Tasks"
    NW: "Chat"
  # Arbitrary executable binaries, commands, or scripts
  commands:
    E: "sublime.exe"
    N: "wt.exe"
    SE: "explorer.exe"
    S: "control.exe"

rendering:
  show_stubs: true
  show_deadzone: true
  highlight_hovered: true
  show_labels: true            # Toggle label rendering
  font:
    family: "Segoe UI"         # Windows font family
    size: 13                   # Font point size / pixel height
    weight: 600                # Font weight (400 normal, 600 semibold, 700 bold)
    radius_ratio: 0.62         # Distance ratio from deadzone to outer radius

diagnostics:
  enabled: true
  log_selection: true
  log_cancel: true
  log_fatal_errors: true

toast:
  enabled: true
  duration_ms: 1000          # Overlay lifespan in milliseconds (default: 1000ms / 1s)
  corner: bottom_right       # Options: bottom_right, bottom_left, top_right, top_left
  margin_x: 24               # Margin from screen edge in physical pixels
  margin_y: 24               # Margin from screen edge in physical pixels
  font_size: 13              # Toast text size
  show_sector_direction: true # e.g. "Terminal (N)" vs "Terminal"
```

---

## Project Architecture

```text
winpie/
├── Cargo.toml
├── config/
│   └── default.yaml          # Runtime configuration
├── spec/
│   ├── INVARIANTS.yaml       # Formal invariant specifications
│   ├── STATE_MACHINE.yaml    # Formal FSM definition
│   └── GEOMETRY.yaml         # Sector & angle definitions
├── src/
│   ├── main.rs               # Entrypoint
│   ├── app.rs                # Application runtime, DPI & Win32 message pump
│   ├── input/
│   │   └── mod.rs            # WH_KEYBOARD_LL & WH_MOUSE_LL low-level hooks
│   ├── interaction/
│   │   ├── mod.rs
│   │   └── state.rs          # Pure FSM state transition engine
│   ├── geometry/
│   │   └── mod.rs            # 8-way radial angle & Euclidean boundary math
│   ├── overlay/
│   │   ├── mod.rs
│   │   └── window.rs         # Win32 Layered window & RGSS precomputed renderer
│   └── diagnostics.rs        # Diagnostics logging
└── tests/
    ├── geometry.rs           # Mathematical & coordinate test suite
    └── interaction.rs        # State machine transition verification
```

---

## Testing

Run the full automated test suite:

```powershell
cargo test
```

All 13 test suites verify geometry boundaries, rotation offsets, negative coordinate spaces, FSM transitions, and gesture cancellations.

---

## License

MIT or Apache-2.0
