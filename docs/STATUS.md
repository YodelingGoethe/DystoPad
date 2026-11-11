# Status

Last updated: 2026-11-11

## Where things stand

- **App:** DystoPad 1.2.1, on `main` since 2026-11-11, not tagged yet. 1.2.0 wasn't tagged either: its Linux CI job failed three times because GitHub's Ubuntu runner couldn't reach Ubuntu's package servers, and those runs are gone from GitHub since. 1.2.1 fixes snapping on Windows across monitors with different scaling: the window no longer jumps between them, and it snaps to half of a monitor with less scaling. The owner tags it when CI passes on Linux. 1.2.0: tabs move between windows as in Windows Notepad: off the strip into a new window, onto another window's strip, or along the strip to reorder. The preview fills the window (Settings can keep GitHub's column in split view), and wide tables wrap their cells. Ctrl+Alt types what AltGr types on Windows. 1.1.0 (2026-10-31) moved the tabs into the title bar, as in Windows Notepad (the window draws its own frame buttons, moves, maximizes and resizes itself). 1.0.0 (2026-10-28) was the Rust rewrite of Dystonotepad 0.1.0, with everything from the rewrite brief: the rename and new ".Md" icon, the list-heading bug fix, GitHub's code block styling, first-words tab names, the resizable split, the blue unsaved dot and the GPL license.
- **Stack:** Rust 2024, eframe/egui 0.37 (wgpu), pulldown-cmark, syntect, rfd for the file dialogs, interprocess for the single instance; on Windows, winit's message hook and windows-sys (`src/win32.rs`).
- Verified on 2026-11-11 for 1.2.1 (Windows 11, four monitors: the main one 2560×1440 at 125 %, one of the same size at 100 % to its right, 1920×1080 at 100 % to its left, a portrait one), driving the debug build with simulated keys and mouse and logging the window's rect:
  - `cargo test` (42 tests) passes.
  - Before the fix, Win+Right from the right half of the main screen left the window straddling both screens at the main screen's size (winit moved it back). Now Win+Right and Win+Left across all three landscape screens give exactly the half: 1280×1380 at 125 %, 1280×1392 and 960×1032 at 100 % (960 used to come out as 1125, the minimum width at 125 %).
  - Dragging the window by the strip across screens: it follows the mouse and takes the new scale; for one moment in the crossing it has another size. Dragged to the top of the 100 % screen it maximizes there; dragged from there to the far left edge it snaps to 960×1032.
  - A maximized window moved with Win+Shift+arrow across the 125 % and 100 % screens stays maximized and fills each; restored, it has its old size. A screenshot after snapping onto the 100 % screen shows the interface drawn at 100 %, filling the window.
  - Not tried: snapping a second window (the same hook and code path), and fullscreen (F11) across monitors.
- Verified on 2026-11-11 for 1.2.0 (Windows 11 at 125 % scaling, German keyboard), driving the debug build with simulated keys and mouse and checking screenshots:
  - `cargo test` (42 tests) passes.
  - Ctrl+left Alt with Q, E, 2, 3, 7, 8, 9, 0, ß, + and < types `@€²³{[]}\~|`, and the window stays open. AltGr+Q types one `@`. Ctrl+Alt+S does nothing, and Ctrl+A still selects all. This works in a dragged-out window too.
  - Tabs: dragging one along the strip reorders it. Off the strip, a ghost of it follows the mouse; let go over the window's own editor or on the desktop, it opens in a new window, offset by the drag, with the same size. Dropped on another window's strip, it goes in between the tabs it was dropped between, also when grabbed in a window in the background and let go after a pause. A window's only tab dropped elsewhere moves the window by the drag.
  - Closing the main window while another is open asks about each unsaved tab (Cancel keeps the window); after that the main window hides and the other keeps working, takes files from a second start, and moves by its tab. Ctrl+W on a second window's last unsaved tab asks, then closes only that window. Closing the last window doesn't ask and ends DystoPad. The session holds the tabs of all open windows, and the next start shows them in one window.
  - Preview: full width in Preview and Split view; a five-column table wraps its long cells (inline code and links in wrapped rows included), and a small table keeps its natural width. The settings switch fits at 1093×560.
- Verified on 2026-10-31 for 1.1.0 (Windows 11 at 125 % scaling), driving the debug build with simulated mouse and keys and checking screenshots:
  - `cargo test` (41 tests) passes. The title bar at 1093×560 and 1280×650 (logical) and maximized: the window buttons fit, show Windows' tooltips ("Restore down" while maximized) and the close button turns red on hover.
  - Dragging the empty strip moves the window; a double-click maximizes; the maximize button restores; dragging it to the top of the screen snaps it to maximized; minimize minimizes; F11's fullscreen shows the restore icon, which leaves fullscreen; the close button closes the window and the session is saved.
  - Dragging the right edge and the top left corner resizes the window by exactly the dragged amount. The editor's scroll bar, next to the right edge, still drags.
  - With more tabs than fit, a new tab and Ctrl+Tab back to the first are scrolled into view, and the strip's line stays closed under the hidden ones.
