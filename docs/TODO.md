# TODO
## Bugs


## general changes:
- ctrl + alt does not work the same as alt Gr. it should work for things like [,],{,}, etc (german keyboard)
  - Postponed (2026-10-31). Why it's hard: Windows already turns Ctrl+Alt+8 into "[", but egui-winit, the library between Windows and the app, throws away any character typed while Ctrl is held, so that Ctrl+C doesn't type a "c" (`egui-winit` 0.37, `src/lib.rs`, the `is_cmd` check). AltGr works only because Windows doesn't report it as Ctrl. DystoPad never sees the character, so it can't put it back. The fix is one line in egui-winit, but that means building from a patched copy of egui (a fork or a copy in this repo) until egui releases the fix, which the owner doesn't want. Without a patch, DystoPad can rebuild only `[ ] { } \ |` from the key that was pressed; `@ € ~ µ` would still not work. Next time: check whether a newer egui has fixed it, or ask egui upstream to fix it.

## Post work tasks:
once you're done with these features, update CLAUDE.md, STATUS.md and the changelog. Give it a new version number and push to github. then run a local build as well so that the bare executable sits in the release folder. Then delete all finished items from TODO.md (this document).