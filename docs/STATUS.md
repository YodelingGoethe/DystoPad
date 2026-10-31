# Status

Last updated: 2026-10-31

## Where things stand

- **App:** DystoPad 1.1.0, released as `v1.1.0` on 2026-10-31: the tabs moved into the title bar, as in Windows Notepad (the window draws its own frame buttons, moves, maximizes and resizes itself). 1.0.0 (2026-10-28) was the Rust rewrite of Dystonotepad 0.1.0, with everything from the rewrite brief: the rename and new ".Md" icon, the list-heading bug fix, GitHub's code block styling, first-words tab names, the resizable split, the blue unsaved dot and the GPL license.
- **Postponed:** Ctrl+Alt typing what AltGr types (`[ ] { } @ €` on a German keyboard). `TODO.md` explains why: the fix is in egui-winit, and the owner doesn't want to build from a patched copy of egui.
- **Stack:** Rust 2024, eframe/egui 0.37 (wgpu), pulldown-cmark, syntect, rfd for the file dialogs, interprocess for the single instance.
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
  - The title bar on Linux (X11 and Wayland), dragging the window between monitors with different scaling, and Windows' window menu (Alt+Space, right-click on the strip; probably gone with the system frame).
  - The setup through its pages (only the silent install was run), and the portable exe on another PC.
  - A web link clicked in the preview opening in the browser (it uses the same call as printing), and zoom with Ctrl+mouse wheel.
  - Linux: CI builds and tests it; the AppImage is first built by the release workflow and hasn't been started on a Linux desktop. macOS: not built.

## Known simplifications

- Files in UTF-16 or Windows-1252 are read correctly but saved as UTF-8 (`shortcut:` in `src/files.rs`).
- Find has match case and whole word, but no regular expressions (Dystonotepad's CodeMirror search had them).
- The preview shows only the text, line breaks and images of HTML in a document; images inside table cells show their alt text. It lays out the whole document every frame, which is fine for notes but slow for very long files.
- No scroll sync between the editor and the preview, no drag-and-drop reordering of tabs, no spellcheck, no Mermaid or math.
- Undo history isn't kept across restarts. The window size and position aren't remembered; it opens at 1440×900, or maximized on small screens.
- Unsigned builds: Windows SmartScreen warns on the first start.
- The window draws its own title bar, so Windows 11's snap layouts don't pop up over the maximize button. Dragging the window to the top of the screen still maximizes it (tested); Win+Z and snapping to the sides weren't tried. The window resizes from its outermost 4 px (12 px in the corners) instead of Windows' wider invisible border.

## Next steps (suggested, not yet agreed with the owner)

1. Use the new title bar for a while: moving, snapping, resizing, several monitors.
2. Install the setup from the release, check "Open with" for `.txt` and `.md`, then uninstall Dystonotepad.
3. Start the AppImage on a Linux desktop, and check the title bar there.
4. Ctrl+Alt as AltGr (`TODO.md`): check whether a newer egui has fixed it.
