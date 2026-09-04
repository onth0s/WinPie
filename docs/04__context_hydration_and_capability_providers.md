# Phase 3: Context Hydration & Capability Providers

**Document ID:** `04__context_hydration_and_capability_providers.md`  
**Roadmap Phase:** Phase 3  
**Status:** Planned  
**Dependencies:** Phase 2 (`03__context_detection.md`), COM / UI Automation APIs, Shell APIs (`IShellWindows`), Terminal ConPTY / OSC 7 protocols  

---

## 1. Executive Summary & Strategic Intent

Phase 2 captured the raw, low-level OS snapshot (window handle, PID, executable name, raw clipboard format codes). Phase 3 translates this low-level snapshot into **semantic capabilities**: rich, structured domain knowledge about what the user is actually doing.

Raw detection tells WinPie: *"The active process is `explorer.exe`."*  
Context hydration tells WinPie: *"The user is in `D:\Projects\3D\Assets` with three `.fbx` models selected."*

### Architectural Tenet: Provider Isolation & Graceful Degradation
> **Capability providers must execute independently with strict isolation. A failure, timeout, or missing plugin in one provider (e.g., a stalled COM query to Photoshop or an unresponsive Blender IPC socket) must never abort invocation, freeze the UI message loop, or delay radial presentation.**

---

## 2. Component Architecture & Hydration Pipeline

The hydration engine models each domain as an isolated `CapabilityProvider` implementation:

```text
                        ┌───────────────────────────────┐
                        │ Phase 2: InvocationContext    │
                        │ (Raw Immutable OS Snapshot)   │
                        └───────────────┬───────────────┘
                                        │
                                        ▼
                        ┌───────────────────────────────┐
                        │      Hydration Dispatcher     │
                        └───────┬───────────────┬───────┘
                                │               │
          ┌─────────────────────┼───────────────┼─────────────────────┐
          │                     │               │                     │
          ▼                     ▼               ▼                     ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│ ExplorerProvider │  │ TerminalProvider │  │  VSCodeProvider  │  │ClipboardProvider │
│ (IShellWindows)  │  │ (CWD / OSC 7)    │  │ (Workspace/Doc)  │  │ (MIME / Content) │
└─────────┬────────┘  └─────────┬────────┘  └─────────┬────────┘  └─────────┬────────┘
          │                     │               │                     │
          └─────────────────────┼───────────────┼─────────────────────┘
                                │ Results (Bounded Timeouts)
                                ▼
                        ┌───────────────────────────────┐
                        │    Hydrated CapabilitySet     │
                        │ (Typed Semantic Attributes)   │
                        └───────────────────────────────┘
```

---

## 3. Capability Provider Trait & Life Cycle

All providers conform to a standardized, non-blocking contract:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderResult<T> {
    Available(T),
    NotApplicable,
    Timeout,
    Error(String),
}

pub trait CapabilityProvider: Send + Sync {
    /// Unique identifier for this provider (e.g. "provider.shell.explorer")
    fn id(&self) -> &'static str;

    /// Evaluates if this provider is relevant based on the raw invocation context
    fn is_applicable(&self, context: &InvocationContext) -> bool;

