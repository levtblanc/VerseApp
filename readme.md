verseapp/
├── Cargo.toml                    // Dependency manifest & build configuration
└── src/
    ├── main.rs                   // App entry point & Iced runtime initialization
    ├── app/                      // Modular Elm Architecture Core
    │   ├── mod.rs                // Facade module re-exporting ReaderApp & Message
    │   ├── state.rs              // ReaderApp struct, new(), theme(), session saving, & memory purging
    │   ├── messages.rs           // Exhaustive Message enum definition
    │   ├── actions.rs            // Keyboard normalization (key_to_string) & Action dispatchers
    │   ├── tasks.rs              // Non-blocking Tokio worker task queue & pre-fetch batching
    │   ├── update.rs             // State reducer (update loop & debounced scroll/zoom handlers)
    │   └── view.rs               // Root UI layout composition & window assembly
    ├── engine/                   // Decoupled Document Abstraction & Renderers
    │   ├── mod.rs                // Factory dispatcher (load_document)
    │   ├── traits.rs             // DocumentBackend trait, PageRenderRequest, RenderQuality, TocItem
    │   ├── mupdf.rs              // Mutex-locked MuPDF Fitz C FFI engine (PDF, EPUB, FB2, MOBI, XPS, CBZ)
    │   ├── djvu.rs               // Native DjVu engine & IFF chunk parser
    │   └── docx.rs               // Native DOCX engine & ZIP XML paragraph extractor
    ├── models/                   // Domain Models & Persistent State
    │   ├── mod.rs
    │   ├── session.rs            // SessionData, AppSettings, FileHistoryRecord, KeyBinding, Action, PageLayout, SidePanelTab
    │   └── workspace.rs          // RuntimeTab state, bounded texture LRU cache, quality-aware pre-fetching
    └── ui/                       // Graphical User Interface & Themes
        ├── mod.rs
        ├── theme.rs              // Light & Dark Iced theme definitions
        └── components/           // Reusable UI Components
            ├── mod.rs
            ├── tab_bar.rs        // Scrollable Chrome-like browser tab bar with embedded close buttons
            ├── viewer.rs         // Centered document viewport, floating control tray, & split/overlay canvas
            ├── side_panel.rs     // Navigation panel (TOC Outline tree + Page Thumbnails grid)
            └── settings.rs       // Interactive keymap remapping modal

Component-by-Component Description
1. Project Manifest (Cargo.toml)

    iced (v0.13): GUI framework using the Elm Architecture (Model-View-Update pattern) with image, tokio, and svg features.

    mupdf (v0.8): C FFI bindings to Fitz/MuPDF for native vector and text rasterization.

    djvu (v0.1) & zip (v2.1): Native parsers for DjVu files and DOCX/eBook ZIP archives.

    rfd (v0.15): Async Rusty File Dialog for cross-platform native file pickers.

    serde & serde_json: Serializes settings, reading history, and open tabs to disk (session.json).

2. Engine Layer (src/engine/)

Decouples file decoding and page rasterization from the user interface.

    src/engine/traits.rs:

        DocumentBackend: Supertrait (Send + Sync + std::fmt::Debug) requiring page_count(), page_dimensions(), render_page(), and table_of_contents().

        RenderQuality: Enum defining resolution scaling tiers:
    src/engine/mupdf.rs:

        Wraps mupdf::Document in a ThreadSafeDocument(Mutex<Document>) to guarantee safe thread access across background Tokio workers.

        Handles PDF, EPUB, FB2, MOBI, XPS, and CBZ formats natively.

        Converts 3-byte RGB samples to 4-byte RGBA buffers with alpha = false for high-contrast white paper backgrounds.

    src/engine/djvu.rs: Handles .djvu files using IW44/JB2 wavelet decoding and downsampling.

    src/engine/docx.rs: Handles .docx files by extracting paragraphs and images from word/document.xml.

    src/engine/mod.rs: Factory dispatcher load_document(path) that inspects extensions and returns Result<Arc<dyn DocumentBackend>, String>.


3. Domain Models & Persistence (src/models/)

    src/models/session.rs:

        Action: Enum defining all shortcuts (OpenFile, NextTab, PrevTab, ToggleSidePanel, ToggleFullscreen, ToggleTabBar, etc.).

        FileHistoryRecord: Stores reading history per file (current page, zoom, layout, continuous mode).

        SessionData: Manages JSON I/O (session.json) in system configuration directories (dirs::config_dir()).

    src/models/workspace.rs:

        RuntimeTab: Manages in-memory document tab state:

            texture_cache: Quality-aware texture map preventing high-res renders from being overwritten by draft renders.

            thumbnail_cache: Stores

                    
            180×240
            180×240

                  

            pixel preview handles.

            required_pages_with_quality(): Priority queue returning (page_index, RenderQuality) pairs for predictive pre-fetching. In Double View, both facing pages are requested as RenderQuality::High.

            purge_inactive_cache(): Retains the active page texture when a tab becomes inactive while freeing background memory, keeping total RAM around

                    
            ∼15–25 MB
            ∼15–25 MB

                  

4. GUI Components (src/ui/components/)

    src/ui/components/tab_bar.rs:

        Scrollable Tab Strip: Rendered inside a horizontal scrollable widget.

        Embedded Close Button (✕): Integrated inside each tab pill with a red hover effect.

        New Tab (+) Button: Pinned at the end of the tab bar for opening documents.

    src/ui/components/viewer.rs:

        Centered Control Tray: Floating top toolbar displaying navigation buttons, single/double view mode, continuous mode, zoom badges, theme toggle (🌓), fullscreen (🖥), and panel toggle (📂).

        Centered Viewport Layout: Document pages are kept dead-centered in the workspace horizontally and vertically.

        Flush Double Spreads: Facing pages in Double View join

                
        100%
        100%

              

        flush with zero horizontal gap, sharing identical reference aspect ratios.

        Clipping Viewport: Uses .clip(true) on the scrollable container to prevent large pages or EPUBs from overflowing window borders.

    src/ui/components/side_panel.rs:

        Sub-tab switcher between 📑 Outline (TOC tree) and 🖼 Thumbnails.

        Assigned scrollable::Id allowing automatic scroll tracking to follow page turns in the main reader.

    src/ui/components/settings.rs:

        Dimmed modal overlay for interactive hotkey remapping and theme configuration.


5. Application Core (src/app/)

    src/app/mod.rs: Facade module re-exporting ReaderApp and Message.

    src/app/messages.rs: Defines all application event messages, including debounced settle events (ScrollSettled, ZoomSettled).

    src/app/state.rs: Defines ReaderApp struct, startup tab restoration, purge_all_inactive_tabs(), and disk session serialization.

    src/app/actions.rs: Helper function key_to_string for key normalization and shortcut action dispatching.

    src/app/tasks.rs:

        spawn_render_task / spawn_thumbnail_render_task: Dispatches background Tokio worker tasks.

        request_page_renders_with_quality: Dispatches render tasks according to the priority quality queue.

    src/app/update.rs:

        Core update reducer.

        Non-blocking File Opening: Offloads load_document() to tokio::task::spawn_blocking.

        Two-Pass Dynamic Rendering: Triggers fast RenderQuality::Draft renders during scrolling/zooming, then schedules a

                
        180 ms
        180 ms

              

        debounced upgrade to RenderQuality::High when movement stops (ScrollSettled, ZoomSettled).

    src/app/view.rs: Root layout renderer assembling the top header, error banner, viewer canvas, side panel overlay/dock, and settings modal.