# Phase 1: Formalize Interaction Contract

**Document ID:** `02__formalize_interaction_contract.md`  
**Roadmap Phase:** Phase 1  
**Status:** Ready for Execution  
**Dependencies:** Phase 0 (`01__native_radial_interaction_primitive.md`), YAML Schema Parsers, Criterion.rs / Microbenchmark Framework  

---

## 1. Executive Summary & Strategic Intent

Phase 1 elevates WinPie from an empirical prototype to a mathematically verifiable interaction system. The guiding mission is **Specification-Implementation Isomorphism**: every runtime behavior, state transition, geometrical boundary, and performance guarantee must be formally declared in machine-readable specifications (`.yaml`) and verified via automated test harnesses.

Rather than relying on qualitative assertions (e.g., "fast", "fluid", "transparent"), Phase 1 defines strict quantifiable bounds for latency, jitter, input swallowing, and focus stability.

---

## 2. Specification Formalization

WinPie specifications are maintained in the [`spec/`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/spec) directory and serve as the single source of truth:

```text
                               ┌────────────────────────┐
                               │   spec/INVARIANTS.yaml │
                               └───────────┬────────────┘
                                           │
                               ┌───────────┴────────────┐
                               │ spec/STATE_MACHINE.yaml│
                               └───────────┬────────────┘
                                           │
                               ┌───────────┴────────────┐
                               │   spec/GEOMETRY.yaml   │
                               └───────────┬────────────┘
                                           │
                     ┌─────────────────────┴─────────────────────┐
                     ▼                                           ▼
         Automated Behavioral Tests                  Runtime Contract Checks
        (Synthetic Win32 Event Sim)                 (Debug Invariant Assertions)
```

### 2.1. `INVARIANTS.yaml` Finalization
Formal invariants are partitioned into four strict categories:
1. **`input`**: Swallowing contracts, zero-leak guarantees, re-press guards, and modifier isolation.
2. **`state`**: Transition completeness, absence of deadlock, guaranteed terminal states (`IDLE` / `WAIT_RELEASE`).
3. **`geometry`**: Strict determinism across 2D continuous space, non-overlapping sectors, deadzone invariance.
4. **`focus`**: Window focus preservation, absence of foreground thread hijacking, z-order stability.

### 2.2. `STATE_MACHINE.yaml` Formalization
Defines the state space $S = \{\text{IDLE}, \text{ACTIVE}, \text{WAIT\_RELEASE}\}$, input alphabet $\Sigma$, transition function $\delta: S \times \Sigma \to S$, and side-effect mapping $\omega: S \times \Sigma \to E$.
- Disallows undefined transitions: Any event outside $\Sigma$ in state $S$ must evaluate to an explicit `NOOP` or `CANCEL` fallback.
- Explicitly models unified cancellation: left-click out-of-bounds, right-click anywhere, and system errors share an identical teardown pipeline.

### 2.3. `GEOMETRY.yaml` Formalization
- Standardizes polar coordinate conversion, boundary tie-breaking rules, and angle normalization ($[0, 360^\circ)$).
- Declares exact angle tolerances and sector indexing order (clockwise starting at North $= 0^\circ$).

---

## 3. Formal Invariant Catalog

