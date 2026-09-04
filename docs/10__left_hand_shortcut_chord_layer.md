# Phase 9: Left-Hand Shortcut / Chord Layer

**Document ID:** `10__left_hand_shortcut_chord_layer.md`  
**Roadmap Phase:** Phase 9  
**Status:** Planned  
**Dependencies:** Phase 0 Input Hook (`input/keyboard.rs`), Phase 6 (`07__command_graph.md`), Phase 7 (`08__contextual_command_resolution.md`)  

---

## 1. Executive Summary & Strategic Intent

Phase 9 introduces the keyboard equivalent of WinPie's spatial interaction language. While mouse gestures excel when one hand is already on the mouse or pen tablet, users operating purely via keyboard require an equally rapid, low-cognitive-load input modality.

### The Architectural Invariant: Modality Equivalence
> **Different input modalities—radial mouse gestures, spatial flicks, and left-hand keyboard chords—must resolve into the exact same underlying Command Graph nodes.**

Keyboard chording is not a separate shortcut manager; it is an alternative **syntax** addressing the same semantic capability tree.

```text
            ┌── Radial Gesture (Direction Vector: N, E, SW)
            │
Input ──────┼── Mouse Stroke (Continuous flick curve)
            │
            └── Left-Hand Chord (Key Sequence: [Caps]+[W], [E])
                    │
                    ▼
             Command Graph (Phase 6 & 7)
                    │
                    ▼
             Executable Action (Phase 10)
```

---

## 2. Left-Hand Ergonomics & Spatial Cluster

Right-handed users frequently keep their right hand on a mouse or stylus, while their left hand rests on the home row (`WASD` / `QWEASD` / `Tab-Caps-Shift`). Phase 9 optimizes exclusively for **left-hand single-hand chording**:

```text
┌─────┬─────┬─────┬─────┬─────┐
│ Tab │  Q  │  W  │  E  │  R  │  ──► Top Row: Build, Navigate, Run
├─────┼─────┼─────┼─────┼─────┤
│Caps │  A  │  S  │  D  │  F  │  ──► Home Row: Primary Actions, Select, Transform
├─────┼─────┼─────┼─────┼─────┤
│Shift│  Z  │  X  │  C  │  V  │  ──► Bottom Row: Tools, Toggles, System
└─────┴─────┴─────┴─────┴─────┘
```

### 2.1. Spatial Correspondence Mapping
To leverage spatial memory across modalities, keyboard keys can map directly to radial directions:

| Key | Spatial Direction | Semantic Family |
| :--- | :--- | :--- |
| `W` | **North** | Navigate / View |
| `E` | **North-East** | Build / Run |
| `D` | **East** | File / Export |
| `C` | **South-East** | Terminal / Logs |
| `S` | **South** | Tools / Transform |
| `Z` | **South-West** | Layout / Windows |
| `A` | **West** | Edit / Modify |
| `Q` | **North-West** | History / State |

A user can invoke WinPie and either flick the mouse **North** or strike key **`W`**—both trigger the identical command node.

---

## 3. Chording & Sequence FSM

Phase 9 introduces multi-key chording within the `ACTIVE` interaction state:

```text
                  [ Win+Esc Qualified or CapsLock Hold ]
                                    │
                                    ▼
                             ┌─────────────┐
                             │ ACTIVE_ROOT │
                             └──────┬──────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │ Key Pressed (e.g. 'W' / North)│
                    ▼                               ▼
            [ Leaf Action? ]                [ Has Children? ]
                    │                               │
             (Commit & Run)                         ▼
                                            ┌─────────────┐
                                            │ACTIVE_TIER_1│
                                            └──────┬──────┘
                                                   │
                                     Key Pressed (e.g. 'D' / East)
                                                   │
                                                   ▼
                                            [ Commit & Run ]
```

### 3.1. Chording Modifiers & Sticky Keys
- **Hold-and-Release (Modal):** Holding `Caps` (remapped) activates WinPie; pressing `W` then `E` navigates tiers; releasing `Caps` executes the selection.
- **Key Sequences (Vim-style / Leader key):** Pressing `Win+Esc` arms WinPie; pressing `E` then `F` commits immediately without holding down modifiers.

---

## 4. Input Swallowing & OS Pass-Through Isolation

In accordance with Phase 0/1 invariants:
1. When WinPie is `IDLE`, keys (`Q, W, E, A, S, D...`) pass through to the OS with **zero latency** and zero interception.
2. The instant WinPie enters `ACTIVE`, `WH_KEYBOARD_LL` intercepts left-hand navigation keys and **swallows them completely** (`return 1`). Foreground applications receive no stray typing characters.
3. If an unmapped key is pressed, WinPie can either cancel cleanly or ignore the key based on configuration.
4. Upon exit to `IDLE` or `WAIT_RELEASE`, all key swallowing disarms immediately.

---

## 5. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-CHORD-001` | Input | While `ACTIVE`, mapped keyboard keys must be swallowed with 100% reliability; zero characters leak to foreground apps. |
| `INV-CHORD-002` | Contract | Any command node reachable via mouse gesture must be reachable via an equivalent keyboard chord path. |
| `INV-CHORD-003` | Performance | Keyboard chord dispatch latency from keydown to FSM state transition must be $\le 0.1\,\text{ms}$. |
| `INV-CHORD-004` | Safety | Activation hooks must never remap or interfere with emergency OS shortcuts (`Ctrl+Alt+Del`, `Win+L`). |

---

## 6. Edge Cases & Resilience Strategy

1. **Typing Fast During Invocation:** If a user is rapidly typing in an editor and accidentally hits `Win+Esc`, any keydown in flight within that millisecond could be swallowed. The engine verifies key state timestamps to ensure only key events occurring *strictly after* the invocation transition are swallowed.
2. **Caps Lock Toggle Annoyance:** If `Caps Lock` is utilized as a chord modifier, WinPie intercepts the key state directly and suppresses the hardware `CAPS_LOCK` LED toggle to prevent unwanted capitalization state changes.
3. **Key Repeat / Typematic Jitter:** Windows sends repeated `WM_KEYDOWN` messages when a key is physically held. The keyboard FSM ignores auto-repeat events (`lParam & (1 << 30)`), treating held keys as a single state.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Input Leak Test:** Open Notepad; activate WinPie; strike `W`, `A`, `S`, `D` repeatedly; dismiss WinPie; assert Notepad contains 0 text characters.
2. [ ] **Modality Parity Test:** Verify all nodes in the default configuration profile are accessible and trigger identical payloads via both mouse vector and keyboard chord.
3. [ ] **Chord Traversal Test:** Execute compound chord sequence `[W, E, D]`; assert FSM navigates tiers and triggers target leaf action.
