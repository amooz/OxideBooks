# Iconography

## Style

OxideBooks uses **outline icons** from the [Heroicons](https://heroicons.com) library (MIT licensed). The outline style (as opposed to solid/filled) is consistent with the minimalist philosophy — it adds meaning without visual weight.

All icons are SVG, rendered inline or as an icon font. Do not use bitmap icon formats.

---

## Grid and Geometry

Icons are designed on a **24×24px grid** with:

- **Stroke weight:** 1.5px (scales to 1px at 16px size, 2px at 32px)
- **Corner treatment:** Rounded joins and rounded end caps (`stroke-linejoin: round`, `stroke-linecap: round`)
- **Padding:** 2px inset from the 24px bounding box (20px effective drawing area)
- **Fill:** `none` (outline style only)
- **Stroke color:** Inherits from `currentColor` — never hardcode hex values in SVG

```svg
<svg
  xmlns="http://www.w3.org/2000/svg"
  width="24" height="24"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <!-- paths -->
</svg>
```

---

## Sizing

| Context | Size | CSS class |
|---|---|---|
| Inline text icon | 16px | `.icon-sm` |
| Default (most UI) | 20px | `.icon-base` |
| Toolbar / nav | 24px | `.icon-lg` |
| Empty states | 48px | `.icon-xl` |
| Illustrations | 64–96px | `.icon-2xl` |

```css
.icon-sm   { width: 1rem;    height: 1rem;    }   /* 16px */
.icon-base { width: 1.25rem; height: 1.25rem; }   /* 20px */
.icon-lg   { width: 1.5rem;  height: 1.5rem;  }   /* 24px */
.icon-xl   { width: 3rem;    height: 3rem;    }   /* 48px */
.icon-2xl  { width: 4rem;    height: 4rem;    }   /* 64px */
```

Icons inherit `color: currentColor` — set the parent's text color to change the icon.

---

## Accessibility

Every icon must communicate meaning accessibly:

**Decorative icon (has an adjacent text label):**
```html
<svg aria-hidden="true" focusable="false">...</svg>
<span>Create Invoice</span>
```

**Standalone icon (no text):**
```html
<button aria-label="Delete account">
  <svg aria-hidden="true" focusable="false">...</svg>
</button>
```

Never use `title` elements inside SVG for accessibility — use `aria-label` on the parent interactive element.

---

## Icon Catalog

### Navigation

| Icon | Heroicon name | Usage |
|---|---|---|
| Dashboard / Home | `home` | Main dashboard link |
| Accounts | `rectangle-stack` | Chart of accounts |
| Transactions | `arrows-right-left` | Journal entries |
| Invoices | `document-text` | Invoices & bills |
| Contacts | `user-group` | Customer/vendor list |
| Reports | `chart-bar` | Financial reports |
| Settings | `cog-6-tooth` | Organization settings |
| Roles | `shield-check` | RBAC management |
| Identity | `key` | SSO / identity providers |

### Actions

| Icon | Heroicon name | Usage |
|---|---|---|
| Create / Add | `plus` | Primary create action |
| Edit | `pencil` | Edit / update |
| Delete | `trash` | Destructive delete |
| Save | `check` | Confirm / save |
| Cancel | `x-mark` | Cancel / dismiss |
| Filter | `funnel` | Filter list |
| Search | `magnifying-glass` | Search input |
| Export | `arrow-down-tray` | Download / export |
| Copy | `clipboard-document` | Copy to clipboard |
| Refresh | `arrow-path` | Reload / refresh |
| Expand | `chevron-down` | Expand section |
| Collapse | `chevron-up` | Collapse section |
| Navigate right | `chevron-right` | Breadcrumb, list item |
| Sort ascending | `bars-arrow-up` | Table sort |
| Sort descending | `bars-arrow-down` | Table sort |
| More options | `ellipsis-horizontal` | Row action menu |

### Status and Feedback

| Icon | Heroicon name | Semantic color | Usage |
|---|---|---|---|
| Success | `check-circle` | `--color-positive` | Success state, paid |
| Error | `x-circle` | `--color-negative` | Error state, overdue |
| Warning | `exclamation-triangle` | `--color-warning` | Warning, partial |
| Info | `information-circle` | `--color-primary` | Informational |
| Draft | `pencil-square` | `--color-neutral-status` | Draft status |
| Voided | `no-symbol` | `--color-text-3` | Voided status |
| Loading | `arrow-path` | `--color-text-2` | Spinner (animated) |

### Finance-Specific

| Icon | Heroicon name | Usage |
|---|---|---|
| Debit | `arrow-up-right` | Debit entry, outflow |
| Credit | `arrow-down-left` | Credit entry, inflow |
| Bank / Account | `building-library` | Bank accounts |
| Invoice | `document-text` | Receivable invoice |
| Bill | `inbox` | Payable bill |
| Payment | `banknotes` | Payment received/made |
| Organization | `building-office-2` | Organization / tenant |
| User | `user-circle` | User account |
| Lock / Auth | `lock-closed` | Authentication, security |
| SSO | `arrow-top-right-on-square` | External auth redirect |
| API token | `code-bracket` | SCIM / API tokens |

---

## Color Usage

Icons use `currentColor` and pick up the text color of their context:

```css
/* Default: secondary text color */
.icon { color: var(--color-text-2); }

/* Primary action */
.btn-primary .icon { color: var(--color-text-inverse); }

/* Positive state */
.status-paid .icon { color: var(--color-positive); }

/* Negative state */
.status-overdue .icon { color: var(--color-negative); }

/* Destructive action */
.btn-danger .icon { color: var(--color-negative); }
```

**Never** tint an icon teal/red/green for decorative purposes. Color on icons always carries the same meaning as color on text.

---

## Do and Don't

**Do:**
- Use a consistent 1.5px stroke weight
- Keep icons at standard sizes (16, 20, 24, 48px)
- Always pair standalone icons with an `aria-label`
- Use `currentColor` — never inline fill/stroke hex values

**Don't:**
- Mix solid and outline icon styles in the same UI
- Use icons without labels in primary navigation
- Scale icons to non-standard sizes (e.g., 18px, 22px)
- Use icons to replace text in dense data tables (numbers and codes are sufficient)
- Use red or green icons outside of positive/negative financial meaning

---

## Empty States

Empty state illustrations use a single large icon (48–64px) centered in the empty area, colored with `--color-text-3`, accompanied by a short explanatory sentence and a primary CTA:

```
    [icon: document-text, 48px, --color-text-3]

    No invoices yet
    Create your first invoice to start tracking receivables.

    [+ Create Invoice]
```

No decorative illustrations, gradients, or mascots. The empty state should feel like a natural part of the interface, not a separate design system.
