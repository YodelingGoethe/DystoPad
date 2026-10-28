# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

DystoPad is a small desktop notepad in Rust (eframe/egui, no web view) in the neon style of the owner's **Dystopoly** game, with a Markdown mode. It should stay as basic as Windows Notepad. It replaces Dystonotepad (Tauri + TypeScript); `docs/TODO.md` is the brief for the rewrite. `docs/Stylingguidelines.md` is the authoritative spec for the look; it is written for CSS, and `src/theme.rs` translates it to egui. If it disagrees with Dystopoly's client (`github.com/YodelingGoethe/Dystopoly`), Dystopoly wins and the copy here should be updated. The repository is public, so keep it presentable.

See `docs/STATUS.md` for what is done and verified and for the next steps. Keep it updated at the end of a work session.

## Commands

- `cargo run`, `cargo test`, `cargo build --release`. Single test: `cargo test heading`. In Git Bash, prepend `$HOME/.cargo/bin` to `PATH` if `cargo` isn't found.
- `cargo fmt` before committing (rustfmt defaults).
- `DYSTOPAD_DATA_DIR=<folder>` makes the app keep its session and settings there instead of `%APPDATA%\DystoPad` (`~/.local/share/DystoPad`), and skips the Dystonotepad import. Use it for test runs, so the owner's real tabs stay untouched.
- Installer: `makensis /DVERSION=x.y.z packaging\windows\installer.nsi` (NSIS also sits in `%LOCALAPPDATA%\tauri\NSIS` on the owner's machine). AppImage: `packaging/linux/appimage.sh x.y.z`.
- Screenshots of the running app: PowerShell with `PrintWindow` (flag 2) after `SetProcessDPIAware`, since egui draws with the GPU.

## Versioning and releases

Versions are `major.minor.hotfix`, tagged `vX.Y.Z`: minor for features, hotfix for fixes only. To release: set the version in `Cargo.toml` (then `cargo build` to update `Cargo.lock`), give the top entry of `docs/CHANGELOG.md` the date, commit, tag, and push `main` and the tag. `.github/workflows/release.yml` builds into a draft release and publishes it once all builds pass. A release has exactly three downloads, and the owner wants it kept that way: the NSIS setup and the portable `.exe` (`DystoPad_<version>_x64-portable.exe`) for Windows, and the AppImage for Linux. Add user-facing changes to the changelog as they land; it is compiled into the app (Settings → What's new). Only tag when the owner says so. `ci.yml` runs the tests on Windows and Linux for every push.

## Architecture

- `src/app.rs`: the window. It owns the tabs, menus, format bar, status bar, dialogs, shortcuts, open/save, print and the session.
  - **Tabs:** `Tab.saved` is the text as it is on disk (`None` = unknown, always unsaved); `dirty()` compares it with `text`. An unsaved tab's title is `first_words` of its text.
  - **Modes:** `md` (the Markdown switch) and `view` (`Editor | Split | Preview`, ignored while `md` is off). New tabs start plain; turning Markdown on goes to Split; opening a `.md` sets `md` and Preview.
  - **Actions:** menus, buttons and shortcuts push an `Action`; they run at the start of the next frame, before the editor is drawn, so focus changes and injected key events (Undo/Redo from the menu) reach the text field in that frame. Cut/Copy/Paste from the menu go through `ViewportCommand::Request*`.
  - **Session:** `touch()` schedules a write 800 ms after the last change; closing the window writes it at once. Closing the window never asks; closing a tab with unsaved changes asks Save / Don't save / Cancel; closing the last tab closes the window.
  - **Split view:** the preview is drawn before the editor, so the editor's scroll-to-cursor can't scroll the preview, and the divider is registered last so it wins the drag.
- `src/editor.rs`: the text field's colors (`Highlighter`, cached per text/size/matches) and the keys that format: Tab/Shift+Tab, Enter in lists, Ctrl+B/I/K, taken out of the input before `TextEdit` sees them. Strikethrough lines are painted by hand (`paint_strikes`): egui draws them too low at this line height.
- `src/format.rs`: the editing commands as pure functions on text and a char selection (wrap, headings, lists, link, table, list continuation, indent), plus `for_preview`, which keeps a bare list marker (Enter, then Tab) from turning the line above into a heading.
- `src/markdown.rs`: pulldown-cmark events turned into a small block tree (GitHub's options, alerts, footnotes, bare links, heading ids). HTML in documents is reduced to its text, line breaks and images.
- `src/preview.rs`: lays the tree out like github-markdown-css (dark), everything scaled by the zoom. Text is a selectable `Label`; inline-code boxes, link underlines and link clicks work from the galley's glyph positions (one glyph per char, tested).
- `src/highlight.rs`: syntect grammars mapped to GitHub's colors (`RULES`, dark for the preview, light for printing). The editor maps the same colors to the neon category colors.
- `src/files.rs`: decoding (UTF-8, BOM, UTF-16, Windows-1252 fallback), line endings, paths. Saving always writes UTF-8 with the file's original line endings and BOM.
- `src/print.rs`: the document as an HTML page in the temp folder, opened in the browser, which prints it. The page's CSP allows only its own script (a nonce), since documents may contain HTML.
- `src/session.rs`: `session.json` (same format as Dystonotepad's, which is imported once) and `settings.json`, written through a temp file.
- `src/single_instance.rs`: a local socket per user; a second start sends its file paths to the running window and exits.
- `src/i18n.rs`: English and German fill the same `Dict` struct, so a missing translation doesn't compile.
- `src/theme.rs`, `src/icons.rs`: palette, fonts, glows and the neon widgets; Lucide line icons and GitHub's Octicons as embedded SVGs.

## Styling rules

From `docs/Stylingguidelines.md`; they apply to all UI code:

- Use the widgets in `theme.rs` (`button`, `button_row`, `neon_box`, `neon_text`, `switch`, `pills`, `icon_button`, `field`) instead of restyling egui's. Colors come from the palette there, mixed with `mix`, `tint` and `fade` (the guide's `color-mix()`).
- Only Tilt Neon and Quicksand in the interface (bundled in `assets/fonts`). The one exception is the text you edit and code, which use the system monospace font (Consolas), as Notepad does. The preview uses GitHub's fonts and styles as they are.
- Stroke-only 24×24 line icons (`icons.rs`, Lucide paths). No emoji or icon fonts.
- Exactly one primary button per dialog; destructive choices are danger buttons. Toasts are for errors only.
- The window must fit 1093×560 and 1280×650 with no control outside it, and text no smaller than 12 px.