- Verified on 2026-10-28 (Windows 11 at 125 % scaling, Rust 1.99.1), driving the debug and release builds with simulated clicks and keys and checking screenshots:
  - `cargo test` (41 tests) passes; `cargo build --release` gives a 16 MB `dystopad.exe` with its icon and "DystoPad" as the file description; `makensis` builds the 6 MB setup.
  - The setup, run silently: installs to `%LOCALAPPDATA%\Programs\DystoPad` without admin rights, adds the Start menu entry, "Open with" for `.txt`, `.md` and `.markdown` and an Apps entry, and leaves the default `.txt` app alone. Starting the installed exe the way "Open with" does opens the file as a new tab. The silent uninstall removes all of it.
  - Importing the owner's real Dystonotepad session (a copy, in a test folder): both tabs came back with their text, Markdown mode and cursor, including one whose file no longer exists (kept as unsaved).
  - Session restore: tabs come back with their mode, view and cursor; an unsaved untitled tab keeps its text, blue dot and first-words title ("12 Okay some insight fro…").
  - Preview against GitHub: headings with rules, inline code boxes, the Java sample from the brief in GitHub's colors (`public`/`void`/`if` red, `onInitializeClient`/`load`/`save` purple, `false` blue, the rest white, tabs 8 wide), nested numbered lists (i., ii.), quotes, Note/Tip/Warning alerts with Octicons, a rule, task lists, a table with aligned columns, a footnote.
  - The bug from the brief: "- milk", Enter, Tab shows an empty nested bullet in the preview, and no heading in the editor's highlighting either.
  - Markdown switch on a new tab goes to Split; Enter continues the list, Tab nests it; Ctrl+F and Ctrl+H open the find bar (wraps in a narrow pane) and highlight matches; dragging the divider resizes the panes.
  - Settings dialog, File and Edit menus, the Save / Don't save / Cancel prompt on Ctrl+W, go to line, zoom, German in the menus, status bar and dialogs, the changelog dialog.
  - Edit menu: Select all, Cut/Copy (whole text and a mouse selection, checked on the clipboard), Paste replacing the selection, Undo. The Bold button wraps the selection.
  - A second start with a file hands it to the running window as a new tab and exits. A file dragged onto the window (a real OLE drag) opens as a new tab; a Windows-1252 file shows its umlauts.
  - Printing: Ctrl+P opens the page in the default browser (Firefox) with its print dialog. The page, rendered in headless Edge, matches GitHub's light style, task lists, alerts and footnotes included.
- **Not yet verified:**
  - Several windows on Linux: on Wayland egui can't tell where windows are, so a tab dropped off the strip always opens a new window wherever the compositor puts it, and a window's only tab can't move it. Dropping a tab between monitors with different scaling (the window positions are compared in pixels, so it should work).
  - The title bar on Linux (X11 and Wayland), dragging the window between monitors with different scaling on Linux, and Windows' window menu (Alt+Space, right-click on the strip; probably gone with the system frame).
  - The setup through its pages (only the silent install was run), and the portable exe on another PC.
  - A web link clicked in the preview opening in the browser (it uses the same call as printing), and zoom with Ctrl+mouse wheel.
  - Linux: CI builds and tests it; the AppImage is first built by the release workflow and hasn't been started on a Linux desktop. macOS: not built.

## Known simplifications

- Files in UTF-16 or Windows-1252 are read correctly but saved as UTF-8 (`shortcut:` in `src/files.rs`).
- Find has match case and whole word, but no regular expressions (Dystonotepad's CodeMirror search had them).
- The preview shows only the text, line breaks and images of HTML in a document; images inside table cells show their alt text. It lays out the whole document every frame, which is fine for notes but slow for very long files.
- No scroll sync between the editor and the preview, no spellcheck, no Mermaid or math.
- A dragged tab's ghost shows only inside its own window (egui draws nothing outside its windows); a new window appears when the tab is let go, rather than following the mouse as in Notepad. The strip doesn't scroll while a tab is dragged to its edge.
- Table cells wrap when the table is too wide, but a centered or right-aligned column aligns the cell's text block, not each wrapped line.
- Undo history isn't kept across restarts. The window size and position aren't remembered; it opens at 1440×900, or maximized on small screens.
- Unsigned builds: Windows SmartScreen warns on the first start.
- The window draws its own title bar, so Windows 11's snap layouts don't pop up over the maximize button. Dragging the window to the top or the sides of the screen and Win+arrow snap it (tested); Win+Z wasn't tried. The window resizes from its outermost 4 px (12 px in the corners) instead of Windows' wider invisible border.

## Next steps (suggested, not yet agreed with the owner)

0. Check that CI passes on Linux for the 1.2.1 push. Once it does and the owner says so, tag `v1.2.1` and push the tag (1.2.0 was never released, so its changelog entry goes out with 1.2.1's); if that's not on 2026-11-11, change the date of 1.2.1 in the changelog first.
1. Use the new title bar for a while: moving, snapping, resizing, several monitors.
2. Install the setup from the release, check "Open with" for `.txt` and `.md`, then uninstall Dystonotepad.
3. Start the AppImage on a Linux desktop, and check the title bar there.
4. Use several windows for a while: tearing tabs off, merging them back, closing windows with unsaved tabs.
