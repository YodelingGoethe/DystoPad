# Changelog

Versions are `major.minor.hotfix`. Each version is a `vX.Y.Z` tag on the Releases page, with a Windows installer, a portable Windows `.exe` and a Linux AppImage.

## 1.1.0 (2026-10-31)

- **The tabs sit in the title bar,** as in Windows Notepad, next to DystoPad's own minimize, maximize and close buttons. Drag the empty space beside the tabs to move the window, double-click it to maximize, and drag the window's edges to resize it.
- **Fixed:** a new or opened tab that doesn't fit in the tab strip is now scrolled into view.

## 1.0.0 (2026-10-28)

Dystonotepad is now **DystoPad**, rebuilt from scratch in Rust as a native app. It starts with the tabs you had open in Dystonotepad, unsaved text included.

- **New icon,** with ".Md" on it.
- **Unsaved tabs are named after their first words,** as in Notepad, and Save as suggests that name.
- **Resizable split view:** drag the line between the editor and the preview.
- **The unsaved dot is blue** instead of gold.
- **Code blocks look like GitHub's:** its colors (method calls like `.load()` included), font size, line height and padding.
- **Fixed:** pressing Enter and then Tab in a list no longer turns the line above into a heading for a moment.
- **Printing** opens the document in your browser, ready to print.
- **Free software** under the GNU GPL, version 3 or later.

## 0.1.0 (2026-10-28), as Dystonotepad

The first version.

- **A plain notepad:** tabs, opening and saving text files, undo and redo, find and replace, go to line, word wrap, printing and zoom (Ctrl +/−, Ctrl+mouse wheel). The status bar shows the line, column, character count and zoom.
- **Tabs stay open** until you close them, unsaved text included, even after closing the app. Files opened from Explorer open as a new tab in the running window.
- **Markdown mode:** the switch in the bottom right turns a tab into a Markdown editor with highlighting and lists that continue on Enter, next to a live preview in GitHub's dark style. The switch at the top shows the editor, both, or only the preview. A format bar adds headings, lists, bold, italic, strikethrough, links and tables.
- **Markdown files** (`.md`) open as their preview. Saving suggests `.md` while Markdown mode is on and `.txt` otherwise.
- **English and German,** switchable in the settings.
