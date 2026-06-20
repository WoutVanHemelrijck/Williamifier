# Design Guide — Double Pendulum UI

Use this document to recreate the visual style of this interface on other web pages.

---

## Design Philosophy

Minimal, dark, technical. No decorative elements. Every pixel serves a function. Typography carries the hierarchy — not color, gradients, or shadows. Spacing is tight but not cramped.

---

## Color Palette

```css
:root {
  --bg:           #000000;               /* page background */
  --surface:      #0a0a0a;               /* slightly lifted surfaces (control bar) */
  --border:       rgba(255,255,255,0.08); /* default border / divider */
  --border-hover: rgba(255,255,255,0.15); /* border on hover / focus */
  --text:         #ededed;               /* primary text */
  --text-muted:   #666666;               /* secondary / label text */
  --text-dim:     #333333;               /* tertiary / barely visible text */
  --accent:       #ffffff;               /* pure white — used for active indicators */
}
```

**Semantic colors (buttons only):**

| Role    | Text       | Border                     | Background                  |
|---------|------------|----------------------------|-----------------------------|
| Play    | `#4ade80`  | `rgba(74,222,128,0.35)`    | `rgba(74,222,128,0.08)`     |
| Pause   | `#f87171`  | `rgba(248,113,113,0.35)`   | `rgba(248,113,113,0.08)`    |

No other semantic colors. Everything else is white at varying opacity.

---

## Typography

```css
font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
font-size: 13px;
-webkit-font-smoothing: antialiased;
```

Import:
```html
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
```

| Usage              | Size  | Weight | Color           | Notes                          |
|--------------------|-------|--------|-----------------|--------------------------------|
| Page title         | 13px  | 600    | `--text`        | `letter-spacing: -0.01em`      |
| Tab buttons        | 13px  | 400    | `--text-muted`  | Active → `--text`              |
| Control labels     | 11px  | 500    | `--text-muted`  | `text-transform: uppercase`, `letter-spacing: 0.02em` |
| Control values     | 11px  | 400    | `--text`        | Monospace font                 |
| Inputs / selects   | 12px  | 400    | `--text`        |                                |
| Cell labels        | 10px  | 500    | `--text-dim`    | `text-transform: uppercase`, `letter-spacing: 0.06em` |
| Tooltip text       | 11px  | 400    | `--text`        | `line-height: 1.5`             |
| Monospace values   | 11px  | 400    | `--text-muted`  | `'SF Mono', 'Fira Code', monospace` |
| Modal title        | 13px  | 600    | `--text`        |                                |

---

## Layout Structure

```
┌─────────────────────────────────────────────────────┐
│  NAV BAR  (48px tall)                               │
│  Title · Tab · Tab                                  │
├─────────────────────────────────────────────────────┤
│  CONTROL BAR  (44px tall)                           │
│  Label  Input  │  Label  Input  │  Btn  Btn  ···    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  CONTENT AREA  (flex: 1, fills remaining height)    │
│                                                     │
│  Grid cells separated by 1px --border lines         │
│                                                     │
└─────────────────────────────────────────────────────┘
```

- `body`: `display: flex; flex-direction: column; height: 100%; overflow: hidden`
- Nav and control bar: `flex-shrink: 0`
- Content area: `flex: 1; min-height: 0`
- Grid gaps use `gap: 1px; background: var(--border)` on the grid container (cells have `background: var(--bg)`)

---

## Nav Bar

```css
#nav {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 16px;
  height: 48px;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}
```

- Title on the left: `font-size: 13px; font-weight: 600; margin-right: 16px`
- Tabs follow immediately, no gap fill between title and tabs

---

## Tab Buttons

```css
.tab-btn {
  background: transparent;
  border: none;
  border-radius: 6px;
  color: var(--text-muted);
  padding: 6px 10px;
  font-size: 13px;
  font-weight: 400;
  transition: color 0.15s, background 0.15s;
  position: relative;
}

.tab-btn:hover {
  color: var(--text);
  background: rgba(255,255,255,0.05);
}

/* Active state: text color + underline indicator */
.tab-btn.active { color: var(--text); }
.tab-btn.active::after {
  content: '';
  position: absolute;
  bottom: -1px;       /* sits on top of the nav border-bottom */
  left: 6px;
  right: 6px;
  height: 1px;
  background: var(--accent);
  border-radius: 1px;
}

.tab-btn:disabled { opacity: 0.3; cursor: not-allowed; }
```

