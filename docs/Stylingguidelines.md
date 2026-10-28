# Styling guidelines: the neon family

These are instructions for building an app in the same neon-on-black style as **Dystopoly**, the owner's property-trading board game, so that all of the owner's apps look like one family. This file has everything you need: the rules, the colors, the fonts, a stylesheet to copy, and the logo template.

The reference implementation is the Dystopoly client (`github.com/YodelingGoethe/Dystopoly`):
- `apps/client/src/styles.css`: the theme
- `apps/client/src/theme.ts`: the palette
- `apps/client/src/components/icons.tsx`: the icons
- `apps/client/src-tauri/icons-src/icon.svg`: the app icon

If this file and Dystopoly disagree, Dystopoly wins. Fix this file when that happens.

## The look in one paragraph

It looks like neon signs in a dark room. The background is almost black (`#030308`), with a faint blue-violet glow in the middle. Everything that matters is drawn as a thin glowing tube in one saturated color: borders, text, icons, buttons. There are no filled buttons, no white or grey surfaces, and no drop shadows that aren't glows. Body text is calm, pale blue-white and has no glow. The glow is saved for things you can interact with and for the moments that matter.

## Rules

1. **Copy the stylesheet** below as the base of the app. Don't restyle its primitives (`.btn`, `.neon-box`, `.neon-text`, inputs, toggles); build on them.
2. **One color per glowing element, set through `--c`.** Every glow, border and tint is derived from `--c` with `color-mix()`. To color something, set `--c` on it; don't write new shadow stacks.
3. **Use the color roles:**
   - cyan `--line` for frames
   - green `--ok` for the main action
   - red `--danger` for destructive things and losses
   - gold `--money` for numbers that matter
   - pink `--logo` only for the logo
4. **Two fonts only:** Tilt Neon for things that glow (titles, buttons, big numbers, the logo), Quicksand for everything else.
5. **Outline icons only,** drawn like neon tubes: stroke, no fill, round ends.
6. **Desktop apps fill the window and never scroll the whole page.** Panels scroll inside themselves. The layout has to fit a laptop at 150% display scaling (see Layout).
7. **Nothing important happens silently:** changes are animated, and important events wait for the user to confirm them (see "Lessons from playtesting").

## Colors

| Token | Value | Use |
| --- | --- | --- |
| `--bg` | `#030308` | page background (under a radial gradient, see `body`) |
| `--line` | `#4cc9ff` | panel frames, focus rings, default glow, normal buttons |
| `--text` | `#e8f3ff` | body text |
| `--muted` | `#7f8bab` | labels, hints, secondary text, disabled switches |
| `--ok` | `#36ff8a` | primary button, success, gains, "your turn" |
| `--danger` | `#ff3b5c` | destructive buttons, errors, losses, warnings that block |
| `--money` | `#ffd84d` | money, scores, key numbers, timers, soft warnings |
| `--logo` | `#ff3fb4` | the logo and app icon only |
| `--panel-bg` | `rgba(8, 10, 22, 0.78)` | panel fill |
| `--popup-bg` | `#070912` | opaque fill for menus, modals and dialogs |

**Category colors.** Use these when things need telling apart, such as players, teams, categories or series. They come from Dystopoly's player tokens and property groups.

- **First choice, up to six:** Dystopoly's player colors, the most distinct from each other:
  - `#ff2d6f` pink-red
  - `#ffb627` amber
  - `#00ffd0` aqua
  - `#3f8cff` blue
  - `#a6ff00` lime
  - `#c77dff` violet
- **Eight more:** Dystopoly's property groups:
  - `#d0844c` brown
  - `#8be9fd` light blue
  - `#ff5fd2` pink
  - `#ff9a1f` orange
  - `#ff3b4e` red
  - `#ffe94d` yellow
  - `#36ff8a` green
  - `#5468ff` indigo

  The green and red are close to `--ok` and `--danger`, so don't use them for an entity on a screen where green and red already mean success and loss.

