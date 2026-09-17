# WinPie

Lightweight, zero-latency 8-way radial interaction primitive and modal submenu system for Windows 10/11 x64, built in Rust.

WinPie provides global, focus-preserving radial menu invocation anchored to the current cursor position, supporting flick gestures, mouse clicking, nested modal submenus with keyboard/mouse navigation, dual ambient toast HUDs, universal color/opacity theming, and full command-line lifecycle management.

---

## Features

- **Global Hotkey Activation**: Invoke anywhere via <kbd>Win</kbd> + <kbd>Esc</kbd>. Swallows the activation trigger and suppresses Windows Start Menu popups via non-intrusive modifier disarming (`0xE8`).
- **Flick or Click Commitment**:
  - **Flick Gesture**: Hold <kbd>Win</kbd>, press <kbd>Esc</kbd>, flick cursor into target sector, and release <kbd>Win</kbd> to immediately commit.
  - **Click to Commit**: Left-click any sector slice to commit.
- **Nested Modal Submenus**:
  - Sectors can launch interactive modal submenu cards with arbitrary nesting depth.
  - **Keyboard Driven**: Press a key (<kbd>A</kbd>-<kbd>Z</kbd>, <kbd>0</kbd>-<kbd>9</kbd>) to preview row glow and tooltip; release the key (<kbd>KeyUp</kbd>) to commit execution or drill down.
  - **Mouse Interactive**: Hover rows to preview item tooltips and highlight; left-click to execute or drill down.
  - **Breadcrumb Navigation**:
    - <kbd>Tab</kbd>: Loops / resets back to the root submenu.
    - <kbd>Shift</kbd> + <kbd>Tab</kbd>: Backtracks up one level to the parent menu.
    - <kbd>Esc</kbd> or **Right-Click**: Unconditionally cancels back to `IDLE` with zero input leakage.
- **Dual Toast HUD Architecture**:
  - **Primary Toast**: Displays ambient tooltips on sector/item hover and action execution notifications with auto-dismiss timers.
  - **Dedicated Hint Toast**: Spawns in a configurable screen corner (e.g. `bottom_right`) displaying navigation shortcuts (`[Tab] Loop  [Shift+Tab] Back  [Esc / RMB] Cancel`).
  - **Startup Toast**: Configurable welcome notification on daemon launch (default 5s duration).
- **Universal Color & Opacity Theming**:
  - Configurable sleek corner radii (`menu_corner_radius: 4.0`, `toast_corner_radius: 4.0`).
  - Hex color codes (`#RRGGBB`) and opacity multipliers (`0.0..=1.0`) for every UI element: spokes, rim, hover glow, deadzone core/border, backgrounds, card borders, key badges, branch arrows, and labels.
  - Offscreen ClearType GDI rasterization with 32-bit pre-multiplied Porter-Duff alpha compositing.
- **Unified Cancellation & Re-Press Guard**:
  - Right-click anywhere cancels immediately.
  - Left-clicking inside the deadzone or outside the wheel radius cancels cleanly.
  - Silent `WAIT_RELEASE` state prevents stuck modifiers or accidental re-activations until keys are physically released.
- **Dynamic Contextual Profiles**:
  - Automatically adapts radial menu sectors, labels, tooltips, commands, and themes based on the currently active application (e.g. Visual Studio Code, Windows File Explorer, Sublime Text, Terminal).
  - Sub-millisecond foreground window inspection using Win32 API (`GetForegroundWindow`, `GetClassNameW`, `QueryFullProcessImageNameW`).
  - Pre-rasterized buffers for every profile guaranteeing zero latency.
  - Seamless fallback and inheritance: unassigned sectors automatically inherit from the global default configuration.
- **Command-Line Lifecycle & Single-Instance Daemon**:
  - `winpie` operates both as a background daemon (with single-instance Mutex protection) and as a CLI control tool (`winpie kill`, `winpie reload`, `winpie status`, `winpie autostart`).
- **Automated PATH Linking & Windows Startup**:
  - `build.ps1 -Release` automatically links `winpie.exe` into user `PATH` (`%USERPROFILE%\.cargo\bin\winpie.exe`) and creates the Windows Startup shortcut (`WinPie.lnk`).


---

## CLI Usage

```powershell
WinPie - High-Performance Radial Menu for Windows

USAGE:
  winpie                       Start WinPie radial menu daemon
  winpie kill                  Stop the running WinPie process
  winpie reload                Restart / reload WinPie with updated config
  winpie status                Check if WinPie is currently running
  winpie inspect               Live stream active window context & profile matching
  winpie autostart enable      Launch WinPie automatically on Windows login
  winpie autostart disable     Remove WinPie from Windows startup
  winpie autostart status      Check Windows startup shortcut status
  winpie help                  Show this help message

DEFAULT CONTROLS:
  Win + Esc                    Activate radial pie menu
  Left Click                   Commit sector or enter submenu
  Release Win Key              Commit hovered sector
  Right Click / Esc            Cancel menu
```

