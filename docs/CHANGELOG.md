# Release Notes (v2.2.2)

##  New Features & Improvements
- **Android: keep screen on while reading** — the screen no longer dims mid-chapter (reader setting, default on).
- **Batch convert to EPUB** — multi-select books in the library → "Convert to EPUB" converts all, imports each and moves the originals to the recycle bin (per-book status in the dialog).
- **Bulk "Add to Shelf"** — assign multiple books to a shelf from the multi-select toolbar.
- **Download queue panel** — a Downloads slide-over shows every active book download with live progress (percent + MB); LibGen/Gutenberg register their titles.
- **Statistics: weekly trend chart** — pages-read and reading-time bars from real daily stats; reading-goal reached toast + badge.
- **MangaFire memory bound** — chapter/page caches capped at 50 series.

##  Bug Fixes
- `empty_trash` now also removes orphaned converted-EPUB files (only files no book references).
- Fixed a broken reference in OnlineMangaView (download-all button) and a missing reader-settings setter type (tsc clean).

# Release Notes (v2.2.0)

##  New Features & Improvements
- **Native multi-format reading**: PDF, MOBI, AZW3, DOCX, FB2, TXT, HTML and Markdown books now open in their original format — no more forced "Convert to EPUB" on open. Markdown (.md) is a fully supported new format (import, read, convert).
- **Convert-or-Open dialog**: opening a non-EPUB book offers "Convert to EPUB for the best reading experience" or "Open as-is" (per-book choice remembered for the session).
- **Convert to EPUB improvements**: live conversion percentage with stage labels; converted EPUB is auto-imported into the library and the original is moved to the recycle bin to avoid duplicates; the reader automatically swaps to the converted EPUB.
- **High-fidelity EPUB conversion**: real chapter/TOC/metadata/image extraction for every format (MOBI/AZW3 via the reader pipeline — 34 clean chapters vs. one giant blob before; PDF with line-based heading detection, no body text in titles, Info-dict title/author and embedded cover; DOCX with correct title/author; TXT with title heuristics; FB2/HTML/MD chapter structure). RSS "Generate Daily EPUB" output fixed and validated.
- **PDF reader matches the EPUB experience**: same topbar layout and controls, theme-adaptive pages (dark/black themes invert white PDFs, sepia/paper warm tints), typography settings applied to the text layer, "Page X of Y" indicator.
- **Book covers for every format**: embedded cover extraction for DOCX (first image) and FB2; Google Books → Open Library online lookup fallback when a book has no embedded cover.
- **Fixed the pdf.js worker**: worker bundled via Vite's worker pipeline + CSP worker-src + version-aligned pdfjs-dist — PDFs load reliably in built apps (previously stuck on "Rendering PDF Document…").

##  Bug Fixes
- MOBI: real decoder fixes — PalmDOC decompression (record padding, invalid matches), compression-2 books without huffman tables, hybrid UTF-8/cp1252 text decoding, vendored HUFF/CDIC decoder for huffman-compressed books. Garbled text is gone.
- DOCX: minimal-but-valid .docx files open and convert correctly (direct ZIP/XML fallback); corrupt files give a clean error.
- "Book N not opened" errors: TOC/chapter queries now lazy-open the book from the database — races on open are gone for every format.
- HTML/FB2/MOBI reader flicker on chapter change fixed (mount-effect churn guard + stale-response tokens); PDF endless loading loop fixed (same root cause class).
- EPUB conversion encoding: a byte-wise UTF-8 mangling bug corrupted every converted book ("â€™" mojibake) — all conversions are now byte-perfect.
- Light mode: "Convert to EPUB" menu item now visible on light themes.
- Android: convert-or-open dialog skipped on Android (Radix portal touch issues); verified with a full APK build, install and launch smoke test on an emulator.
- Converted EPUBs are written to a durable app-data location (survive reboots) and reliably auto-import (fixed a race where the 100% progress event skipped the import).

# Release Notes (v1.62.0)

##  New Features & Improvements
- **Auto-Updates**: Added background automatic update checking on startup!
  - **Desktop**: Automatically checks for updates and prompts you to install via the Tauri updater.
  - **Android**: Automatically checks the latest GitHub release and prompts you to download the newest `.apk`.
- **Text-to-Speech (TTS)**: Added a chapter-wise "Start Reading" button to Epub settings, allowing you to seamlessly start reading from the beginning of the current chapter without needing to highlight text.
- **Manga Reader - Floating Page Number**: Added a non-intrusive, theme-adaptive floating page number at the bottom of the manga reader to help you track your progress.
- **Manga Reader - Mobile UX**: Streamlined the Manga sidebar on Android, moving complex settings to the Advanced Settings panel to keep the interface smooth and lightweight.
- **Epub Reader - Mobile**: Disabled double page view for Android devices.

##  Bug Fixes
- Re-added the missing TTS controls/doodle icon on both Desktop and Android interfaces.
- Fixed the visual glitch and lingering loading spinner when switching from online books to online manga by implementing smooth skeleton loading screens.