- An entity keeps its color everywhere: its chip, its icon, its marks on a board, its rows in lists. Never recolor an entity by context.
- Keep a single view to about four glow colors, plus cyan frames. More turns into noise.
- Glowing **text** is never the raw color. Mix it with white, as `.neon-text` does: `color-mix(in srgb, var(--c) 30%, white)`, with the raw color in the `text-shadow`. Icons, borders and small accents may use the raw color.
- Derived shades always come from `color-mix(in srgb, var(--c) N%, white | transparent)`, never from new hex values:
  - **mixed with white:** 25–40% for text, 55–60% for borders
  - **mixed with transparent:** 8–14% for hover fills, 35–70% for glows

## Typography

- **Tilt Neon** (`--font-neon`, weight 400 only): the logo, panel titles, buttons, headings, big numbers, and status words like "Your turn".
- **Quicksand** (`--font-ui`, variable weight): everything else. The base size is 14px. Labels are 12px `--muted` with `letter-spacing: 0.04em`. Use weight 500–600 for emphasis, never bold Tilt Neon.
- Titles get slight letter spacing (0.03–0.12em). Text is never all caps, except the logo.
- Load the fonts locally so desktop apps work offline:

  ```bash
  npm i @fontsource/tilt-neon @fontsource-variable/quicksand
  ```
  ```ts
  import "@fontsource/tilt-neon";
  import "@fontsource-variable/quicksand";
  ```

  Without npm, use Google Fonts: `https://fonts.googleapis.com/css2?family=Quicksand:wght@300..700&family=Tilt+Neon&display=swap`. The font stack in the stylesheet covers both names ("Quicksand Variable" from fontsource, "Quicksand" from Google).

## The logo

The logo has two parts, and every app in the family reuses both. Only the app's name and its initial change; the colors, font and frame stay the same, which is what makes the apps recognizable as one family.

### Wordmark

The app's name in capitals, in Tilt Neon, pink, with wide letter spacing:

```html
<h1 class="logo hero neon-text flicker">APPNAME</h1>   <!-- start screen: big, with the flicker -->
<h1 class="logo neon-text">APPNAME</h1>                <!-- other screens: smaller, steady -->
```

- The start screen shows the big version (84px, `.flicker`), centered near the top. Other full screens show the 44px version at the top. Inside the main view, the logo may sit faded in an empty center area, as Dystopoly does in the middle of its board, but hide it when content needs the space.
- `.flicker` goes only on the start screen's logo. Nothing else flickers.
- No tagline inside the wordmark, no icon in front of it, no other color.

### App icon

A dark rounded square with a cyan neon frame, and the app's initial as a pink neon tube with a pale core. Here is the template on a 1024×1024 canvas. Replace both `LETTER_PATH`s with the letter's path:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
  <defs>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="18" result="wide"/>
      <feGaussianBlur in="SourceGraphic" stdDeviation="6" result="near"/>
      <feMerge><feMergeNode in="wide"/><feMergeNode in="near"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>
  <rect x="40" y="40" width="944" height="944" rx="200" fill="#04040c"/>
  <rect x="96" y="96" width="832" height="832" rx="150" fill="none" stroke="#4cc9ff" stroke-width="14" filter="url(#glow)"/>
  <path d="LETTER_PATH" fill="none" stroke="#ff3fb4" stroke-width="44" stroke-linejoin="round" stroke-linecap="round" filter="url(#glow)"/>
  <path d="LETTER_PATH" fill="none" stroke="#ffd6ef" stroke-width="14" stroke-linejoin="round" stroke-linecap="round"/>
