# WinPie - Agent Guidelines & Architecture Gotchas

This document serves as the operational guide and invariant reference for autonomous agents and contributors working on the WinPie codebase.

---

## 1. Critical Architecture Gotchas & Win32 Invariants

### 1.1 NEVER Use `GetAsyncKeyState` Inside Low-Level Hook Callbacks
- **The Pitfall**: In the Windows subsystem, low-level hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`) are invoked **synchronously before** the OS updates its asynchronous keystate table queried by `GetAsyncKeyState()`.
- **The Consequence**: Calling `GetAsyncKeyState(VK_LWIN)` inside `ll_keyboard_proc` during a `WM_KEYUP` / `WM_SYSKEYUP` event will still return `0x8000` (pressed). This causes `is_up` checks to evaluate as still held, completely breaking release commits (`WM_WINPIE_WINUP`) and key-release reset signals (`WM_WINPIE_ALLKEYSUP`).
- **The Rule**: Low-level hook callbacks must **exclusively** rely on atomic boolean flags (`LEFT_WIN_DOWN`, `RIGHT_WIN_DOWN`, `ESCAPE_DOWN`, `SHIFT_DOWN`) updated directly and synchronously by incoming `WM_KEYDOWN` and `WM_KEYUP` messages.

### 1.2 Typematic Autorepeat & The `REQUIRE_KEY_RELEASE` Guard
- **The Pitfall**: When a user physically holds down a key combination (like <kbd>Win</kbd> + <kbd>Esc</kbd>), the keyboard hardware driver continuously emits typematic autorepeat messages (`WM_KEYDOWN` every ~30ms).
- **The Consequence**: If the radial menu or modal submenu is cancelled (via right-click, deadzone click, or <kbd>Esc</kbd>) while <kbd>Win</kbd> or <kbd>Esc</kbd> are still physically depressed, the very next hardware autorepeat tick (~30ms later) will see `IS_ACTIVE == false` and immediately re-open the radial menu in the exact same frame. To the user, the menu appears impossible to dismiss.
- **The Rule**:
  - Whenever an active menu is dismissed while keys are held, immediately set `REQUIRE_KEY_RELEASE.store(true, Ordering::SeqCst)`.
  - The activation check in `ll_keyboard_proc` MUST verify `!REQUIRE_KEY_RELEASE.load(Ordering::SeqCst)` before activating.
  - `REQUIRE_KEY_RELEASE` MUST ONLY be cleared back to `false` when all activation keys have physically emitted `is_up` (`!win_held && !esc_held`).

### 1.3 FSM Self-Healing on Fresh Activation
- **The Pitfall**: If an external application steals focus or a low-level keyup message is dropped by the OS, the interaction FSM could remain stranded in `State::WaitRelease`.
- **The Rule**: `InteractionEvent::WinEscDown(anchor)` in `src/interaction/state.rs` must unconditionally transition to `State::Active { anchor, hover: None }` and emit `InteractionEffect::Activated` from both `State::Idle` **and** `State::WaitRelease`.

### 1.4 Input Swallowing vs. System Input Leakage
- **The Pitfall**: Greedily returning `LRESULT(1)` for unhandled keys or mouse events traps OS input and can lock the user out of Windows without <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>Del</kbd>.
- **The Rule**:
  - In `ll_mouse_proc`: **NEVER swallow `WM_LBUTTONUP` or `WM_RBUTTONUP`**. Only swallow `WM_LBUTTONDOWN` and `WM_RBUTTONDOWN` when `IS_ACTIVE` or `IS_MODAL_MENU` are active.
  - In `ll_keyboard_proc`: In modal submenus, only swallow alphanumeric shortcut keys (<kbd>A</kbd>-<kbd>Z</kbd>, <kbd>0</kbd>-<kbd>9</kbd>), <kbd>Tab</kbd>, and <kbd>Esc</kbd>. Pass through all other keys (modifiers, arrows, Enter, function keys) via `CallNextHookEx`.

### 1.5 Windows Start Menu Disarming via Mask Key `0xE8`
- **The Pitfall**: Releasing the Windows key without an intervening key stroke triggers the Windows Start Menu.
- **The Rule**: When <kbd>Win</kbd> + <kbd>Esc</kbd> activation is detected, immediately synthesize a dummy unassigned virtual key stroke (`0xE8`) using `SendInput` before releasing. `ll_keyboard_proc` must pass through `0xE8` events to prevent recursion.

### 1.6 Process Mutex & Background Daemon Management
- **The Pitfall**: Multiple running instances of WinPie can conflict on low-level hooks, double-capture hotkeys, and lock the compiled binary during `cargo build`.
- **The Rule**:
  - WinPie enforces a single-instance daemon using a named Win32 mutex (`Global\WinPie_SingleInstance_Mutex`).
  - Both `build.ps1` and `run.ps1` must forcefully terminate existing `winpie` processes (`Stop-Process -Force`) before compiling or running.

---

## 2. Codebase Structure & Key Files

| Module / File | Responsibility |
|---|---|
| `src/input/keyboard.rs` | Low-level `WH_KEYBOARD_LL` hook, hotkey detection, modal key capture, autorepeat guard |
| `src/input/mouse.rs` | Low-level `WH_MOUSE_LL` hook, mouse move deduplication, click/RMB interception |
| `src/input/state.rs` | Atomic key flags (`LEFT_WIN_DOWN`, `REQUIRE_KEY_RELEASE`, etc.), mask key injection |
| `src/interaction/state.rs` | Pure deterministic Finite State Machine (FSM) for radial menu and modal submenus |
| `src/context.rs` | Sub-millisecond foreground window context inspection and process running checks |
| `src/config/mod.rs` | YAML configuration schema, theme parsing, and contextual profile inheritance |
| `src/ipc.rs` | Single-instance mutex, control window message receiver, and Windows autostart |
| `src/overlay/` | Layered Win32 windows, GDI rasterizer with 4x RGSS, and Toast HUD overlays |

---

## 3. Verification & Quality Gates

When modifying input hooks, FSM transitions, or overlay logic:
1. **Always run the full build pipeline**:
   ```powershell
   .\build.ps1 -Release -Clippy -Test
   ```
2. **Maintain 0 Clippy warnings** (`-D warnings` enforced in build script).
3. **Verify all 36+ unit and integration tests pass** across:
   - `tests/geometry.rs`: Radial math, Euclidean bounds, theme deserialization.
   - `tests/interaction.rs`: FSM transitions, breadcrumbs, RMB cancel, WaitRelease re-arm.
   - `tests/context.rs`: Foreground window context matching and process inspection.
4. **Interactive Verification**:
   - Verify <kbd>Win</kbd> + <kbd>Esc</kbd> opens radial menu repeatedly.
   - Verify <kbd>Win</kbd>-up commit and left-click commit.
   - Verify Right-Click dismisses cleanly without autorepeat re-opening while keys are held.
   - Verify `winpie inspect` streams active window context in real time.