    /// Hydrates semantic capabilities within a strict deadline (e.g. 5ms)
    fn hydrate(&self, context: &InvocationContext, deadline: Instant) -> CapabilityBag;
}
```

---

## 4. Built-In Domain Providers

### 4.1. Windows Explorer Provider (`ExplorerProvider`)
- **Mechanism:** Leverages Windows Shell COM interfaces (`IShellWindows`, `IShellFolderViewDual`, `IShellItemArray`).
- **Extracted Attributes:**
  - `shell.current_folder`: Normalized filesystem path of the active Explorer window.
  - `shell.selected_items`: List of selected file/folder paths.
  - `shell.selected_extensions`: Unique set of lowercase extensions (e.g. `[".blend", ".png", ".rs"]`).
  - `shell.selection_count`: Total number of selected items.
- **Fault Safety:** COM operations are dispatched on an STA worker pool with a $5.0\,\text{ms}$ cancellation timeout to prevent hangs on network shares (SMB/WebDAV).

### 4.2. Terminal & Console Provider (`TerminalProvider`)
- **Supported Hosts:** Windows Terminal (`wt.exe`), PowerShell (`powershell.exe`, `pwsh.exe`), Command Prompt (`cmd.exe`).
- **Mechanism:**
  - Title parsing (extracting `user@host: ~/path` or Windows Terminal tab titles).
  - OSC 7 / OSC 9;9 Shell Integration sequence tracking if available.
  - Win32 Console API fallback (`GetConsoleTitleW`, parent process environment inspection).
- **Extracted Attributes:**
  - `terminal.cwd`: Current working directory of the shell session.
  - `terminal.shell_type`: Identified shell binary (`pwsh`, `bash`, `cmd`).
  - `terminal.in_git_repo`: Boolean flag if `.git` directory exists in CWD.

### 4.3. Developer IDE Provider (`VSCodeProvider`)
- **Mechanism:** Inspects VS Code window title patterns (`"[FileName] - [Workspace] - Visual Studio Code"`), queries local recent workspace states, or interfaces with lightweight local extension socket if active.
- **Extracted Attributes:**
  - `editor.active_file`: Absolute path of the focused source file.
  - `editor.file_extension`: e.g. `.rs`, `.py`, `.ts`, `.md`.
  - `editor.workspace_root`: Root folder of the open project.

### 4.4. Creative Suite Providers (Blender / Photoshop)
- **Mechanism:**
  - Blender: Reads window title (`Blender* [D:\Art\character.blend]`), queries local Blender IPC loopback port if enabled.
  - Photoshop: Reads active document name and mode from window title or COM automation.
- **Extracted Attributes:**
  - `app.document_path`: Open `.blend` / `.psd` file path.
  - `app.is_dirty`: Whether the current document has unsaved edits.

### 4.5. Clipboard Capability Provider (`ClipboardProvider`)
- **Mechanism:** Evaluates format flags captured in Phase 2 without large data copies.
- **Extracted Attributes:**
  - `clipboard.has_text`: Boolean.
  - `clipboard.has_files`: Boolean (via `CF_HDROP`).
  - `clipboard.file_extensions`: Extracted extensions of files currently on clipboard.
  - `clipboard.has_image`: Boolean (`CF_BITMAP`, `CF_DIBV5`).
  - `clipboard.is_url`: Regex evaluation on plain text preview.

---

## 5. Formal Invariants & Isolation Guarantees

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-PROV-001` | Providers | A failure or panic in any `CapabilityProvider` must not terminate or halt the hydration pipeline. |
| `INV-PROV-002` | Providers | Hydration must enforce a hard global deadline of $\le 5.0\,\text{ms}$. Providers exceeding their individual allocation are canceled. |
| `INV-PROV-003` | Providers | Hydration providers must never modify the target application state (strictly read-only introspection). |
| `INV-PROV-004` | Providers | All hydrated capabilities are stored in a typed, immutable `CapabilityBag` passed to Phase 7 Command Resolution. |

---

## 6. Edge Cases & Resilience Strategy

1. **Unresponsive Network Shares in Explorer:** If an Explorer window is viewing a disconnected network share (`\\192.168.1.50\share`), COM calls can freeze for 30 seconds. WinPie executes COM queries on an isolated thread with an immediate $5\,\text{ms}$ timeout guard, falling back to `shell.current_folder: Unknown`.
2. **Rapid Selection Changes:** Because hydration operates against the static HWND snapshot from Phase 2, background clicks during radial display cannot corrupt the hydrated data.
3. **Huge Clipboard Data:** If the user copied a 500MB image or 10,000 files, the clipboard provider only queries file count and header metadata without loading large buffers into WinPie memory.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Explorer Selection Test:** Select 3 files in File Explorer; trigger WinPie; verify `shell.selected_items` contains all 3 paths and extensions are correctly partitioned.
2. [ ] **Provider Timeout Enforcement Test:** Create a mock provider that intentionally sleeps for $100\,\text{ms}$; trigger WinPie; assert the hydration engine terminates at exactly $5\,\text{ms}$ and overlay displays without delay.
3. [ ] **Graceful Degradation Test:** Disconnect local network; trigger WinPie over hung network paths; verify UI launches smoothly with fallback capabilities.