The active underline "bleeds" over the nav's bottom border — `bottom: -1px` — to visually connect the tab to its content.

---

## Control Bar

```css
.ctrl-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 16px;
  height: 44px;
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-wrap: nowrap;
}
```

### Control Bar Elements

**Label** — uppercase small text before an input group:
```css
.ctrl-label {
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  white-space: nowrap;
}
```

**Value readout** — shows the current value of a range slider:
```css
.ctrl-value {
  color: var(--text);
  font-size: 11px;
  font-family: 'SF Mono', 'Fira Code', monospace;
  min-width: 20px;
}
```

**Separator** — vertical rule between groups:
```css
.ctrl-sep {
  width: 1px;
  height: 16px;
  background: var(--border-hover);
  flex-shrink: 0;
  margin: 0 4px;
}
```

**Spacer** — pushes remaining items to the right:
```css
.ctrl-spacer { flex: 1; min-width: 8px; }
```

---

## Buttons

Default button — ghost style, no fill:

```css
button {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-muted);
  padding: 5px 12px;
  font-size: 12px;
  font-family: inherit;
  font-weight: 400;
  white-space: nowrap;
  transition: color 0.15s, border-color 0.15s, background 0.15s;
}
button:hover {
  color: var(--text);
  border-color: var(--border-hover);
  background: rgba(255,255,255,0.04);
}
```

**Play button:**
```css
.btn-play {
  color: #4ade80;
  border-color: rgba(74,222,128,0.35);
  background: rgba(74,222,128,0.08);
  font-weight: 500;
}
.btn-play:hover {
  background: rgba(74,222,128,0.15);
  border-color: rgba(74,222,128,0.55);
}
```

**Pause button:**
```css
.btn-pause {
  color: #f87171;
  border-color: rgba(248,113,113,0.35);
  background: rgba(248,113,113,0.08);
  font-weight: 500;
}
.btn-pause:hover {
  background: rgba(248,113,113,0.15);
  border-color: rgba(248,113,113,0.55);
}
```

---

## Form Inputs

**Number input:**
```css
input[type="number"] {
  width: 56px;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  padding: 4px 8px;
  font-size: 12px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}
input[type="number"]:focus { border-color: var(--border-hover); }
input[type="number"]::-webkit-inner-spin-button { opacity: 0.3; }
```

**Range slider:**
```css
input[type="range"] {
  width: 80px;
  accent-color: var(--text);   /* white thumb and track fill */
  cursor: pointer;
}
```

**Select dropdown:**
```css
select {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  padding: 4px 8px;
  font-size: 12px;
  font-family: inherit;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s;
}
select:focus { border-color: var(--border-hover); }
select option { background: #111; color: var(--text); }
```

---

## Custom Colored Select (dot + label)

Used when options need a color swatch (e.g. "pick a pendulum" where each has a hue):

```html
<div class="custom-select">
  <button class="cs-btn">
    <span class="cs-dot" style="background: #ff0000"></span>
    <span class="cs-text">Pendulum 1</span>
    <svg><!-- chevron --></svg>
  </button>
  <div class="cs-dropdown">
    <div class="cs-option selected">
      <span class="cs-dot" style="background: #ff0000"></span>
      Pendulum 1
    </div>
    <!-- more options -->
  </div>
</div>
```

```css
.custom-select { position: relative; display: inline-flex; flex-shrink: 0; }

.cs-btn {
  display: inline-flex; align-items: center; gap: 6px;
  background: transparent; border: 1px solid var(--border); border-radius: 6px;
  color: var(--text); padding: 4px 8px; font-size: 12px; font-family: inherit;
  cursor: pointer; min-width: 120px; transition: border-color 0.15s;
}
.cs-btn:hover { border-color: var(--border-hover); }
.cs-text { flex: 1; text-align: left; }
.cs-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }

.cs-dropdown {
  display: none; position: absolute; top: calc(100% + 4px); left: 0;
  background: #111; border: 1px solid var(--border); border-radius: 6px;
  z-index: 150; min-width: 100%; overflow: hidden;
}
.cs-dropdown.open { display: block; }

.cs-option {
  display: flex; align-items: center; gap: 7px;
  padding: 6px 10px; font-size: 12px; color: var(--text);
  cursor: pointer; white-space: nowrap; transition: background 0.1s;
}
.cs-option:hover    { background: rgba(255,255,255,0.06); }
.cs-option.selected { background: rgba(255,255,255,0.04); }
```

