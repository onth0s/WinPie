# Phase 4: Context-Aware Visual Identity

**Document ID:** `05__context_aware_visual_identity.md`  
**Roadmap Phase:** Phase 4  
**Status:** Planned  
**Dependencies:** Phase 2 (`03__context_detection.md`), Phase 3 (`04__context_hydration_and_capability_providers.md`), Phase 0 Overlay Renderer (`overlay.rs`)  

---

## 1. Executive Summary & Strategic Intent

Phase 4 bridges context detection and user perception. In a pure radial system, interaction happens at muscle-memory speeds ($< 300\,\text{ms}$). The user cannot stop to read text labels on every slice.

The primary objective of Phase 4 is to establish an instant subconscious visual confirmation of the detected environment:
> **The visual identity (color, center glyph, ambient glow) becomes semantic state, not merely decorative styling.**

When WinPie activates over Blender, the surface renders in Blender Orange with the Blender glyph in the center deadzone. Over Windows Terminal, it renders in Terminal Emerald Green. Over Visual Studio Code, it renders in IDE Azure. The user immediately knows—without reading a word—which context WinPie is operating in before committing a gesture.

---

## 2. Visual Model & Appearance Schema

```text
Invocation Snapshot (Phase 2 & 3)
                │
                ▼
   ┌──────────────────────────┐
   │ Context Appearance Rule  │
   │  Evaluator & Matcher     │
   └────────────┬─────────────┘
                │
                ▼
   ┌──────────────────────────┐
   │    ContextAppearance     │
   ├──────────────────────────┤
   │ accent_color: ColorRGBA  │ ──► Center hub ring, active hover border, toast accent
   │ base_background: Color   │ ──► Slice fill, dark translucency baseline
   │ center_icon: IconSource  │ ──► Rendered within deadzone (Phase 5 SVG/Bitmap)
   │ context_label: String    │ ──► Breadcrumb header (e.g. "BLENDER: scene.blend")
   │ metadata_badge: Option   │ ──► Status tags (e.g. "GIT: main", "3 FILES")
   └──────────────────────────┘
```

---

## 3. Visual Element Hierarchy & Spatial Mapping

```text
                         [ Context Breadcrumb Label ]
                       "Visual Studio Code: WinPie.rs"
                                      │
                         ┌────────────┴────────────┐
                         │       North Slice       │
                         │                         │
            ┌────────────┼─────────────────────────┼────────────┐
            │            │                         │            │
            │ West Slice │    ┌───────────────┐    │ East Slice │
            │            │    │  Center Hub   │    │            │
            │            │    │  (Deadzone)   │    │            │
            │            │    │   [App Icon]  │    │            │
            │            │    └───────────────┘    │            │
            │            │   Semantic Accent Ring  │            │
            ├────────────┼─────────────────────────┼────────────┤
            │            │                         │            │
            │            │       South Slice       │            │
            └────────────┴────────────┬────────────┴────────────┘
                                      │
                         [ Status Badge: "MODIFIED" ]
```

### 3.1. Center Deadzone Hub
- Previously an empty circle or deadzone indicator.
- In Phase 4, the deadzone ($r \le R_{\text{dead}}$) houses the **Target Context Badge**:
  - Crisp high-DPI application or context icon.
  - Border ring tinted with `accent_color`.
  - Serves as the visual focal point when the cursor begins at $(0, 0)$.

### 3.2. Dynamic Slice Tinting & Hover Feedback
- Slices in resting state inherit a neutral, high-contrast dark surface ($\text{RGBA}(24, 24, 28, 230)$) with faint accent edge lines.
- The hovered slice illuminates with an alpha-blended tint derived from `accent_color` ($\text{RGBA}(R_{\text{accent}}, G_{\text{accent}}, B_{\text{accent}}, 60)$) and a high-intensity accent border stroke.

### 3.3. Toast Notification Harmonization
- The commit confirmation toast inherits the exact `ContextAppearance` theme:
  - Left border stripe colored with `accent_color`.
  - Icon matching the committed command or application domain.

---

## 4. Declarative Theme Configuration

Appearance rules are declared cleanly in configuration files (`config/appearance.yaml`):

```yaml
context_themes:
  - id: blender
    match:
      process: "blender.exe"
    appearance:
      accent_color: "#E87D0D"
      base_background: "#1E1E1E"
      icon: "icons/apps/blender.svg"
      display_name: "Blender"

  - id: vscode
    match:
      process: "Code.exe"
    appearance:
      accent_color: "#007ACC"
      base_background: "#181824"
      icon: "icons/apps/vscode.svg"
      display_name: "VS Code"

  - id: terminal
    match:
      process: ["wt.exe", "powershell.exe", "cmd.exe"]
    appearance:
      accent_color: "#107C10"
      base_background: "#0C0C0C"
      icon: "icons/apps/terminal.svg"
      display_name: "Terminal"

  - id: explorer
    match:
      process: "explorer.exe"
    appearance:
      accent_color: "#F2C811"
      base_background: "#202020"
      icon: "icons/apps/folder.svg"
      display_name: "Files"

  - id: default_fallback
    match:
      fallback: true
    appearance:
      accent_color: "#5B8DEF"
      base_background: "#1E1E24"
      icon: "icons/apps/default.svg"
      display_name: "System"
```

---

## 5. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-THEME-001` | Appearance | Theme matching must be instantaneous ($\le 0.1\,\text{ms}$) using pre-compiled hash lookups against process name. |
| `INV-THEME-002` | Appearance | Every valid invocation must resolve to exactly one valid `ContextAppearance` (default fallback guaranteed). |
| `INV-THEME-003` | Appearance | Dynamic accent colors must meet WCAG AA contrast ratios against text labels ($\ge 4.5:1$). |
| `INV-THEME-004` | Appearance | Overlay redraw due to hover transitions must never trigger disk reads or color re-parsing. |

---

## 6. Edge Cases & Handling

1. **Unknown or Unconfigured Application:** If an application has no explicit theme entry, WinPie falls back to extracting the application's native icon via `ExtractIconExW` / `SHGetFileInfoW` and generates a dominant accent color dynamically or uses `default_fallback`.
2. **Light vs. Dark System Mode:** WinPie respects Windows system dark/light theme preference (`AppsUseLightTheme` registry setting), automatically adjusting text luminance and background alpha levels.
3. **Color Blindness Accessibility:** Accent colors are accompanied by spatial position and distinct icon silhouettes; color is never the sole communicator of state.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Instant Theme Resolution Test:** Run theme matcher against 1,000 process names; verify 100% deterministic theme output in $< 0.05\,\text{ms}$.
2. [ ] **Visual Continuity Test:** Trigger WinPie in Blender; verify center hub shows Blender icon and slice highlights use `#E87D0D`.
3. [ ] **DPI Scale Invariance:** Verify center icon renders with zero pixelation across $100\%$, $125\%$, $150\%$, and $200\%$ display scaling.