</svg>
```

Dystopoly's own icon is this template with its D. These letters are tested and look right:

| Letter | `LETTER_PATH` |
| --- | --- |
| D (Dystopoly) | `M352 268 H520 C688 268 776 382 776 512 C776 642 688 756 520 756 H352 Z` |
| A | `M300 756 L512 268 L724 756 M376 588 H648` |
| C | `M732 340 C690 292 620 268 548 268 C400 268 300 370 300 512 C300 654 400 756 548 756 C620 756 690 732 732 684` |
| K | `M340 268 V756 M708 268 L340 560 M470 456 L724 756` |
| M | `M300 756 V268 L512 540 L724 268 V756` |
| S | `M716 336 C676 286 610 268 528 268 C408 268 316 318 316 396 C316 474 400 498 512 516 C624 534 708 560 708 640 C708 718 616 756 496 756 C410 756 340 734 300 684` |

To draw another letter:
- **Shape:** it's a bent glass tube, not a font glyph, so keep it to straight lines and smooth cubic curves in one or a few strokes. Use a single initial; two letters get unreadable at 16px.
- **Size:** the centerline runs from y = 268 to y = 756, roughly x = 300 to 776, centered on x = 512.
- **Stroke:** keep both layers. The pink layer is 44 wide with the glow filter; the pale core is 14 wide with no filter.
- **Check it:** render it at 256, 32 and 16px and look at it before you're done.

For a Tauri app, save it as `src-tauri/icons-src/icon.svg` and run `npx tauri icon src-tauri/icons-src/icon.svg`. For anything else, export a 1024px PNG and use it as the app icon and favicon.

## The `--c` glow system

Every glowing element reads its color from the CSS variable `--c`, and the primitives derive borders, text color, glows and hover fills from it. So recoloring something is one line:

```html
<button class="btn" style="--c: #c77dff">Violet button</button>
<section class="panel neon-box" style="--c: var(--money)">Gold panel</section>
<span class="neon-text" style="--c: var(--ok)">Your turn</span>
<svg class="glow" style="--c: #00ffd0; color: #00ffd0">…</svg>
```

In React, `--c` needs a cast: `style={{ "--c": color } as CSSProperties}`. Keep the palette in one `theme.ts` and look colors up there; don't scatter hex values through components.

## Stylesheet

Copy this file as the app's base stylesheet, e.g. `src/styles.css`. App-specific styles go below it or in their own file and reuse its tokens and classes.

```css
/* Neon-on-black theme (Dystopoly family). Glowing elements take their color from --c. */

:root {
  --bg: #030308;
  --line: #4cc9ff;                 /* frames, focus rings, default glow */
  --text: #e8f3ff;
  --muted: #7f8bab;
  --ok: #36ff8a;                   /* primary actions, success, gains */
  --danger: #ff3b5c;               /* destructive actions, errors, losses */
  --money: #ffd84d;                /* amounts, scores, key numbers, warnings */
  --logo: #ff3fb4;                 /* the logo only */
  --panel-bg: rgba(8, 10, 22, 0.78);
  --popup-bg: #070912;
  --font-ui: "Quicksand Variable", "Quicksand", "Segoe UI", system-ui, sans-serif;
  --font-neon: "Tilt Neon", "Quicksand Variable", "Quicksand", system-ui, sans-serif;
  color-scheme: dark;
}