---

## Tooltip / Help Icon

Small `?` circle that reveals a tooltip on hover. Place inline next to the control it describes.

```html
<span class="help-icon" data-tip="Your tooltip text here.">
  <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
    <circle cx="7" cy="7" r="6" stroke="currentColor" stroke-opacity="0.55"/>
    <text x="7" y="7.7" text-anchor="middle" font-size="7.5" font-weight="700"
          font-family="monospace" fill="currentColor" fill-opacity="0.75">?</text>
  </svg>
</span>
```

```css
.help-icon {
  display: inline-flex; align-items: center; justify-content: center;
  width: 14px; height: 14px;
  color: var(--text-muted); cursor: default; position: relative;
  flex-shrink: 0; user-select: none;
}

/* Tooltip bubble */
.help-icon::after {
  content: attr(data-tip);
  position: absolute;
  top: calc(100% + 8px);
  left: 50%; transform: translateX(-50%);
  background: #151515;
  border: 1px solid rgba(255,255,255,0.14);
  border-radius: 6px;
  padding: 7px 11px;
  font-size: 11px; line-height: 1.5;
  color: var(--text);
  white-space: normal; width: 200px;
  pointer-events: none; z-index: 200;
  opacity: 0; transition: opacity 0.15s;
  font-weight: 400; font-family: inherit;
}
.help-icon:hover::after { opacity: 1; }

/* Align tooltip to the left edge when near the left side */
.help-icon.tip-left::after { left: 0; right: auto; transform: none; }
```

---

## Content Grid Cells

Cells in a grid layout. The grid itself has `gap: 1px; background: var(--border)` so the gaps act as dividers.

```css
.quad-cell {
  position: relative;
  background: var(--bg);
  overflow: hidden;
}

/* Small label in the top-left corner of a cell */
.quad-label {
  position: absolute;
  top: 10px; left: 12px;
  font-size: 10px; font-weight: 500;
  letter-spacing: 0.06em; text-transform: uppercase;
  color: var(--text-dim);
  pointer-events: none; z-index: 1; user-select: none;
}
```

---

## Modal

Semi-transparent dark overlay with a compact dark card:

```css
/* Overlay */
#overlay {
  display: none;
  position: fixed; inset: 0; z-index: 500;
  background: rgba(0,0,0,0.72);
  align-items: center; justify-content: center;
}
#overlay.active { display: flex; }

/* Card */
#modal {
  background: #0d0d0d;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 20px 24px;
  width: 340px;
  display: flex; flex-direction: column; gap: 14px;
}

/* Modal header: title left, close button right */
.modal-header {
  display: flex; align-items: center; justify-content: space-between;
}
.modal-header span { font-size: 13px; font-weight: 600; color: var(--text); }

/* Close button */
.modal-close {
  border: none; background: none; padding: 2px 6px;
  color: var(--text-muted); font-size: 14px; cursor: pointer; border-radius: 4px;
}
.modal-close:hover { color: var(--text); background: rgba(255,255,255,0.06); }
```

**Form rows inside the modal:**
```css
.modal-row {
  display: flex; align-items: center; gap: 8px;
  font-size: 12px; color: var(--text-muted);
}
.modal-row label { width: 76px; flex-shrink: 0; }
.modal-row select,
.modal-row input[type="number"] { flex: 1; }
.modal-unit { color: var(--text-dim); font-size: 11px; }

/* Hint line below a form row */
.modal-hint {
  font-size: 11px; color: var(--text-dim);
  margin-top: -6px; padding-left: 84px;
  font-family: 'SF Mono', 'Fira Code', monospace;
}
```

---

## Progress Bar

Thin accent bar, used inside modals:

