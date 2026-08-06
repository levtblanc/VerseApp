# Verse Document Reader

A modular, high-performance, multi-format document reader written in Rust using the **Iced 0.13** GUI framework (Elm Architecture), **MuPDF Fitz** C Engine (`mupdf` crate), and native DjVu and DOCX engine pipelines.

---

## Supported Formats

`PDF` · `EPUB` · `DOCX` · `DjVu` · `XPS` · `MOBI` · `CBZ` · `FB2`

---

## Key Features

* **Multi-Tab Workspace & Drag Reordering**: Chrome-style tab strip with horizontal mouse drag-and-drop tab shuffling ($O(1)$ memory swaps that preserve reading state), tab close auto-jumps to neighboring tabs, and soft tab persistence.
* **Virtualized Continuous Scroll Engine**:
  * **Zero-Lag Scrolling**: Pages outside the immediate visible window ($4$ behind, $8$ ahead) render as zero-texture blank boxes with exact pixel dimensions, keeping VRAM/RAM under $35\text{ MB}$ even on $1000$+ page documents.
  * **Pixel-Accurate Y-Offset Tracking**: `page_at_y_offset()` and `y_offset_for_page()` calculate exact scroll positions across mixed-height page documents.
* **Facing Pages & View Modes**: Single page, double facing pages, single continuous, and double continuous mode.
* **Interactive Control Tray**:
  * Floating control tray with theme-independent off-white text.
  * **Direct Page Input Box**: Type page numbers directly and press `Enter` (auto-clamped and sanitized against invalid inputs).
  * Dual-axis scrolling (`Direction::Both` when zoomed in beyond screen width; centered `Direction::Vertical` at normal zoom).
* **Navigation Side Panel**:
  * Table of Contents (Outline tree) and $210\text{ pt}$ fixed-height Thumbnail grid.
  * **Active Thumbnail Highlight**: Active page thumbnail features a blue accent border (`#6194EA`), slate-blue container glow, and highlighted label text.
  * **Dual-Mode Thumbnail Engine**: Renders visible side panel thumbnails dynamically in real-time ($<5\text{ ms}$ draft renders) and auto-centers the active thumbnail when turning pages in the main reader.
* **Theme-Adaptive Floating Scrollbars**:
  * Track background is $100\%$ transparent.
  * Scroller thumb dynamically turns **translucent light gray in Dark Mode** and **translucent dark gray in Light Mode** for high contrast visibility across themes.
  * Scroller thickness locked to a comfortable $12\text{ px}$.
* **Polished Settings & Shortcut Remapping**:
  * Glassmorphic modal overlay for customizable hotkeys and theme toggles.
  * **Live Modifier Capture**: Displays held modifier combinations (`Ctrl + Shift + ...`) in real-time while remapping shortcuts. Press `Escape` to cancel.

---

## Project Directory Map

```text
verseapp/
├── Cargo.toml                # Project manifest, trimmed features & release profiles
└── src/
    ├── main.rs               # App entry point & Iced runtime initialization
    ├── app/                  # Elm Architecture Core
    │   ├── mod.rs            # Facade module re-exporting ReaderApp & Message
    │   ├── state.rs          # ReaderApp struct, new(), theme(), save_session(), panic-proof restoration
    │   ├── messages.rs       # Exhaustive Message enum definition
    │   ├── actions.rs        # Key normalization, modifier filters & Action dispatcher
    │   ├── tasks.rs          # Tokio worker tasks, non-blocking thumbnail queue (8 workers)
    │   ├── view.rs           # Root UI stack assembly & persistent base layout
    │   └── update/           # Modular App Reducer
    │       ├── mod.rs        # Main Message dispatcher
    │       ├── tabs.rs       # Open, close, select & mouse drag-swap tab handlers
    │       ├── navigation.rs # Page turns, zoom, scroll & page input box handlers
    │       ├── side_panel.rs # Side panel toggle, sub-tab & thumbnail scroll handlers
    │       ├── settings.rs   # Hotkey remapping, window mode & event handlers
    │       └── renders.rs    # Page & thumbnail background render completion callbacks
    ├── engine/               # Document Engine Layer
    │   ├── mod.rs            # Factory loader (load_document)
    │   ├── traits.rs         # DocumentBackend trait, RenderQuality, PageRenderRequest
    │   ├── mupdf.rs          # Thread-safe MuPDF engine with O(1) lock-free dimension cache
    │   ├── djvu.rs           # Native DjVu wavelet engine
    │   └── docx.rs           # In-memory DOCX XML-to-HTML converter & MuPDF Fitz pipeline
    ├── models/               # Persistence & Workspace State
    │   ├── mod.rs
    │   ├── session.rs        # SessionData, AppSettings, FileHistoryRecord, KeyBinding
    │   └── workspace.rs      # RuntimeTab, byte-budget LRU texture cache, side_panel_thumb_y
    └── ui/                   # User Interface & Theme System
        ├── mod.rs            # Exports components and theme
        ├── theme.rs          # Theme-adaptive transparent & invisible scrollable styles
        └── components/
            ├── mod.rs
            ├── tab_bar.rs    # Horizontal scrollable tab bar with mouse_area drag reordering
            ├── side_panel.rs # Outline TOC tree & fixed-height thumbnail grid
            ├── settings.rs   # Glassmorphic keymap modal overlay
            └── viewer/       # Decomposed Document Reader Viewport
                ├── mod.rs    # Root reader canvas layout assembly
                ├── toolbar.rs# Floating control tray & interactive page input
                ├── continuous.rs # Virtualized single & double continuous scroll views
                └── paginated.rs  # Paginated single & double facing page views