* { box-sizing: border-box; }
html, body, #root { height: 100%; margin: 0; }
body {
  background: radial-gradient(ellipse at 50% 40%, #0c0c1e 0%, #050510 55%, var(--bg) 100%);
  color: var(--text);
  font-family: var(--font-ui);
  font-size: 14px;
  overflow: hidden;
  user-select: none;
  -webkit-font-smoothing: antialiased;
}
button { font: inherit; color: inherit; }

/* ── Neon primitives ─────────────────────────────────────────────── */

.glow { filter: drop-shadow(0 0 1.5px var(--c, currentColor)) drop-shadow(0 0 5px var(--c, currentColor)); }

.neon-text {
  --c: var(--line);
  font-family: var(--font-neon);
  color: color-mix(in srgb, var(--c) 30%, white);
  text-shadow: 0 0 1px #fff, 0 0 6px var(--c), 0 0 16px var(--c), 0 0 32px color-mix(in srgb, var(--c) 50%, transparent);
}

.neon-box {
  --c: var(--line);
  border: 1.5px solid color-mix(in srgb, var(--c) 55%, white);
  border-radius: 12px;
  box-shadow:
    0 0 2px var(--c),
    0 0 12px color-mix(in srgb, var(--c) 55%, transparent),
    inset 0 0 8px color-mix(in srgb, var(--c) 35%, transparent);
}

.flicker { animation: flicker 6s infinite; }
@keyframes flicker {
  0%, 18%, 22%, 25%, 53%, 57%, 100% { opacity: 1; }
  20%, 24%, 55% { opacity: 0.55; }
}

/* The wordmark: the app's name, in capitals. */
.logo { --c: var(--logo); margin: 0; font-weight: 400; letter-spacing: 0.12em; font-size: 44px; }
.logo.hero { font-size: 84px; }

.muted { color: var(--muted); }
.hint { color: var(--muted); font-size: 13px; text-align: center; }
.small { font-size: 12px; }
.nowrap { white-space: nowrap; }
.danger-text { color: var(--danger); --c: var(--danger); }

.tag {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 1px 8px; border-radius: 99px; font-size: 12px;
  border: 1px solid color-mix(in srgb, var(--c, var(--line)) 60%, transparent);
}
.chip { display: inline-flex; align-items: center; gap: 6px; color: color-mix(in srgb, var(--c) 40%, white); }

.big-number {
  font-family: var(--font-neon);
  font-size: 30px;
  color: #fff6cf;
  text-shadow: 0 0 2px #fff, 0 0 10px var(--money), 0 0 24px color-mix(in srgb, var(--money) 50%, transparent);
}

/* ── Buttons and inputs ──────────────────────────────────────────── */

.btn {
  --c: var(--line);
  background: transparent;
  border: 1.5px solid color-mix(in srgb, var(--c) 60%, white);
  border-radius: 10px;
  padding: 8px 18px;
  color: color-mix(in srgb, var(--c) 25%, white);
  font-family: var(--font-neon);
  font-size: 15px;
  letter-spacing: 0.03em;
  cursor: pointer;
  box-shadow: 0 0 2px var(--c), 0 0 8px color-mix(in srgb, var(--c) 40%, transparent);
  text-shadow: 0 0 6px var(--c);
  transition: background 0.15s, box-shadow 0.15s, opacity 0.15s;
}
.btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--c) 14%, transparent);
  box-shadow: 0 0 3px var(--c), 0 0 18px color-mix(in srgb, var(--c) 70%, transparent), inset 0 0 10px color-mix(in srgb, var(--c) 30%, transparent);
}
.btn:disabled { opacity: 0.32; cursor: not-allowed; }
.btn.primary { --c: var(--ok); }
.btn.danger { --c: var(--danger); }
.btn.big { padding: 11px 30px; font-size: 18px; }
.btn.small { padding: 4px 10px; font-size: 13px; border-radius: 8px; }
.btn.ghost { border-color: transparent; box-shadow: none; }
/* A button that fires by itself when the bar runs out: <button class="btn primary counting">End turn (3)<span class="countdown-bar" style="width: 60%"></span></button> */
.btn.counting { position: relative; overflow: hidden; }
.countdown-bar {
  position: absolute; left: 0; bottom: 0; height: 3px; transition: width 0.1s linear;
  background: color-mix(in srgb, var(--c) 60%, white); box-shadow: 0 0 6px var(--c);
}

.icon-btn {
  background: none; border: none; cursor: pointer; padding: 4px; display: grid; place-items: center;
  color: var(--text); filter: drop-shadow(0 0 4px var(--line));
}

.button-row { display: flex; flex-wrap: wrap; gap: 10px; justify-content: center; align-items: center; }

.field { display: flex; flex-direction: column; gap: 5px; }
.field > span { font-size: 12px; color: var(--muted); letter-spacing: 0.04em; }

input, select, textarea {
  background: rgba(4, 6, 16, 0.9);
  border: 1px solid color-mix(in srgb, var(--line) 35%, transparent);
  border-radius: 8px;
  padding: 8px 10px;
  color: var(--text);
  font: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}
