# Phase 2: Context Detection

**Document ID:** `03__context_detection.md`  
**Roadmap Phase:** Phase 2  
**Status:** Planned  
**Dependencies:** Phase 0 (`01__native_radial_interaction_primitive.md`), Phase 1 (`02__formalize_interaction_contract.md`), Win32 OS Introspection APIs (`GetForegroundWindow`, `GetWindowThreadProcessId`, `QueryFullProcessImageNameW`, `MonitorFromPoint`, `OpenClipboard`)  

---

## 1. Executive Summary & Strategic Intent

Phase 2 introduces the foundational contextual primitive of WinPie: **Point-in-Time Environmental Introspection**.

Before Phase 2, WinPie operates as a static radial launcher whose menu items are fixed at compile or startup time. In Phase 2, WinPie acquires the ability to observe the exact operating system state at the moment the activation gesture occurs.

### The Governing Principle: The Frozen Snapshot Invariant
> **Context is captured atomically once at invocation trigger, then frozen as an immutable snapshot. All subsequent resolution, layout calculations, and action dispatch operate exclusively against that snapshot.**

This invariant guarantees that:
- WinPie never experiences race conditions if the foreground window changes, minimizes, or closes during radial navigation.
- Long-running clipboard operations or window title changes do not cause menu flicker or mid-gesture mutation.
- Resolution logic remains purely functional and deterministic: `f(Snapshot) -> ResolvedMenu`.

---

## 2. Invocation Context Data Model

At the microsecond of `Win+Esc` qualification, the engine constructs an immutable `InvocationContext` structure:

```rust
pub struct InvocationContext {
    /// Unique monotonic invocation ID and high-resolution timestamp
    pub invocation_id: u64,
    pub timestamp_qpc: i64,

    /// Foreground window handle and metadata
    pub window: WindowSnapshot,

    /// Owning process metadata
    pub process: ProcessSnapshot,

    /// Physical cursor coordinates at trigger
    pub cursor_pos: PhysicalPoint,

    /// Monitor containing the cursor (including DPI and work area)
    pub monitor: MonitorSnapshot,

    /// Clipboard metadata and light payload signatures
    pub clipboard: ClipboardSnapshot,

    /// Flags indicating which context domains succeeded/failed capture
    pub captured_flags: ContextDomainFlags,
}
```

```text
InvocationContext (Immutable Snapshot)
├── window
│   ├── hwnd: HWND
│   ├── class_name: String
│   ├── window_title: String
│   └── window_rect: RECT
├── process
│   ├── pid: u32
│   ├── image_name: String           ("blender.exe", "Code.exe", "wt.exe")
│   ├── image_path: PathBuf          ("C:\Program Files\Blender Foundation\...")
│   └── is_elevated: bool
├── cursor
│   └── physical_pos: (i32, i32)
├── monitor
│   ├── hmonitor: HMONITOR
│   ├── virtual_rect: RECT
│   ├── work_area: RECT
│   └── dpi: u32                     (e.g., 96, 120, 144, 192)
└── clipboard
    ├── sequence_number: u32
    ├── available_formats: Vec<u32>  (CF_TEXT, CF_HDROP, custom registered formats)
    ├── text_preview: Option<String> (truncated preview, e.g. max 256 chars)
    └── file_paths: Option<Vec<PathBuf>>
```

---

## 3. Subsystem Specifications & Capture Pipeline

The capture sequence must execute with extreme velocity ($< 2.0\,\text{ms}$) immediately preceding overlay display:

```text
[Win+Esc Trigger Qualified]
             │
             ▼
┌─────────────────────────────────────────┐
│     Step 1: Capture Window & HWND       │ ──► GetForegroundWindow()
│                                         │ ──► GetWindowThreadProcessId()
└────────────────────┬────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────┐
│     Step 2: Inspect Process Metadata    │ ──► OpenProcess(PROCESS_QUERY_LIMITED_INFO)
│                                         │ ──► QueryFullProcessImageNameW()
└────────────────────┬────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────┐
│     Step 3: Resolve Physical Monitor    │ ──► MonitorFromPoint(cursor_pos)
│                                         │ ──► GetMonitorInfoW(), GetDpiForMonitor()
└────────────────────┬────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────┐
│     Step 4: Probe Clipboard Signatures  │ ──► GetClipboardSequenceNumber()
│                                         │ ──► Non-blocking format interrogation
└────────────────────┬────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────┐
│     Freeze Immutable InvocationContext  │ ──► Handed to downstream Resolver & Overlay
└─────────────────────────────────────────┘
```