```css
.progress-wrap {
  height: 3px;
  background: rgba(255,255,255,0.08);
  border-radius: 2px;
  overflow: hidden;
}
.progress-bar {
  height: 100%;
  width: 0%;
  background: #4ade80;          /* green, same as btn-play */
  transition: width 0.08s linear;
  border-radius: 2px;
}
```

---

## Floating Hint (in-canvas)

Appears centered near the bottom of a canvas area:

```css
.map-hint {
  position: absolute;
  bottom: 14px; left: 50%; transform: translateX(-50%);
  font-size: 11px; color: rgba(255,255,255,0.55);
  background: rgba(0,0,0,0.45);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 6px;
  padding: 5px 12px;
  pointer-events: none; white-space: nowrap;
  letter-spacing: 0.02em;
  transition: opacity 0.4s;
}
.map-hint.hidden { opacity: 0; }
```

---

## "How It Works" Page

A scrollable long-form content page, accessed via a third nav tab. No control bar — the nav bar goes directly into the article. Uses KaTeX for math rendering.

### Setup

```html
<!-- In <head> -->
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" crossorigin="anonymous" />

<!-- Before </body> -->
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js" crossorigin="anonymous"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js" crossorigin="anonymous"
        onload="renderMathInElement(document.getElementById('page-tech'), {
          delimiters: [
            {left: '$$', right: '$$', display: true},
            {left: '$',  right: '$',  display: false}
          ],
          throwOnError: false
        })"></script>
```

Write math inline with `$...$` (inline) or `$$...$$` (display/block). KaTeX replaces them automatically on load.

### Page Container

```css
#page-tech {
  flex: 1;
  overflow-y: scroll;
  background: var(--bg);
  scrollbar-gutter: stable;   /* prevents layout shift when scrollbar appears */
}

.tech-content {
  max-width: 860px;
  margin: 0 auto;
  padding: 48px 40px 80px;
}
```

### Page Header

```css
.tech-content h1 {
  font-size: 22px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 10px;
  letter-spacing: -0.02em;
}

.tech-intro {
  font-size: 14px;
  line-height: 1.75;
  color: var(--text-muted);
  margin-bottom: 36px;
  max-width: 600px;
}
```

### Collapsible Sections (`<details>`)

Each topic is a `<details>` element with a styled `<summary>`. Sections stack with a single top border; the last one also gets a bottom border.

```html
<details class="tech-section" open>
  <summary class="tech-summary">
    <span class="tech-summary-label">Section title</span>
  </summary>
  <div class="tech-body">
    <p>Body text goes here. Math: $E = mc^2$</p>
    $$\ddot\theta_1 = \frac{-g(2m_1+m_2)\sin\theta_1 \cdots}{L_1 D}$$
  </div>
</details>
```

```css
details.tech-section {
  border-top: 1px solid var(--border);
}
details.tech-section:last-of-type {
  border-bottom: 1px solid var(--border);
}

.tech-summary {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 16px 0;
  cursor: pointer;
  list-style: none;
  user-select: none;
}
.tech-summary::-webkit-details-marker { display: none; }

.tech-summary-label {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text);
  flex-shrink: 0;
  transition: color 0.15s;
}
.tech-summary:hover .tech-summary-label { color: var(--accent); }

/* Chevron rotates 90° when open */
.tech-summary::after {
  content: '›';
  font-size: 18px;
  font-weight: 300;
  margin-left: auto;
  color: var(--text-muted);
  transition: transform 0.2s;
  line-height: 1;
  flex-shrink: 0;
}
details[open] > .tech-summary::after {
  transform: rotate(90deg);
}
```

### Body Text

```css
.tech-body {
  padding-bottom: 32px;
}

.tech-body p {
  font-size: 14px;
  line-height: 1.8;
  color: var(--text);
  margin-bottom: 16px;
  max-width: 600px;
}
.tech-body p:last-child { margin-bottom: 0; }
```

### KaTeX Math Styling

KaTeX renders `$...$` and `$$...$$` automatically. Two overrides keep it consistent with the rest of the UI:

```css
/* Block equations: add breathing room and allow horizontal scroll on narrow viewports */
.tech-body .katex-display {
  margin: 1.4em 0;
  overflow-x: auto;
  overflow-y: hidden;
}

/* Slightly scale up inline math to match 14px body text */
.tech-body .katex {
  font-size: 1.05em;
}
```

