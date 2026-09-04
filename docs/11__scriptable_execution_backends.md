# Phase 10: Scriptable Execution Backends

**Document ID:** `11__scriptable_execution_backends.md`  
**Roadmap Phase:** Phase 10  
**Status:** Planned  
**Dependencies:** Phase 0 Executor (`executor.rs`), Phase 2 (`03__context_detection.md`), Phase 3 (`04__context_hydration_and_capability_providers.md`)  

---

## 1. Executive Summary & Strategic Intent

Phase 0 established basic execution: spawning a target `.exe` via Windows `CreateProcessW`.

Phase 10 does **not** seek to invent an over-engineered, fragile plugin framework. Instead, its mission is to **harden, generalize, and standardize execution backends**:
- Support native binaries, PowerShell scripts (`.ps1`), and Python scripts (`.py`) as first-class citizens.
- Pass rich, structured context (from Phases 2 & 3) directly into the executed script via standardized environment variables and JSON payloads.
- Guarantee robust background execution: asynchronous dispatch, process detachment, stdout/stderr diagnostic capture, timeout enforcement, and clean error handling without blocking the WinPie UI thread.

---

## 2. Executor Architecture & Backend Taxonomy

```text
                             Resolved Command Node
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │       Execution Manager       │
                       │ (Environment & Payload Builder)│
                       └───────────────┬───────────────┘
                                       │
         ┌─────────────────────────────┼─────────────────────────────┐
         │                             │                             │
         ▼                             ▼                             ▼
┌──────────────────┐          ┌──────────────────┐          ┌──────────────────┐
│ Native Executable│          │ PowerShell (.ps1)│          │   Python (.py)   │
│ Backend          │          │ Backend          │          │ Backend          │
├──────────────────┤          ├──────────────────┤          ├──────────────────┤
│ Direct spawn via │          │ EncodedCommand / │          │ Dispatches to    │
│ CreateProcessW   │          │ Runspace worker  │          │ python.exe / venv│
└────────┬─────────┘          └────────┬─────────┘          └────────┬─────────┘
         │                             │                             │
         └─────────────────────────────┼─────────────────────────────┘
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │    Process Supervisor Thread   │
                       ├───────────────────────────────┤
                       │ - Async process monitoring    │
                       │ - Stdout/Stderr ring buffer   │
                       │ - Timeout watchdog            │
                       │ - Diagnostics & Toast emitter │
                       └───────────────────────────────┘
```

---

## 3. Context Injection & Environment Contract

When an action executes, WinPie constructs a specialized execution context populated with environment variables derived from the `InvocationContext`:

| Environment Variable | Source | Description |
| :--- | :--- | :--- |
| `WINPIE_INVOCATION_ID` | Phase 2 | Unique integer ID of the triggering gesture. |
| `WINPIE_TARGET_HWND` | Phase 2 | Window handle of foreground application (hex string). |
| `WINPIE_TARGET_PROCESS`| Phase 2 | Full path to the active executable. |
| `WINPIE_TARGET_PID` | Phase 2 | Process ID of the active window. |
| `WINPIE_MONITOR_DPI` | Phase 2 | DPI scaling factor of the active display. |
| `WINPIE_CLIPBOARD_TEXT`| Phase 3 | Text contents of the clipboard (if text). |
| `WINPIE_SELECTED_PATHS`| Phase 3 | Semicolon-delimited list of selected files in Explorer. |
| `WINPIE_TERMINAL_CWD` | Phase 3 | Current working directory of active shell. |
| `WINPIE_CONTEXT_JSON` | Phase 2/3 | Serialized JSON file path containing the entire capability bag. |

### 3.1. Structured JSON Payload Passing
For complex scripts, passing huge strings via command-line arguments risks hitting Windows `MAX_PATH` or command-line length limits ($32,767$ characters). WinPie writes the full invocation payload to a temporary JSON file (`%TEMP%\winpie_ctx_<id>.json`) and passes `--winpie-context <path>`, ensuring reliable data delivery.

---

## 4. Backend Specifications

### 4.1. Native Executable Backend (`backend: native`)
- **Direct Execution:** Uses `std::process::Command` with Windows creation flags `DETACHED_PROCESS` or `CREATE_NEW_PROCESS_GROUP`.
- **Zero Window Theft:** Disables window creation (`CREATE_NO_WINDOW`) for background utilities, or launches GUI applications detached from WinPie's process hierarchy.
- **Process Independence:** Child processes continue running even if WinPie is restarted or terminated.

### 4.2. PowerShell Script Backend (`backend: powershell`)
- **Invocation Command:** Launches `powershell.exe` (or `pwsh.exe` if detected on `PATH`) with flags:
  ```powershell
  pwsh.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <script.ps1>
  ```
- **Error Trapping:** Scripts run in non-interactive mode. Unhandled PowerShell script errors are caught, formatting the error message to WinPie's diagnostic ring.

### 4.3. Python Script Backend (`backend: python`)
- **Interpreter Resolution:**
  1. Project-configured virtualenv (e.g. `python_path: ".venv/Scripts/python.exe"`).
  2. Active target application embedded Python (e.g. Blender's bundled `blender/bin/python.exe`).
  3. System `python.exe` on `PATH`.
- **Standard Execution Contract:** Script receives arguments and environment automatically.

---

## 5. Process Supervision, Timeouts & Diagnostics

1. **Non-Blocking UI Thread:** Execution dispatch occurs on a dedicated background worker thread pool. The GUI message pump thread returns to `IDLE` in $< 1\,\text{ms}$.
2. **Timeout Enforcement:** Commands define optional timeouts (default: $30\,\text{seconds}$ for scripts, $\infty$ for launched GUI apps). If a background script hangs, the supervisor terminates the process tree (`GenerateConsoleCtrlEvent` followed by `TerminateProcess`).
3. **Diagnostic Ring Buffers:** The supervisor captures the last $100$ lines of `stdout` and `stderr`. If a script exits with a non-zero code, WinPie logs the error and optionally raises a red diagnostic toast notification.

---

## 6. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-EXEC-001` | Executor | Action dispatch must be strictly non-blocking; the GUI message pump must return to `IDLE` within $\le 1.0\,\text{ms}$. |
| `INV-EXEC-002` | Executor | Child processes must be spawned detached; a crash or exit of WinPie must not kill launched target applications. |
| `INV-EXEC-003` | Executor | Environment variable construction must not allocate memory or lock resources on the GUI thread. |
| `INV-EXEC-004` | Executor | Temporary context JSON payload files must be cleaned up automatically after script termination. |

---

## 7. Verification & Acceptance Criteria

1. [ ] **Environment Injection Test:** Execute a test PowerShell script via WinPie over Explorer; assert `$env:WINPIE_TARGET_PROCESS` matches `explorer.exe` and `$env:WINPIE_SELECTED_PATHS` contains test files.
2. [ ] **Timeout Termination Test:** Launch a Python script containing `time.sleep(100)` configured with a $2\,\text{s}$ timeout; assert supervisor terminates process at $2\,\text{s}$ and logs timeout error.
3. [ ] **Detached Lifetime Test:** Launch `notepad.exe` via WinPie; kill `winpie.exe` process; assert `notepad.exe` remains running without parent process dependencies.