input:focus, select:focus, textarea:focus {
  border-color: var(--line);
  box-shadow: 0 0 10px color-mix(in srgb, var(--line) 45%, transparent);
}
input:disabled, select:disabled, textarea:disabled { opacity: 0.6; }
select option { background: #0a0c18; }

/* <label class="toggle"><input type="checkbox"><span class="switch"></span>Label</label> */
.toggle { display: flex; align-items: center; gap: 10px; cursor: pointer; padding: 4px 0; }
.toggle input { display: none; }
.toggle .switch {
  width: 34px; height: 18px; border-radius: 99px; flex: none; position: relative;
  border: 1.5px solid color-mix(in srgb, var(--muted) 70%, transparent);
  transition: all 0.2s;
}
.toggle .switch::after {
  content: ""; position: absolute; top: 2px; left: 2px; width: 11px; height: 11px; border-radius: 50%;
  background: var(--muted); transition: all 0.2s;
}
.toggle input:checked + .switch { border-color: var(--ok); box-shadow: 0 0 8px var(--ok); }
.toggle input:checked + .switch::after { left: 18px; background: #eafff2; box-shadow: 0 0 6px var(--ok); }
.toggle input:disabled + .switch { opacity: 0.5; }

/* Settings that start collapsed:
   <details class="settings-group"><summary><span>Experimental</span><span class="changed-mark">changed</span></summary>
   <div class="settings-group-body">fields and toggles</div></details> */
.settings-group { border-top: 1px solid color-mix(in srgb, var(--line) 20%, transparent); padding-top: 10px; }
.settings-group summary {
  display: flex; align-items: center; gap: 10px; cursor: pointer; list-style: none;
  font-weight: 500; font-size: 13px; color: var(--muted); letter-spacing: 0.05em;
}
.settings-group summary::-webkit-details-marker { display: none; }
.settings-group summary::before { content: "▸"; color: var(--line); transition: transform 0.15s; }
.settings-group[open] summary::before { transform: rotate(90deg); }
.settings-group summary:hover { color: var(--text); }
.settings-group-body { display: flex; flex-direction: column; gap: 12px; padding-top: 12px; }
.changed-mark {
  font-size: 11px; padding: 1px 8px; border-radius: 99px; color: var(--money);
  border: 1px solid color-mix(in srgb, var(--money) 50%, transparent);
}

/* Two or more equal tabs: <div class="tabs"><button class="on">…</button><button>…</button></div> */
.tabs {
  display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--line) 40%, transparent); border-radius: 10px;
}
.tabs button {
  background: none; border: none; padding: 8px; cursor: pointer; font-size: 14px;
  color: var(--muted); letter-spacing: 0.06em; transition: all 0.15s;
}
.tabs button.on { color: #fff; background: color-mix(in srgb, var(--line) 20%, transparent); text-shadow: 0 0 6px var(--line); }

/* Small pill switch, e.g. the EN | DE language toggle. */
.pill-toggle { display: inline-flex; overflow: hidden; border: 1px solid color-mix(in srgb, var(--line) 40%, transparent); border-radius: 99px; }
.pill-toggle button { background: none; border: none; padding: 4px 11px; cursor: pointer; color: var(--muted); font-size: 12px; letter-spacing: 0.08em; }
.pill-toggle button.on { color: #fff; background: color-mix(in srgb, var(--line) 22%, transparent); text-shadow: 0 0 6px var(--line); }

/* ── Panels and screens ──────────────────────────────────────────── */

/* <section class="panel neon-box"><h2 class="panel-title">…</h2>…</section> */
.panel { background: var(--panel-bg); padding: 14px; }
.panel-title {
  margin: 0 0 10px; font-family: var(--font-neon); font-weight: 400; font-size: 16px;
  letter-spacing: 0.06em; color: color-mix(in srgb, var(--line) 30%, white);
  text-shadow: 0 0 8px var(--line);
}
.sub-title { margin: 16px 0 8px; font-weight: 500; font-size: 13px; color: var(--muted); letter-spacing: 0.05em; }

.scroll { overflow-y: auto; min-height: 0; }
.scroll::-webkit-scrollbar { width: 6px; }
.scroll::-webkit-scrollbar-thumb { background: color-mix(in srgb, var(--line) 40%, transparent); border-radius: 3px; }

/* A full-window page (start screen, settings) that scrolls itself when the window is short. */
.screen {
  --pad-y: 48px;
  --pad-x: 24px;
  height: 100%; overflow-y: auto;
  display: flex; flex-direction: column; align-items: center; gap: 18px;
  padding: var(--pad-y) var(--pad-x);
}
/* Centered, but scrollable from the top when too short (unlike justify-content: center). */
.screen.centered > :first-child { margin-top: auto; }
.screen.centered > :last-child { margin-bottom: auto; }
/* The main buttons stay in view at the bottom while long content scrolls. */
.screen-actions {
  position: sticky; bottom: calc(-1 * var(--pad-y)); z-index: 2;
  width: calc(100% + 2 * var(--pad-x)); margin: 6px calc(-1 * var(--pad-x)) calc(-1 * var(--pad-y));
  padding: 18px var(--pad-x) var(--pad-y);
  background: linear-gradient(to bottom, transparent, rgb(3 3 8 / 0.94) 32%);
}
@media (max-height: 760px) {
  .screen { --pad-y: 24px; gap: 12px; }
  .logo { font-size: 34px; }
  .logo.hero { font-size: 64px; }
}
@media (max-height: 640px) {
  .logo.hero { font-size: 52px; }
}

/* ── Popups ──────────────────────────────────────────────────────── */

.modal-backdrop {
  position: fixed; inset: 0; z-index: 30; display: grid; place-items: center;
  background: rgba(1, 1, 6, 0.72); backdrop-filter: blur(3px);
  animation: fadeIn 0.15s ease-out;
}
/* <div class="modal-backdrop"><div class="modal neon-box">…</div></div> */
.modal {
  display: flex; flex-direction: column; align-items: center; gap: 14px;
  max-width: min(420px, 94vw); max-height: 92vh; padding: 22px 26px;
  background: var(--popup-bg); text-align: center; animation: popIn 0.3s ease-out;
}

/* <div class="menu"><button class="icon-btn">☰</button><div class="menu-pop neon-box">buttons, hr, .toggle</div></div> */
.menu { position: relative; }
.menu-pop {
  position: absolute; right: 0; top: 34px; z-index: 20; min-width: 230px;
  background: var(--popup-bg); display: flex; flex-direction: column; padding: 6px;
}
.menu-pop button { background: none; border: none; text-align: left; padding: 8px 12px; border-radius: 8px; cursor: pointer; }
.menu-pop button:hover { background: color-mix(in srgb, var(--line) 14%, transparent); }
.menu-pop .toggle { padding: 8px 12px; }
.menu-pop hr { width: 100%; border: none; border-top: 1px solid color-mix(in srgb, var(--line) 20%, transparent); margin: 4px 0; }

.toasts {
  position: fixed; bottom: 22px; left: 50%; transform: translateX(-50%); z-index: 40;
  display: flex; flex-direction: column; gap: 8px; align-items: center; pointer-events: none;
}
.toast {
  background: #0b0710; border: 1px solid var(--danger); color: #ffd5dd; padding: 8px 16px; border-radius: 10px;
  box-shadow: 0 0 14px color-mix(in srgb, var(--danger) 50%, transparent); animation: fadeIn 0.2s;
}
.toast.warn { border-color: var(--money); color: #fff3c4; box-shadow: 0 0 14px color-mix(in srgb, var(--money) 50%, transparent); }

/* ── Motion ──────────────────────────────────────────────────────── */

@keyframes fadeIn { from { opacity: 0; } }
@keyframes popIn { from { opacity: 0; transform: scale(0.94); } }
/* Something important that just happened, shown big. */
.flash { animation: flashIn 0.35s cubic-bezier(0.2, 0.9, 0.3, 1.25); }
@keyframes flashIn { from { opacity: 0; transform: scale(0.7); } }
/* A number that changed: <span class="delta up">+500</span> next to it. */
.delta { font-family: var(--font-neon); animation: deltaUp 2.2s ease-out forwards; }
.delta.up { color: var(--ok); text-shadow: 0 0 8px var(--ok); }
.delta.down { color: var(--danger); text-shadow: 0 0 8px var(--danger); }
@keyframes deltaUp {
  0% { opacity: 0; transform: translateY(8px); }
  15% { opacity: 1; transform: none; }
  80% { opacity: 1; }
  100% { opacity: 0; transform: translateY(-10px); }
}
/* "This one is active": a slow breathing ring, never a glitch. Needs position: relative on the element. */
.active-ring::after {
  content: ""; position: absolute; inset: -16%; border-radius: 50%; pointer-events: none;
  border: 1.5px solid color-mix(in srgb, var(--c) 70%, transparent);
  box-shadow: 0 0 8px color-mix(in srgb, var(--c) 45%, transparent);
  animation: activePulse 2.6s ease-in-out infinite;
}
@keyframes activePulse {
  0%, 100% { opacity: 0.35; transform: scale(0.94); }
  50% { opacity: 0.85; transform: scale(1); }
}

@media (prefers-reduced-motion: reduce) {
  .flicker, .active-ring::after { animation: none; }
}
```

How the pieces combine:
- **Panel:** `<section class="panel neon-box"><h2 class="panel-title">Players</h2>…</section>`. Add `scroll` to a part inside it that can grow.
- **Buttons:**
  - one `.btn.primary` per view, for the main action (`.btn.primary.big` when it's the one thing to do)
  - `.btn` for the rest
  - `.btn.danger` for destroying or giving up
  - `.btn.small` inside lists
  - put them in a `.button-row`
- **Field:** `<label class="field"><span>Your name</span><input></label>`.
- **Settings:** the few everyone needs stay visible. Everything else goes into `.settings-group`s that start collapsed, and a group's header gets a `.changed-mark` when something inside differs from the defaults, so nobody misses a changed rule.
- **Dialogs:** `.modal-backdrop > .modal.neon-box`, with a `.neon-text` title, a `.hint`, and a `.button-row`. A destructive confirmation gets a `danger-text` title and a `.btn.danger`. Render dialogs into `body` (a React portal), so a fixed or sticky parent can't cover them.
- **Toasts:** inside `.toasts`; a `.toast` for errors, `.toast.warn` for warnings. They're for connection trouble and errors only. Game or app events get their own visible moment (see below).
- **Language switch** (if the app has more than one language): a `.pill-toggle` in the top right corner, like Dystopoly's EN | DE.

## Icons

- Line icons on a 24×24 viewBox, with `fill="none"`, `stroke="currentColor"`, `stroke-width="1.8"`, round caps and round joins. Larger pictures (vehicles, pieces, mascots) use a 64×64 viewBox with the same style.
- Color comes from `color: var(--c)`, and the glow from the `.glow` class.
- Draw them by hand like Dystopoly's `icons.tsx`, or use [Lucide](https://lucide.dev) with `strokeWidth={1.8}`, which matches. No filled icon sets, no icon fonts, no emoji in the interface.
- Objects seen on a board or map are drawn from above, nose up, and rotated in CSS to face their direction.

## Layout

- **Full window, no page scroll.** `body` has `overflow: hidden`, and each screen or panel scrolls itself (`.screen`, `.scroll`). Long pages keep their main buttons in a sticky `.screen-actions` bar at the bottom.
- **Columns** use `clamp()` widths around a flexible middle. Dystopoly uses:

  ```css
  grid-template-columns: clamp(210px, 20vw, 290px) minmax(0, 1fr) clamp(220px, 22vw, 320px);
  gap: 14px; padding: 14px;
  ```

  Reduce the gap and padding to 10px below 1300×720.
- **A centerpiece that scales** (a board, a canvas, a big card) is sized in container query units (`container-type: size` on its box, `cqw` inside), so it scales as one piece.
- **It has to fit** at these window sizes in CSS pixels. Check each one with a screenshot at device pixel ratio 1.5:
  - 1093×560: a 1366×768 laptop at 125% scaling
  - 1280×650: a 1920×1080 laptop at 150% scaling
  - 1440×900: the default window

  No button may end up outside the window. If something doesn't fit, stack it (container queries on the panel) or move it into a menu. Don't shrink the text below 12px.
- **Desktop window (Tauri):**
  - default size 1440×900, minimum 900×540
  - `"backgroundColor": "#030308"` in the window config, and `<html style="background:#030308">` in `index.html`, so no white flashes at start
  - when the screen's work area is smaller than the default size, start maximized
  - F11 toggles fullscreen, and `"zoomHotkeysEnabled": true` gives Ctrl +/− interface zoom
- **Top right corner:** a ☰ menu (`.menu-pop`) with settings toggles, fullscreen and leave. Next to it, the language switch. On the start or lobby screen, the app version sits there too as a small muted button (`v1.2.3`) that opens the changelog.

## Motion and feedback

- **Transitions:** 0.12–0.2s on hover, focus and toggles.
- **Entrances:**
  - `fadeIn` (0.15s) for backdrops and toasts
  - `popIn` (0.3s) for dialogs and panels that appear
  - `.flash` (0.35s with a slight overshoot) for a big announcement
- **Changed numbers** get a `.delta.up` (`+500`) or `.delta.down` next to them, which floats up and fades out over 2.2s.
- **The active thing** (whose turn it is, the selected item) gets the slow `.active-ring` pulse (2.6s), never a glitch or a fast blink.
- **Looping animations** are limited to the start screen logo's flicker and the active pulse. Under `prefers-reduced-motion` both stop (already in the stylesheet).

## Lessons from playtesting Dystopoly

The owner and friends played Dystopoly twice and asked for these changes. Build them in from the start:

- **Don't be too fast.** When one action causes several things, play them one after another: for example the dice roll, then the piece moving, then the payment. Show each one before the next. Hide buttons until it's done, so nobody clicks ahead.
- **Make changes to important numbers obvious.** Show a big +/− amount in the middle of the screen for a moment (green up, red down), not only a quiet change in a corner.
- **Popups wait for confirmation.** Something that affects the user (a card, a payment, a penalty) is shown first and only happens after they click **OK** or a button that says what will happen ("Pay Ð 6,000 to Ben"). Popups don't disappear on their own, and they're short: a title, one line, a button. Fast mode (below) is the exception.
- **Colors that mean identity stay fixed.** An owner's color is always the owner's color, whatever it sits on.
- **Subtle beats flashy** for anything that stays on screen. A slight pulse is enough.
- **Zoom and fullscreen.** If there's a big centerpiece, let the user zoom it (wheel at the cursor, drag to pan, +/− buttons) and offer fullscreen.
- **Keep it moving.** Anything that waits on several people gets a relaxed timer, e.g. 15s that restarts on every action, and shows it as a draining bar.
- **Don't make people click through routine steps.** A step that's almost always the same (ending a turn) counts down a few seconds on its button (`.btn.counting`) and then happens by itself. Clicking does it at once, and the countdown waits while a dialog is open.
- **Announce whose turn it is.** When control passes to another person, show it big for a moment ("It's Anna's turn!" in their color), especially when several people share one screen.
- **Make the key random moment a show.** The dice are thrown over the board, tumble, and then show big with the result in plain text ("4 + 3 = 7"), so nobody has to count pips.
- **Offer a fast mode.** Once people know the game, the pacing above gets slow. A "fast mode" switch shortens every animation, shows results at once, and turns confirmations into short flashes that don't wait. It's off by default.

## Checklist

Before you call the styling done:

- [ ] The stylesheet above is the base, and colors are set through `--c` from one palette file.
- [ ] Only the two fonts are used, and they load without internet.
- [ ] The logo is the app name in the wordmark style, and the app icon comes from the template with the app's initial.
- [ ] There's exactly one `.btn.primary` per view.
- [ ] Every icon is a stroke-only line icon.
- [ ] No white or grey surfaces, no filled buttons, no non-glow shadows.
- [ ] Screenshots at 1093×560 and 1280×650 (device pixel ratio 1.5) show every button inside the window.
- [ ] Important events are animated in sequence, and anything that affects the user waits for a confirmation.
