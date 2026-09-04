# Phase 7: Contextual Command Resolution

**Document ID:** `08__contextual_command_resolution.md`  
**Roadmap Phase:** Phase 7  
**Status:** Planned  
**Dependencies:** Phase 2 (`03__context_detection.md`), Phase 3 (`04__context_hydration_and_capability_providers.md`), Phase 6 (`07__command_graph.md`)  

---

## 1. Executive Summary & Strategic Intent

Phase 7 introduces the true cognitive power of WinPie: **Dynamic Contextual Command Resolution**.

In conventional interfaces, menus are static lists of hardcoded options. In WinPie, commands are modeled as **declarative capabilities** guarded by prerequisite predicates.

### The Defining Transformation
> **The exact same spatial gesture can mean completely different things depending on the active context.**

- Flicking **North** while in **Visual Studio Code** executes `Git: Commit & Push`.
- Flicking **North** while in **Blender** executes `Extrude Along Normals`.
- Flicking **North** while in **Windows Explorer** with 5 `.png` files selected executes `Batch Convert to WebP`.
- Flicking **North** on an empty desktop opens `Windows Terminal`.

The user trains a single primary spatial direction (e.g. "Primary Action / Export = North"), and WinPie resolves the most pertinent operation for the current environment.

---

## 2. Declarative Predicate Model

Commands declare their execution requirements via a rich predicate language:

```yaml
command:
  id: blender.import_clipboard_mesh
  label: "Import Clipboard Mesh"
  icon: "icons/blender/import.svg"
  requires:
    - context.process: "blender.exe"
    - clipboard.has_files: true
    - clipboard.extension_in: [".blend", ".obj", ".fbx", ".gltf"]
  action:
    backend: "python"
    script: "scripts/blender/import_mesh.py"

command:
  id: explorer.compress_archive
  label: "Compress to ZIP"
  icon: "icons/archive.svg"
  requires:
    - context.process: "explorer.exe"
    - shell.selection_count: "> 0"
  action:
    backend: "powershell"
    script: "Compress-Archive -Path $env:WINPIE_SELECTED_PATHS -DestinationPath archive.zip"
```

---

## 3. Resolution Engine Architecture

The resolution engine evaluates the hydrated capability set against the master command catalog to project an active radial surface:

```text
       ┌────────────────────────────┐    ┌────────────────────────────┐
       │ Phase 2: InvocationContext │    │ Phase 3: Hydrated Bag      │
       └─────────────┬──────────────┘    └─────────────┬──────────────┘
                     │                                 │
                     └────────────────┬────────────────┘
                                      │
                                      ▼
                       ┌─────────────────────────────┐
                       │   Resolution Engine (FSM)   │
                       ├─────────────────────────────┤
                       │ Predicate Matching Pipeline │
                       │ Priority & Scoring Weights  │
                       │ Directional Slot Assignment │
                       └──────────────┬──────────────┘
                                      │
                                      ▼
                       ┌─────────────────────────────┐
                       │   Resolved Radial Surface   │
                       │    (Active 8-Way Graph)     │
                       └─────────────────────────────┘
```

### 3.1. Predicate Evaluation Logic
For each command node $C$, the resolver checks the predicate conjunct:
$$\text{Eligible}(C) \iff \forall P_i \in C.\text{requires}, \quad P_i(\text{Context}, \text{Capabilities}) = \text{true}$$

Supported predicate primitives:
- `process.matches(pattern)`: Exact binary name, glob, or regex.
- `window.title_contains(str)`: Substring match on window title.
- `clipboard.has_format(id)`: Checks clipboard format table.
- `clipboard.extension_in(list)`: Checks file extension of clipboard payload.
- `shell.selection_count(op, val)`: Compares selection size ($>, \ge, ==, <$).
- `fs.path_exists(path)`: Verifies file/directory existence in CWD.

### 3.2. Directional Slot Allocation & Conflict Resolution
When multiple eligible commands contend for the same radial slice (e.g. two commands requesting slot `N`):
1. **Specificity Score:** Commands with more specific prerequisites score higher than generic fallbacks (e.g. matching specific file extension scores higher than matching just the process name).
2. **Priority Explicit:** Commands specify an optional `priority: <u32>` attribute in configuration.
3. **Overflow Handling:** Subordinate commands that do not win the primary slice are automatically cascaded into a directional submenu or grouped under a "More Actions" child node.

---

## 4. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-RESOLV-001` | Resolver | Command resolution must be purely functional: given the same `InvocationContext` and graph, output is identical. |
| `INV-RESOLV-002` | Resolver | Resolution of the entire active radial tier must complete within $\le 1.0\,\text{ms}$. |
| `INV-RESOLV-003` | Resolver | Every directional slice must contain at most one active primary command; conflicts must be strictly ordered by priority. |
| `INV-RESOLV-004` | Resolver | If no context-specific command matches a sector, the sector falls back to the configured global default. |

---

## 5. Edge Cases & Resilience Strategy

1. **Conflicting Ambiguous Priorities:** If two commands match with identical specificity and priority, WinPie sorts alphabetically by command ID to maintain strict determinism, logging a configuration warning.
2. **Context Shift Mid-Resolution:** Because resolution operates on the immutable Phase 2 snapshot, window focus shifts occurring while the resolver is running cannot affect the output.
3. **Empty Command Graph:** If an application has zero contextual rules defined, WinPie renders the global system command surface seamlessly.

---

## 6. Verification & Acceptance Criteria

1. [ ] **Context Switching Test:** Simulate invocation over `Code.exe` $\to$ verify North resolves to `Git Commit`; simulate invocation over `explorer.exe` $\to$ verify North resolves to `Compress to Zip`.
2. [ ] **Predicate Completeness Test:** Define commands requiring `.blend` clipboard extensions; test with `.blend` on clipboard $\to$ assert command is present; test with `.txt` on clipboard $\to$ assert command is pruned.
3. [ ] **Microbenchmark Gate:** Benchmark predicate resolution over 500 candidate commands; assert total resolution time $\le 0.8\,\text{ms}$.