---

## Quick Start & Installation

### Prerequisites
- Windows 10 or 11 (x64)
- [Rust toolchain](https://rustup.rs/) (1.75+)

### Building and Installing

Use `build.ps1` to run the full verification pipeline and install globally:

```powershell
# Build optimized release binary, link to PATH, and create Windows Startup shortcut:
.\build.ps1 -Release -Clippy -Test

# Now you can run winpie from any terminal!
winpie
```

### Build Script Options

```powershell
.\build.ps1 -Release      # Compile optimized release binary & install to PATH/Startup
.\build.ps1 -Test         # Run full unit and integration test suite
.\build.ps1 -Clippy       # Run linter with -D warnings
.\build.ps1 -Clean        # Clean target cache before building
.\build.ps1 -All          # Full pipeline: Clean -> Clippy -> Build -> Test
```

---

## Configuration Reference

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
  labels:
    N: "Dev Tools"
    NE: "Browser"
    E: "Sublime"
    SE: "Files"
    S: "Settings"
    SW: "Music"
    W: "Tasks"
    NW: "Chat"
  tooltips:
    N: "Development environment & build tools"
    NE: "Open default web browser"
    E: "Launch Sublime Text editor"
    SE: "Open Windows File Explorer"
    S: "Open Windows Settings"
    SW: "Open Media / Music player"
    W: "Open Task Manager"
    NW: "Open Communications / Chat"
  commands:
    NE: "cmd.exe /c start https://google.com"
    E: "sublime.exe"
    SE: "explorer.exe"
    S: "control.exe"
    W: "taskmgr.exe"
  menus:
    N:
      title: "Dev Tools"
      tooltip: "Development tools, editors, and build pipelines"
      items:
        a:
          label: "VS Code"
          tooltip: "Launch Visual Studio Code"
          command: "code.exe"
        s:
          label: "Sublime Text"
          tooltip: "Launch Sublime Text editor"
          command: "sublime.exe"
        t:
          label: "Windows Terminal"
          tooltip: "Open Windows Terminal"
          command: "wt.exe"
        d:
          label: "Cargo Pipeline"
          tooltip: "Rust build and test commands"
          menu:
            title: "Cargo Pipeline"
            tooltip: "Cargo clean, build, and test actions"
            items:
              c:
                label: "Cargo Clean"
                tooltip: "Wipe target build cache"
                command: "cargo clean"
              b:
                label: "Cargo Build"
                tooltip: "Compile debug binary"
                command: "cargo build"
              t:
                label: "Cargo Test"
                tooltip: "Run test suite"
                command: "cargo test"

rendering:
  show_stubs: true
  show_deadzone: true
  highlight_hovered: true
  show_labels: true
  font:
    family: "Segoe UI"
    size: 13
    weight: 600
    radius_ratio: 0.62

diagnostics:
  enabled: true
  log_selection: true
  log_cancel: true
  log_fatal_errors: true

toast:
  enabled: true
  show_hover_tooltips: true
  duration_ms: 1000          # Duration in milliseconds
  corner: top_right          # Options: bottom_right, bottom_left, top_right, top_left
  margin_x: 24
  margin_y: 24
  font_size: 13
  show_sector_direction: true

startup_toast:
  enabled: true
  duration_ms: 5000          # 5 seconds default
  corner: top_right
  margin_x: 24
  margin_y: 24
  font_size: 13
  text: "WinPie is active (Press Win+Esc)"

hint_toast:
  enabled: true
  corner: bottom_right
  margin_x: 24
  margin_y: 24
  font_size: 11
  text: "[Tab] Loop   [Shift+Tab] Back   [Esc / RMB] Cancel"

theme:
  menu_corner_radius: 4.0
  toast_corner_radius: 4.0

  accent_color: "#00AFFF"
  accent_opacity: 1.0

  main_bg_color: "#14161C"
  main_bg_opacity: 0.94

  secondary_bg_color: "#1E222B"
  secondary_bg_opacity: 0.80

  border_color: "#3C465A"
  border_opacity: 0.85

  text_primary: "#FFFFFF"
  text_secondary: "#A0A5B5"
  text_accent: "#00AFFF"

  wheel:
    spoke_color: "#EBEBEB"
    spoke_opacity: 0.75
    rim_color: "#F0F0F0"
    rim_opacity: 0.86
    hover_glow_color: "#00AFFF"
    hover_glow_opacity: 0.86
    sector_bg_color: "#16181E"
    sector_bg_opacity: 0.59
    deadzone_bg_color: "#141414"
    deadzone_bg_opacity: 0.51
    deadzone_border_color: "#F0F0F0"
    deadzone_border_opacity: 0.86

profiles:
  - name: "Visual Studio Code"
    match_rules:
      process: "code.exe"
    wheel:
      labels:
        N: "Format Document"
        NE: "Command Palette"
        E: "Go to Symbol"
        SE: "File Explorer"
        S: "Toggle Terminal"
        SW: "Git Status"
        W: "Find in Files"
        NW: "Toggle Sidebar"
      tooltips:
        N: "Format current active code document"
        NE: "Open VS Code Command Palette"
        E: "Quick jump to symbol in file"
        SE: "Reveal file in Explorer sidebar"
        S: "Toggle built-in integrated terminal"
        SW: "Show Source Control git panel"
        W: "Search across entire workspace"
        NW: "Toggle primary sidebar visibility"
      commands:
        N: "cmd.exe /c code --command editor.action.formatDocument"
        NE: "cmd.exe /c code --command workbench.action.showCommands"
        S: "cmd.exe /c code --command workbench.action.terminal.toggleTerminal"
        W: "cmd.exe /c code --command workbench.action.findInFiles"

  - name: "Windows File Explorer"
    match_rules:
      process: "explorer.exe"
      window_class: "CabinetWClass"
    wheel:
      labels:
        N: "New Folder"
        NE: "Terminal Here"
        E: "VS Code Here"
        SE: "Copy Path"
        S: "Properties"
        SW: "Select All"
        W: "Git Bash Here"
        NW: "Refresh"
      tooltips:
        N: "Create a new directory"
        NE: "Open Windows Terminal in current directory"
        E: "Open directory in Visual Studio Code"
        SE: "Copy full path to clipboard"
        S: "Open Properties window"
        SW: "Select all items in folder"
        W: "Open Git Bash here"
        NW: "Refresh directory view"
      commands:
        NE: "wt.exe"
        E: "code.exe ."
```

---

## Project Architecture

```text
winpie/
├── Cargo.toml
├── build.ps1                 # Release build, testing, linter, PATH & Startup installer
├── run.ps1                   # Local runner script
├── config/
│   └── default.yaml          # Runtime configuration, themes & contextual profiles
├── spec/
│   ├── INVARIANTS.yaml       # Formal invariant specifications
│   ├── STATE_MACHINE.yaml    # Formal FSM definition
│   └── GEOMETRY.yaml         # Sector & angle definitions
├── src/
│   ├── main.rs               # Entrypoint & CLI command dispatcher
│   ├── app.rs                # Application runtime, DPI & Win32 message pump
│   ├── context.rs            # Active window inspection & contextual profile matching
│   ├── ipc.rs                # Win32 IPC, mutex, control window & autostart manager
│   ├── input/
│   │   ├── mod.rs            # WH_KEYBOARD_LL & WH_MOUSE_LL low-level hooks
│   │   ├── keyboard.rs       # Keyboard event interception & modal shortcut processing
│   │   ├── mouse.rs          # Mouse event hit-testing & deduplication
│   │   └── state.rs          # Atomic input state & Win32 Start-menu disarm
│   ├── interaction/
│   │   ├── mod.rs
│   │   └── state.rs          # Pure FSM state transition engine
│   ├── geometry/
│   │   ├── mod.rs            # 8-way radial angle & Euclidean boundary math
│   │   ├── point.rs          # Multi-monitor coordinate packing/unpacking
│   │   └── sector.rs         # Sector definitions & classification
│   ├── overlay/
│   │   ├── mod.rs
│   │   ├── window.rs         # Win32 layered radial menu window
│   │   ├── render.rs         # 4x RGSS wheel & ClearType label rasterization
│   │   ├── modal_menu.rs     # Layered modal submenu card & hit-testing
│   │   └── toast.rs          # Layered toast notification & ambient tooltip overlay
│   ├── executor.rs           # Detached asynchronous command spawner
│   └── diagnostics.rs        # Diagnostics logging & event tracking
└── tests/
    ├── context.rs            # Window context & contextual profile inheritance test suite
    ├── geometry.rs           # Geometry, label, theme, startup & toast test suite
    └── interaction.rs        # State machine, submenu & mouse hit-test verification
```

---

## Testing

Run the full automated test suite:

```powershell
cargo test --all-targets
```

All 36 unit and integration tests verify:
- 8-way angular classification and rotation offsets
- Multi-monitor negative virtual screen coordinate roundtrips
- Deadzone and outer radius Euclidean boundaries
- State machine transitions (flick gestures, click commits, modifier re-press guards)
- Modal submenu key preview, keyup execution, and mouse hit-testing
- Breadcrumb navigation (<kbd>Tab</kbd> loop, <kbd>Shift+Tab</kbd> backtrack, cancellation)
- Universal theme parsing and startup toast configuration
- Active window context inspection, process matching, and profile inheritance fallback

---

## License

MIT or Apache-2.0
