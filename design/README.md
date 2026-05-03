# OxideBooks Design System

A modern minimalist visual language built for financial clarity. Every design decision exists to help users trust numbers, process information quickly, and act without hesitation.

---

## Philosophy

### Clarity over decoration

Financial software is read, not browsed. The design system starts by removing everything that doesn't carry information — no gradients, no shadows for decoration, no rounded corners that don't serve a purpose. What remains is typography, space, and intentional color.

### Numbers are first-class citizens

Accounting is fundamentally about numbers. The type system treats numeric data differently from prose: monospace font, tabular figures, consistent alignment. A column of amounts should align to the decimal point without explicit effort. Red and green have precise meanings here — they are not used for branding.

### High contrast is not an afterthought

The dark mode uses near-black backgrounds (`#09090B`) with near-white text (`#FAFAFA`), not dark gray on slightly-less-dark gray. This is intentional. Accountants work long hours. The eyes deserve genuine contrast. Light mode follows the same logic — primary text approaches black, not middle gray.

### Surfaces, not depth

The interface is flat. Hierarchy is expressed through spacing and typographic weight, not through shadows or z-axis layering. A "card" is a surface defined by a one-pixel border, not a shadow. This makes the system feel modern and keeps the visual weight on the content.

### Color carries meaning

Color is used sparingly and consistently. Teal is the action color — links, focus rings, primary buttons. Green means positive (credit, income, paid). Red means negative (debit, expense, overdue). These mappings never deviate. A user who sees green knows it means positive, regardless of context.

---

## Modes

**Light mode** — the default. White page background, near-white surfaces, warm near-black text. The warmth prevents the sterile feeling of pure cool grays.

**Dark mode (high contrast)** — zinc-family backgrounds (slightly warm, not blue-shifted) with true white text. Accent colors shift toward their lighter range to maintain contrast ratios ≥ 4.5:1 (WCAG AA), targeting ≥ 7:1 (WCAG AAA) for body text.

---

## Contents

| File | Purpose |
|---|---|
| [tokens.css](tokens.css) | CSS custom properties — import to use the system |
| [tokens.json](tokens.json) | Design token definitions for Figma / Style Dictionary |
| [typography.md](typography.md) | Type scale, font families, numeric formatting |
| [iconography.md](iconography.md) | Icon style, grid, usage guidelines |
| [logging.md](logging.md) | Log level semantics, structured field conventions, emoji legend |

---

## Quick Reference

### Brand Colors

| Swatch | Name | Light | Dark |
|---|---|---|---|
| Teal-700 | Primary | `#0F766E` | `#2DD4BF` |
| Teal-600 | Primary Hover | `#0D9488` | `#5EEAD4` |
| Teal-950 | Primary Muted Bg | `#CCFBF1` | `#042F2E` |

### Semantic Colors

| Meaning | Light | Dark | Usage |
|---|---|---|---|
| Positive | `#16A34A` | `#4ADE80` | Credits, income, paid, active |
| Negative | `#DC2626` | `#F87171` | Debits, expenses, overdue, error |
| Warning | `#D97706` | `#FCD34D` | Partial, pending, caution |
| Neutral | `#6366F1` | `#818CF8` | Draft, informational |

### Text Colors

| Role | Light | Dark |
|---|---|---|
| Primary | `#1C1917` | `#FAFAFA` |
| Secondary | `#57534E` | `#A1A1AA` |
| Tertiary / Placeholder | `#A8A29E` | `#71717A` |

---

## Design Principles at a Glance

1. **Remove before adding** — if an element can be removed without losing meaning, remove it.
2. **Align numbers to the decimal** — all monetary and quantity values use tabular lining figures.
3. **Red and green are reserved** — never use these colors for anything except positive/negative financial meaning.
4. **One primary action per view** — never two equally-weighted CTAs in the same section.
5. **Borders define surfaces** — use a `1px` border in `--color-border`, not a shadow.
6. **Space in multiples of 4px** — all spacing values are `4px` increments.
7. **Icons are labels, not decoration** — every icon must have a text label or `aria-label`.