### 3.1. Window & Process Inspection
- `GetForegroundWindow()` captures the active window handle. If `HWND` is NULL (e.g., during secure desktop lock screen, UAC prompt, or task switching), WinPie falls back to a special `SystemDesktopContext`.
- Process executable identification utilizes `PROCESS_QUERY_LIMITED_INFORMATION`. This grants permission to read the process image name even for elevated processes running under different credentials, preventing access-denied faults.
- Window class name (`GetClassNameW`) and window text (`GetWindowTextW` or `WM_GETTEXT` with non-hanging timeouts) are captured.

### 3.2. Physical Geometry & Monitor Context
- Coordinates are captured via `GetCursorPos()` in unscaled physical desktop space.
- `MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST)` identifies the active physical monitor.
- Work area rect (`rcWork`) is extracted from `MONITORINFO` to ensure radial overlay clamp guards prevent rendering off-screen edges.
- Per-monitor DPI is queried via `GetDpiForMonitor()` to inform Phase 4 and Phase 5 rendering dimensions.

### 3.3. Clipboard Snapshot
- To prevent locking contention on the system clipboard, WinPie first inspects `GetClipboardSequenceNumber()`. If unchanged from a cached read, cached metadata is reused.
- When querying clipboard formats (`CountClipboardFormats`, `EnumClipboardFormats`), WinPie attempts `OpenClipboard(NULL)` with a strict retry limit ($< 1\,\text{ms}$).
- If another application currently holds the clipboard lock (e.g. an ongoing copy operation), WinPie immediately degrades gracefully: `clipboard: None` or `ClipboardSnapshot::Locked`, avoiding any UI stall.

---

## 4. Formal Invariants Introduced

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-CTX-001` | Context | `InvocationContext` is instantiated exactly once per gesture and is strictly immutable (`&InvocationContext`). |
| `INV-CTX-002` | Context | Context capture operations must complete within $\le 2.0\,\text{ms}$ total wall-clock time. |
| `INV-CTX-003` | Context | Clipboard lock acquisition failure must never block or cancel radial activation. |
| `INV-CTX-004` | Context | If foreground window is NULL, protected, or destroyed during capture, the context gracefully degrades to `DesktopFallback`. |
| `INV-CTX-005` | Context | No external process handles or clipboard locks may remain open after `InvocationContext` construction finishes. |

---

## 5. Edge Cases & Failure Domains

1. **UAC Elevated Windows:** If the foreground window is running elevated (Administrator) and WinPie is non-elevated, `OpenProcess(PROCESS_ALL_ACCESS)` fails with `ERROR_ACCESS_DENIED`. WinPie strictly queries `PROCESS_QUERY_LIMITED_INFORMATION`, allowing safe image path retrieval across integrity boundaries.
2. **Hung Foreground Window:** If the foreground application is unresponsive, calling synchronous Win32 APIs like `SendMessageTimeout(WM_GETTEXT)` could stall. All string extractions use bounded timeouts ($\le 20\,\text{ms}$) or asynchronous thread introspection.
3. **Multi-Monitor Edge Clamping:** If `Win+Esc` is pressed at the very edge of a monitor (e.g. $(0, 0)$), the monitor work area bounds in `InvocationContext` inform the overlay to offset its drawing center so the wheel remains 100% visible.

---

## 6. Verification & Acceptance Criteria

1. [ ] **Process Identification Test:** Trigger WinPie over 10 distinct applications (Command Prompt, Windows Terminal, Notepad, VS Code, Chrome, Blender, Explorer, Task Manager, Slack, Sublime); verify 100% accurate binary name and path detection.
2. [ ] **Clipboard Concurrency Test:** Run a background loop acquiring and holding the Win32 clipboard lock; trigger WinPie; assert activation succeeds within $2\,\text{ms}$ with clipboard marked as `Locked`.
3. [ ] **Latency Budget Gate:** Benchmark context snapshot creation under CI; assert p99 execution time $\le 1.5\,\text{ms}$.
4. [ ] **Immutability Enforcement:** Rust type system verifies `InvocationContext` contains no interior mutability (`UnsafeCell`, `RefCell`, `Mutex`) and is safely shareable across threads (`Send + Sync`).
