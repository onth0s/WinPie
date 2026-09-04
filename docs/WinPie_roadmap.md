# WinPie Roadmap

### 0. Native Radial Interaction Primitive ✅

* Global `Win+Esc` activation
* Low-level keyboard/mouse hooks
* Input swallowing / shell suppression
* Focus-preserving layered overlay
* 8-way radial geometry
* Deadzone + bounds handling
* Hover feedback
* Win-release / LMB commit
* RMB / invalid-position cancellation
* Multi-monitor + physical-coordinate handling
* Re-press guard
* Deterministic geometry/FSM tests

### 1. Formalize Interaction Contract

Make the implementation and specification completely isomorphic.

* Finalize `INVARIANTS.yaml`
* Finalize `STATE_MACHINE.yaml`
* Finalize `GEOMETRY.yaml`
* Document actual commit/cancel semantics
* Behavioral tests for overlay/input transparency
* Benchmark claims rather than qualitative performance language

### 2. Context Detection

The big next primitive.

Capture an immutable snapshot at invocation:

```text
Invocation
├── foreground window
├── process
├── cursor
├── monitor
├── clipboard
└── available context
```

The important property:

> **Context is captured once for an invocation, then resolution operates on that snapshot.**

### 3. Context Hydration & Capability Providers

Turn detection into useful information.

```text
Context
├── Window
├── Process
├── Monitor
├── Clipboard
├── Selection
├── Shell
├── Document
└── Application-specific providers
```

Examples:

* Explorer → selected files
* Terminal → cwd
* VS Code → workspace/file/selection
* Photoshop → document state
* Blender → document/selection/context
* Clipboard → text/path/image/3D asset/etc.

Providers should fail independently. Missing Blender-specific context shouldn't kill the entire invocation.

### 4. Context-Aware Visual Identity

Make the UI communicate detected context instantly.

```text
ContextAppearance
├── accent_color
├── icon
└── optional metadata
```

For example:

```text
BLENDER
  ↓
orange accent + Blender icon
```

The color becomes **semantic state**, not merely styling.

### 5. Native SVG Icon Pipeline

Pure Rust asset ingestion.

```text
SVG
 ↓
parse/rasterize
 ↓
RGBA bitmap
 ↓
cache
 ↓
Win32 layered rendering
```

Requirements:

* Arbitrary SVG ingestion
* Configurable icon sources
* Pre-rasterization
* DPI-aware raster sizes
* Cached RGBA assets
* No SVG work in interaction hot paths

This also gives you a generic icon system rather than a Blender-specific hack.

---

# 6. Command Graph

This is where WinPie stops being "a pie menu."

Every node has the same conceptual shape:

```text
CommandNode
├── executable?
└── children?
```

So:

```text
Gesture
  ↓
Node
  ├── executable → execute
  └── children   → open child surface
```

Arbitrary depth:

```text
Pie
 └─ NE
     └─ A
         └─ S
             └─ D
                 └─ execute
```

No `SubmenuLevel1`, `SubmenuLevel2`, etc.

---

# 7. Contextual Command Resolution

Commands become declarative capabilities rather than hardcoded UI entries.

```yaml
command:
  id: blender.open_clipboard_asset
  requires:
    - blender
    - clipboard
    - clipboard.path
    - clipboard.extension:.blend
```

Resolution:

```text
InvocationContext
        ↓
Capabilities
        ↓
Command Graph
        ↓
Eligible Commands
        ↓
Radial Surface
```

This is the point where:

> **the same gesture can mean different things depending on context.**

---

# 8. Massive Command Topology

Now exploit the spatial nature of the system.

Something like:

```text
Context
└── 8 primary directions
    ├── 20+ operations
    ├── 20+ operations
    ├── ...
    └── 20+ operations
```

≈ **8 × 20-something operations per context** without requiring a flat list of 160 shortcuts.

The hierarchy itself becomes the mnemonic.

---

# 9. Left-Hand Shortcut / Chord Layer

Add the keyboard equivalent of the spatial language.

The important architectural constraint:

```text
            ┌── Radial gesture
Input ──────┼── Mouse gesture
            └── Keyboard chord
                    ↓
             Command Graph
```

Different input modalities should resolve into the **same command nodes**.

---

# 10. Scriptable Execution Backends

You already have **arbitrary command execution** here, so the roadmap isn't "implement execution."

Instead:

### Harden and generalize it.

Support the existing arbitrary execution model as a proper execution backend:

```text
Command
   ↓
Executor
   ├── native executable
   ├── PowerShell / .ps1
   ├── Python
   └── future backends
```

The key work is:

* structured argument/context passing
* environment construction
* stdout/stderr handling
* exit-code semantics
* timeout/error handling
* cancellation semantics
* execution diagnostics

No need to invent an elaborate plugin framework.

---

# 11. Unified Contextual Command Surface

At this point everything converges:

```text
                    INPUT
                      ↓
             INVOCATION CONTEXT
                      ↓
            CAPABILITY PROVIDERS
                      ↓
               COMMAND GRAPH
                      ↓
                 RESOLUTION
                      ↓
                 EXECUTOR
              ↙      ↓      ↘
           Rust    Python   PS1
```

And the radial UI becomes one **view** over that system.

Context supplies meaning.

Geometry supplies syntax.

The command graph supplies structure.

The executor supplies action.

---

# 12. Hardening & Performance

Final engineering pass:

* Hook robustness
* Message-pump failure recovery
* Context-provider isolation
* Command resolution latency
* SVG/raster cache performance
* Overlay redraw benchmarks
* Process-launch behavior
* Script execution timeouts
* Resource cleanup
* Multi-monitor/DPI edge cases
* Long-running stability
* Stress testing
* Packaging / startup behavior