### Monospace Table / Code Block

Used for precision tables, GPU buffer layouts, error estimates — anything that needs fixed-width alignment:

```html
<div class="tech-table">                f64 (double)    f32 (single)
total bits           64              32
machine ε         2.22 × 10⁻¹⁶   1.19 × 10⁻⁷</div>
```

```css
.tech-table {
  font-family: 'SF Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 11.5px;
  line-height: 1.7;
  color: var(--text);
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px 18px;
  margin: 14px 0;
  white-space: pre;
  overflow-x: auto;
}
```

### Inline Code Span

For variable names and identifiers inline in prose:

```html
<span class="tech-inline">firstFlipTime</span>
```

```css
.tech-inline {
  font-family: 'SF Mono', 'Fira Code', 'Courier New', monospace;
  font-size: 11.5px;
  background: rgba(255,255,255,0.06);
  border-radius: 3px;
  padding: 1px 5px;
}
```

### Inline SVG Diagrams

Diagrams are inline `<svg>` elements, using the same CSS variables so they adapt to the theme automatically. Key conventions:

- `fill="var(--bg)"` for the background rect
- `stroke="var(--border)"` for axes and dashed guides
- `stroke="var(--text)"` for the main geometry (rods, curves)
- `fill="var(--text-muted)"` for labels
- `fill="var(--accent)"` at `opacity="0.88"` for mass nodes; add a second circle at `opacity="0.1"` for a soft glow
- `stroke-dasharray="5,4"` for reference/guide lines
- `font-family="inherit"` on all text nodes (picks up Inter)

```css
.tech-diagram {
  width: 100%;
  max-width: 480px;
  margin: 20px 0 24px;
  display: block;
}
```

### Typical Section Structure

```
Section title (uppercase, 12px, 600)
─────────────────────────────────── ← border-top
  Intro paragraph (14px, line-height 1.8, max-width 600px)
  [optional SVG diagram]
  Equation label text (bold inline)
  $$display math block$$
  More prose
  .tech-table block (if needed)
  More prose
─────────────────────────────────── ← next section border-top
```

---

## Border Radius Reference

| Element              | Radius  |
|----------------------|---------|
| Buttons, inputs, selects | `6px` |
| Modal card           | `10px`  |
| Dropdown menus       | `6px`   |
| Tooltips             | `6px`   |
| Canvas / image elements | `4px` |
| Progress bar         | `2px`   |
| Tab underline        | `1px`   |

---

## Transition Reference

All transitions are `0.15s` ease (default) unless noted:

| Property changed           | Duration |
|----------------------------|----------|
| `color`, `background`, `border-color` | `0.15s` |
| Tooltip `opacity`          | `0.15s`  |
| Progress bar `width`       | `0.08s linear` |
| Hint `opacity` (fade out)  | `0.4s`   |

---

## Checklist for Replication

- [ ] `box-sizing: border-box` on `*`
- [ ] `margin: 0; padding: 0` reset
- [ ] Inter font loaded via Google Fonts (400, 500, 600)
- [ ] CSS variables defined on `:root`
- [ ] `html, body` set to `height: 100%`
- [ ] Body: `display: flex; flex-direction: column; overflow: hidden; -webkit-font-smoothing: antialiased`
- [ ] Nav: `border-bottom: 1px solid var(--border)`, tabs use `::after` underline trick
- [ ] Control bar: `background: var(--surface)` (slightly lighter than `--bg`)
- [ ] All inputs/selects: `background: transparent`, styled border only
- [ ] Grid dividers: `gap: 1px` on grid + `background: var(--border)` on container
- [ ] No box shadows anywhere — borders only
- [ ] No gradients in UI chrome (gradients only inside canvases/visualizations)
- [ ] Modals: `background: rgba(0,0,0,0.72)` overlay + `#0d0d0d` card
- [ ] "How it works" page: KaTeX loaded via CDN, math in `$$...$$` / `$...$`
- [ ] `<details>` sections: `list-style: none`, chevron via `::after`, rotates on `[open]`
- [ ] `.tech-body .katex-display`: `overflow-x: auto` to prevent blowout on narrow screens
- [ ] SVG diagrams: use CSS variables for all colors so they inherit the dark theme
