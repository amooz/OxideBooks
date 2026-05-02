# Typography

## Font Families

### Sans-serif — Interface Text

**Inter** (primary), falling back to the system sans-serif stack.

Inter is designed specifically for screen readability, has wide language coverage, and includes tabular figure variants critical for financial data. It performs well at small sizes in both light and dark contexts.

```css
font-family: var(--font-sans);
```

### Monospace — Numbers and Code

**JetBrains Mono**, falling back to Fira Code, then system monospace.

All monetary amounts, account codes, invoice numbers, dates in tables, and API tokens use the monospace family. This ensures columns align vertically without manual tabbing and makes errors in numerical sequences immediately visible.

```css
font-family: var(--font-mono);
```

**Critical:** Every `<td>` containing a monetary or quantity value must use `--font-mono` with `font-variant-numeric: tabular-nums` so digit widths are uniform.

---

## Type Scale

| Token | Size | Usage |
|---|---|---|
| `--text-xs` | 12px | Labels on badges, helper text, table sub-details |
| `--text-sm` | 14px | Table body, form inputs, secondary text, nav items |
| `--text-base` | 16px | Default body text, descriptions |
| `--text-lg` | 18px | Section subheadings, emphasis |
| `--text-xl` | 20px | Panel headings |
| `--text-2xl` | 24px | Page section titles |
| `--text-3xl` | 30px | Page headings |
| `--text-4xl` | 36px | Dashboard key metrics (large KPI numbers) |

Most UI text lives at `--text-sm` (14px) — this is a dense data application, not a marketing page. Readable at 14px means users see more information per screen.

---

## Numeric Formatting

### Monetary Amounts

```css
.amount {
  font-family: var(--font-mono);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.amount--positive { color: var(--color-positive); }
.amount--negative { color: var(--color-negative); }
```

- Always right-aligned in tables
- Decimal points must align vertically across rows
- Currency symbol left-aligned, amount right-aligned (e.g. `$   1,234.56`)
- Negative amounts use a minus sign (−), not parentheses, and use `--color-negative`
- Zero is displayed as `0.00`, not blank or `–`

### Percentages (tax rates)

Same monospace treatment. Display as `10.00%`, right-aligned.

### Account Codes

```css
.account-code {
  font-family: var(--font-mono);
  font-size: var(--text-sm);
  color: var(--color-text-2);
  letter-spacing: var(--tracking-wide);
}
```

---

## Text Hierarchy

```
Page title     --text-3xl  --weight-semibold  --color-text-1  --tracking-tight
Section title  --text-xl   --weight-semibold  --color-text-1
Card title     --text-base --weight-semibold  --color-text-1
Body           --text-sm   --weight-normal    --color-text-1
Secondary      --text-sm   --weight-normal    --color-text-2
Caption        --text-xs   --weight-normal    --color-text-2
Placeholder    --text-sm   --weight-normal    --color-text-3
```

### Labels (form field labels, table column headers)

```css
.label {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-2);
  letter-spacing: var(--tracking-wider);
  text-transform: uppercase;
}
```

Table column headers and form labels use uppercase with wider tracking to distinguish them from data.

---

## Line Length

For prose (descriptions, notes fields): maximum 65–75 characters (`max-width: 65ch`).

For tables and data displays: no line-length constraint; the table container controls width.

---

## Numeric Display Rules

| Value | Display | Notes |
|---|---|---|
| `10050` USD | `$100.50` | Always 2 decimal places |
| `0` USD | `$0.00` | Never blank |
| `-10050` USD | `−$100.50` | Minus sign + red color |
| `150` quantity | `1.50` | ×100 encoding, display as decimal |
| `1000` tax rate | `10.00%` | ×100 encoding |
| `2024-01-15` date | `Jan 15, 2024` | In sentences; `2024-01-15` in dense tables |

---

## Loading States

Skeleton loaders for numbers use a fixed-width rectangle matching the expected digit count, preventing layout shift:

```css
.skeleton-amount {
  width: 7ch; /* e.g. "1,234.56" */
  height: 1em;
  background: var(--color-surface-3);
  border-radius: var(--radius-sm);
  animation: pulse 1.5s ease-in-out infinite;
}
```
