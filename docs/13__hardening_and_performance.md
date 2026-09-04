# Phase 12: Hardening & Performance

**Document ID:** `13__hardening_and_performance.md`  
**Roadmap Phase:** Phase 12  
**Status:** Planned  
**Dependencies:** All Prior Phases (Phases 0 through 11)  

---

## 1. Executive Summary & Strategic Intent

Phase 12 represents the final production engineering pass for WinPie. A utility that intercepts global input hooks and sits continuously in the background must achieve the reliability of an OS kernel driver:
- It must **never leak memory** or file descriptors over months of continuous operation.
- It must **never hang or stall the Windows message pump**, which would freeze system cursor movement.
- It must **gracefully recover from OS-level faults**: hook unhooking by Windows timeout policies, graphics driver restarts (`DXGI_ERROR_DEVICE_RESET`), display topology changes, and process crashes.

---

## 2. Hardening Vectors & Engineering Disciplines

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                              PRODUCTION HARDENING DISCIPLINES                            │
└───────────────────────────────────────────┬─────────────────────────────────────────────┘
                                            │
         ┌───────────────────┬──────────────┴───────┬───────────────────┐
         ▼                   ▼                      ▼                   ▼
┌──────────────────┐┌──────────────────┐┌──────────────────┐┌──────────────────┐
│  Hook Robustness ││ Message Loop &   ││ Multi-Monitor &  ││ Memory & Handle  │
│  & OS Watchdog   ││ Crash Isolation  ││ DPI Dynamic Sync ││ Leak Elimination │
├──────────────────┤├──────────────────┤├──────────────────┤├──────────────────┤
│ Auto-reinstall on││ Structured SEH / ││ Per-monitor V2   ││ Zero heap allocs │
│ Windows timeout  ││ panic catch_un-  ││ topology change  ││ in hot paths; 0  │
│ unhooking policy ││ wind protection  ││ dynamic rescale  ││ unclosed handles │
└──────────────────┘└──────────────────┘└──────────────────┘└──────────────────┘
```

---

## 3. Detailed Subsystem Hardening Specifications

### 3.1. Hook Watchdog & Auto-Recovery (`input/watchdog.rs`)
- **The Windows Hook Timeout Problem:** Windows enforces a low-level hook timeout (typically $200\,\text{ms} \dots 1000\,\text{ms}$, configured via `LowLevelHooksTimeout` in the registry). If an application hook fails to respond within this window, Windows silently removes the hook from the global hook chain without notification.
- **The Solution:** A dedicated lightweight heartbeat watchdog thread continuously monitors hook liveness:
  - If a simulated synthetic test event is not received within expected intervals, or if `UnhookWindowsHookEx` was triggered by the OS, the watchdog automatically unregisters and reinstalls `WH_KEYBOARD_LL` and `WH_MOUSE_LL` seamlessly.

### 3.2. Structured Panic Recovery & SEH Guards
- All low-level C-ABI hook callbacks (`LowLevelKeyboardProc`, `LowLevelMouseProc`) and window procedures (`WndProc`) are wrapped in `std::panic::catch_unwind`.
- Under no circumstances will a Rust panic unwind across the FFI boundary into Windows `user32.dll` (which results in immediate process termination by the Windows CRT).
- Any caught panic logs a full backtrace to disk (`crash.log`), resets the FSM to `IDLE`, restores normal cursor pass-through, and allows the message loop to proceed.

### 3.3. Multi-Monitor Dynamic Topology & Mixed-DPI Resiliency
- Listens for `WM_DISPLAYCHANGE` and `WM_DPICHANGED` messages.
- If a display is unplugged or resolution alters while WinPie is active, the overlay window immediately repins to the nearest valid active monitor work area, recalibrating radius physical dimensions to match the target display DPI.

### 3.4. Memory & Handle Leak Quarantine
- All GDI objects (`HBITMAP`, `HDC`), window handles (`HWND`), process handles, and COM pointers (`IUnknown`) are wrapped in RAII drop guards.
- Continuous 72-hour soak tests run under synthetic load ($100,000$ consecutive invocations) to prove zero increase in:
  - Working set memory (must remain strictly $< 30\,\text{MB}$).
  - GDI object count (must remain constant).
  - USER object count (must remain constant).
  - Open OS handle count.

---

## 4. Latency & Resource Budgets (Production Acceptance Gates)

The final release must satisfy every metric in this table:

| Subsystem / Operation | Target Budget | Hard Failure Limit | Verification Method |
| :--- | :--- | :--- | :--- |
| **Hook Pass-Through Latency** | $< 0.05\,\text{ms}$ | $> 0.20\,\text{ms}$ | High-res hardware QPC timer inside hook proc |
| **Invocation to First Pixel** | $< 5.0\,\text{ms}$ | $> 8.0\,\text{ms}$ | End-to-end timestamp from `Win+Esc` to `UpdateLayeredWindow` |
| **Hover Redraw Latency** | $< 1.0\,\text{ms}$ | $> 2.5\,\text{ms}$ | Frame blit time during slice transition |
| **Command Resolution Time** | $< 0.5\,\text{ms}$ | $> 1.5\,\text{ms}$ | Graph predicate matching across 2,000 nodes |
| **Idle CPU Utilization** | **0.0%** | $> 0.1\%$ | Windows Performance Monitor (`perfmon`) over 1 hour idle |
| **Active Memory Footprint** | $< 25\,\text{MB}$ | $> 45\,\text{MB}$ | Private working set bytes under heavy load |
| **GDI Handles Leaked** | **0** | $> 0$ | Checked via `GetGuiResources(GR_GDIOBJECTS)` |

---

## 5. Packaging, Autostart & Silent Daemon Operation

1. **Self-Contained Binary:** Builds as a single optimized release executable (`winpie.exe`, $\le 10\,\text{MB}$) with all core assets, fallback icons, and default configurations bundled.
2. **Silent Background Daemon:** Subsystem set to `windows` (`#![windows_subsystem = "windows"]`) to prevent any console window flicker on boot.
3. **Taskbar System Tray Icon:**
   - Provides minimal system tray presence: Status indicator, Open Config folder, Reload Config, View Diagnostics, Exit.
   - Middle-click or double-click to pause/resume global interception.
4. **Clean Autostart:** One-click integration with Windows Startup folder or Task Scheduler (allowing clean startup with elevated privileges if desired).

---

## 6. Verification & Stress Test Suite

1. [ ] **Soak Test (100,000 Invocations):** Automated harness invokes, navigates, and dismisses WinPie 100,000 times over 12 hours. Verifies memory, GDI objects, and thread count remain flat.
2. [ ] **Chaos Input Test:** Injects 10,000 random key combinations and rapid mouse clicks per second while triggering and dismissing WinPie. Asserts no crashes, deadlocks, or stuck modifier keys.
3. [ ] **Display Disconnect Simulation:** Fires simulated `WM_DISPLAYCHANGE` with radical resolution shifts while overlay is visible; asserts zero rendering glitches or division-by-zero crashes.
4. [ ] **Hook Timeout Resilience Test:** Artificially stalls an unrelated test hook to force Windows hook eviction; asserts WinPie watchdog reinstalls hooks within $500\,\text{ms}$.
