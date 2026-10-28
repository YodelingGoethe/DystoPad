# Status

Last updated: 2026-10-28

## Where things stand

- **App:** DystoPad 1.0.0, the Rust rewrite of Dystonotepad 0.1.0, with everything from the rewrite brief (`TODO.md`): the rename and new ".Md" icon, the list-heading bug fix, GitHub's code block styling, first-words tab names, the resizable split, the blue unsaved dot and the GPL license. Not released yet: the owner tags it.
- **Stack:** Rust 2024, eframe/egui 0.37 (wgpu), pulldown-cmark, syntect, rfd for the file dialogs, interprocess for the single instance.
- Verified on 2026-10-28 (Windows 11 at 125 % scaling, Rust 1.99.1), driving the debug and release builds with simulated clicks and keys and checking screenshots:
  - `cargo test` (40 tests) passes; `cargo build --release` gives a 16 MB `dystopad.exe` with its icon and "DystoPad" as the file description; `makensis` builds the 6 MB setup.
  - Session restore: tabs come back with their mode, view and cursor; an unsaved untitled tab keeps its text, blue dot and first-words title ("12 Okay some insight fro…").
  - Preview against GitHub: headings with rules, inline code boxes, the Java sample from the brief in GitHub's colors (`public`/`void`/`if` red, `onInitializeClient`/`load`/`save` purple, `false` blue, the rest white, tabs 8 wide), nested numbered lists (i., ii.), quotes, Note/Tip/Warning alerts with Octicons, a rule, task lists, a table with aligned columns, a footnote.
  - The bug from the brief: "- milk", Enter, Tab shows an empty nested bullet in the preview, and no heading in the editor's highlighting either.
  - Markdown switch on a new tab goes to Split; Enter continues the list, Tab nests it; Ctrl+F and Ctrl+H open the find bar (wraps in a narrow pane) and highlight matches; dragging the divider resizes the panes.
  - Settings dialog, File and Edit menus, the Save / Don't save / Cancel prompt on Ctrl+W.
  - A second start with a file hands it to the running window as a new tab and exits.
- **Not yet verified:**
  - Running the installer ("Open with" for `.txt`, `.md`, `.markdown`, the Start menu entry, uninstalling) and the portable exe on another PC.
  - Printing end to end (the page generation is unit-tested; the browser's print dialog wasn't opened), the clipboard menu items, dropping files on the window, web links opening in the browser, German in every dialog, the changelog dialog, zoom with the wheel.
  - Importing a real Dystonotepad session (tested with a hand-written one in the same format).
  - Linux: CI builds and tests it; the AppImage hasn't been started on a Linux desktop. macOS: not built.

## Known simplifications

- Files in UTF-16 or Windows-1252 are read correctly but saved as UTF-8 (`shortcut:` in `src/files.rs`).
- Find has match case and whole word, but no regular expressions (Dystonotepad's CodeMirror search had them).
- The preview shows only the text, line breaks and images of HTML in a document; images inside table cells show their alt text. It lays out the whole document every frame, which is fine for notes but slow for very long files.
- No scroll sync between the editor and the preview, no drag-and-drop reordering of tabs, no spellcheck, no Mermaid or math.
- Undo history isn't kept across restarts. The window size and position aren't remembered; it opens at 1440×900, or maximized on small screens.
- Unsigned builds: Windows SmartScreen warns on the first start.

## Next steps (suggested, not yet agreed with the owner)

1. Install the setup and check "Open with" for `.txt` and `.md`, then uninstall Dystonotepad.
2. Tag `v1.0.0` when the owner is happy, which builds the three downloads.
3. Use it for a while and collect the next TODO list.