The following core invariants are formally codified in [`spec/INVARIANTS.yaml`](file:///c:/Users/Leonardo/001/00__DEV/WinPie/spec/INVARIANTS.yaml):

| Invariant ID | Category | Severity | Guarantee Statement |
| :--- | :--- | :--- | :--- |
| `INV-INPUT-001` | Input | Critical | `Win+Esc` activates the wheel exactly once per qualifying gesture. |
| `INV-INPUT-002` | Input | Critical | While `ACTIVE`, mouse-button transitions are never forwarded to the foreground application. |
| `INV-INPUT-003` | Input | High | Mouse movement remains observable by WinPie while OS pointer motion continues smoothly. |
| `INV-INPUT-004` | Input | Critical | The resolving left-button-down event is swallowed prior to OS application delivery. |
| `INV-INPUT-005` | Input | Critical | Resolving button events are never re-emitted after returning to `IDLE`. |
| `INV-INPUT-006` | Input | Critical | No race condition or input-processing window exists where resolving clicks can escape. |
| `INV-INPUT-007` | Input | Critical | If activation keys remain physically held upon resolution, WinPie enters `WAIT_RELEASE`. |
| `INV-STATE-001` | State | Critical | `ACTIVE` always terminates in `IDLE` or `WAIT_RELEASE` via commit, cancel, or fatal error. |
| `INV-STATE-002` | State | High | `Esc` release is a no-op while `ACTIVE`. `Win` release commits hovered sector or cancels. |
| `INV-STATE-003` | State | Critical | Left-click inside deadzone or outside radius cancels and terminates `ACTIVE`. |
| `INV-STATE-004` | State | Normal | Hover selection is visual preselection only; it cannot commit without explicit resolution. |
| `INV-GEOMETRY-001`| Geometry | Critical | The wheel contains exactly 8 equal directional sectors ($45^\circ$ each). |
| `INV-GEOMETRY-002`| Geometry | Critical | Sector classification is strictly deterministic for all finite cursor coordinates. |
| `INV-GEOMETRY-003`| Geometry | High | Coordinates inside deadzone ($r \le R_{\text{dead}}$) or outside ($r > R_{\text{max}}$) yield no sector. |
| `INV-FOCUS-001` | Focus | Critical | Overlay creation, display, and dismissal never alter `GetForegroundWindow`. |

---

## 4. Behavioral Testing Harness

To verify these contracts without manual human clicking, Phase 1 establishes an automated Win32 event injection test harness:

```text
┌─────────────────────────────────────────────────────────────┐
│                 Automated Test Controller                   │
│  - Launches WinPie instance in mock headless or windowed mode│
│  - Captures foreground window handle (HWND) before test     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│            Synthetic Input Injection Engine                 │
│  - Injects SendInput keyboard sequences (Win+Esc)           │
│  - Injects synthetic mouse moves and button clicks          │
│  - Verifies event swallowing via downstream hook monitor   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Assertion Verification                     │
│  1. Assert GetForegroundWindow() == Initial HWND            │
│  2. Assert zero clicks delivered to dummy target window      │
│  3. Assert FSM state matches expected sequence              │
│  4. Assert execution triggered only upon valid commit        │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Performance Benchmark Gates

Qualitative performance claims are replaced with concrete benchmark gates executed in continuous integration:

| Metric | Target Gate | Upper Bound (Failure) | Measurement Method |
| :--- | :--- | :--- | :--- |
| **Hook Callback Execution** | $< 0.15\,\text{ms}$ | $> 0.50\,\text{ms}$ | `std::time::Instant` timer inside `WH_KEYBOARD_LL` / `WH_MOUSE_LL` |
| **Trigger-to-Overlay Latency** | $< 4.0\,\text{ms}$ | $> 8.0\,\text{ms}$ | High-resolution timestamp from `Win+Esc` down to first `UpdateLayeredWindow` |
| **Sector Classification** | $< 50\,\text{ns}$ | $> 250\,\text{ns}$ | Microbenchmark of `Sector::from_point` across $1,000,000$ points |
| **Frame Render Time** | $< 1.5\,\text{ms}$ | $> 4.0\,\text{ms}$ | GDI+ / memory buffer rasterization for 8-slice wheel |
| **Heap Allocations in Hot Path** | **0 bytes** | $> 0\text{ bytes}$ | Custom allocator tracking during `ACTIVE` hover tracking |

---

## 6. Acceptance Criteria

1. [ ] **Specification Validation:** Automated test reads `INVARIANTS.yaml`, `STATE_MACHINE.yaml`, and `GEOMETRY.yaml`, confirming that code definitions match the specs 100%.
2. [ ] **Focus Test:** Automated script invokes WinPie 100 consecutive times over Notepad and Terminal; asserts foreground focus is never stolen.
3. [ ] **Input Leak Test:** Automated script clicks mouse buttons rapidly during activation and dismissal; asserts zero clicks register on underlying window controls.
4. [ ] **Benchmark Verification:** All performance gates pass on standard reference hardware (x64 Windows 10/11).
