# Phase 5: Native SVG Icon Pipeline

**Document ID:** `06__native_svg_icon_pipeline.md`  
**Roadmap Phase:** Phase 5  
**Status:** Planned  
**Dependencies:** Phase 0 (`01__native_radial_interaction_primitive.md`), Phase 4 (`05__context_aware_visual_identity.md`), Pure Rust Vector Rasterizer (`resvg`, `tiny-skia`, `usvg`)  

---

## 1. Executive Summary & Strategic Intent

Vector icons are required for modern high-DPI Windows displays. However, vector parsing and tessellation are computationally expensive operations. Performing XML parsing, CSS cascading, or Bézier curve flattening during an interaction gesture would introduce frame stutter and violate WinPie's strict latency budgets.

Phase 5 implements a **pure Rust, zero-hotpath SVG asset ingestion and caching pipeline**:
- Ingestion occurs at startup, on background threads, or upon file modification.
- SVGs are pre-rasterized at exact per-monitor physical pixel dimensions.
- The interaction hot path deals exclusively with pre-multiplied 32-bit ARGB contiguous memory blits directly into the layered window DIB.

---

## 2. Pipeline Architecture

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                        COLD PATH: ASSET INGESTION & WARMUP                              │
│                                                                                         │
│  SVG Files on Disk / Embedded Assets                                                    │
│               │                                                                         │
│               ▼                                                                         │
│  XML & DOM Parser (usvg)                                                                │
│  Parse geometry, paths, gradients, viewbox                                              │
│               │                                                                         │
│               ▼                                                                         │
│  Rasterization Engine (tiny-skia / resvg)                                               │
│  Render at target physical pixel sizes: 16x16, 24x24, 32x32, 48x48, 64x64               │
│               │                                                                         │
│               ▼                                                                         │
│  Color Space Conversion & Alpha Premultiplication                                      │
│  Transform straight RGBA to Win32 Premultiplied BGRA/ARGB                               │
│               │                                                                         │
│               ▼                                                                         │
│  In-Memory Glyph Cache (GlyphCache)                                                     │
│  Key: (IconId, TargetWidth, TargetHeight, AccentTint)                                   │
└───────────────────────────────────────────┬─────────────────────────────────────────────┘
                                            │
                                            ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                     HOT PATH: ZERO-ALLOCATION INTERACTION RENDER                        │
│                                                                                         │
│  Overlay Repaint Event (WM_PAINT / UpdateLayeredWindow)                                 │
│               │                                                                         │
│               ▼                                                                         │
│  O(1) Cache Hash Lookup ──► Retrieve Raw Premultiplied Pixel Slice                      │
│               │                                                                         │
│               ▼                                                                         │
│  Direct SIMD / GDI+ BitBlt into Layered DIB Buffer                                      │
│  Zero XML parsing. Zero Bézier rasterization. Zero heap allocations.                    │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Data Structures & Cache Schema

```rust
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct IconKey {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tint: Option<[u8; 4]>, // RGBA tint override if dynamic
}

pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// 32-bit premultiplied BGRA pixels matching Win32 DIB format
    pub bgra_data: Vec<u8>,
}

pub struct IconPipeline {
    cache: RwLock<HashMap<IconKey, Arc<RasterizedGlyph>>>,
    asset_dir: PathBuf,
}
```

---

## 4. Technical Requirements & Specifications

### 4.1. Pure Rust Ingestion & Sandboxing
- Zero dependency on external C libraries (no `librsvg`, `cairo`, or system WebKit).
- Complete isolation against malformed or malicious SVG files (XML entity expansion protection, circular reference recursion limits enforced by `usvg`).

### 4.2. Per-Monitor DPI Awareness
- Windows supports fractional scaling ($100\% = 96\,\text{DPI}$, $125\% = 120\,\text{DPI}$, $150\% = 144\,\text{DPI}$, $200\% = 192\,\text{DPI}$, $250\% = 240\,\text{DPI}$).
- If an icon is defined at $24 \times 24$ logical points:
  - At $100\%$ scale: rasterized to $24 \times 24\,\text{px}$.
  - At $150\%$ scale: rasterized to $36 \times 36\,\text{px}$.
  - At $200\%$ scale: rasterized to $48 \times 48\,\text{px}$.
- Crisp rasterization avoids bilinear filtering blur or fuzzy edges.

### 4.3. Win32 Premultiplied Alpha Compatibility
- Win32 layered windows (`UpdateLayeredWindow` with `AC_SRC_ALPHA`) require pixel data to be stored in **premultiplied BGRA** byte order:
  $$\text{Color}_{\text{stored}} = \frac{\text{Color}_{\text{straight}} \times \text{Alpha}}{255}$$
- The pipeline performs alpha premultiplication and RGB-to-BGR byte swapping during rasterization, making blitting a direct memory copy.

### 4.4. Dynamic Color Tinting (Monochrome Masking)
- Standard monochrome UI glyphs (e.g. system arrows, file icons, edit tools) can be dynamically tinted to match the active `accent_color` from Phase 4 without re-parsing the SVG:
  - Mask channels are multiplied against the target RGBA tint in a single SIMD pass.

---

## 5. Formal Invariants

| Invariant ID | Subsystem | Formal Statement |
| :--- | :--- | :--- |
| `INV-ICON-001` | Graphics | Zero SVG parsing, XML decoding, or vector math is permitted inside hook or render hot paths. |
| `INV-ICON-002` | Graphics | Cache lookup for an active slice glyph must complete in $\le 1.0\,\mu\text{s}$ ($O(1)$). |
| `INV-ICON-003` | Graphics | All cached bitmaps must conform strictly to 32-bit premultiplied BGRA format. |
| `INV-ICON-004` | Graphics | Memory footprint of cached icons is capped via an LRU eviction policy (default max: 32MB). |
| `INV-ICON-005` | Graphics | Missing or unparseable SVG files must resolve to a bundled fallback glyph without panicking. |

---

## 6. Edge Cases & Resilience Strategy

1. **Non-Square SVGs:** Icons with arbitrary aspect ratios (e.g. $16:9$ banners) are automatically fitted inside the target bounding box with preserved aspect ratios and centered letterboxing.
2. **Dynamic Display Scaling Shift:** If the user drags WinPie or switches displays to a monitor with different DPI, the pipeline checks the cache for the new physical dimension. If absent, a worker thread generates it while the renderer uses nearest-neighbor scaling for one frame as an emergency fallback.
3. **Hot-Reloading Development:** In debug mode, file watchers detect modifications to `.svg` files in `assets/icons/` and flush affected cache entries automatically.

---

## 7. Verification & Acceptance Criteria

1. [ ] **Memory Allocation Verification:** Instrument the overlay paint loop during continuous cursor rotation; assert zero allocations occur when retrieving and blitting cached glyphs.
2. [ ] **DPI Fidelity Gate:** Test rasterization across $100\%$, $125\%$, $150\%$, $175\%$, and $200\%$ scaling; assert rendered dimensions match exact integer pixel requirements.
3. [ ] **Corrupt SVG Test:** Feed intentionally broken, truncated, and maliciously recursive XML files into the loader; verify all error gracefully without crashing.
