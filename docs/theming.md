# Theming a Diagram with a Style block

`pele-cli render diagram.mmd --style theme.css -o out.png` applies a **Style block**: CSS that follows the [pele.run theming reference](https://pele.run/theming). This page lists what the CLI honors and where it differs from a browser.

## How it works

1. Custom properties (`--pele-*`) declared on `.pele` or `:root` set the theme.
2. Every other rule is applied to the rendered diagram.
3. `var()` references, including nested ones and fallbacks, are resolved by the CLI before the PNG is drawn.

```css
.pele {
  --pele-bg: #fffcf0;
  --pele-surface: #f2f0e5;
  --pele-accent: #205ea6;
}
.pele-node[data-id="A"] .pele-label { fill: var(--pele-accent); }
```

## Variables

Declare these on `.pele` or `:root`. Custom properties declared in any other rule are ignored.

| Variable | Default | Meaning |
|---|---|---|
| `--pele-bg` | `#ffffff` | Background. The CLI paints it as the PNG background; `--transparent` leaves it out. |
| `--pele-fg` | `#1f1f1f` | Text |
| `--pele-fg-on-light` | `#1f1f1f` | Text on light fills set in the Diagram |
| `--pele-fg-on-dark` | `#ffffff` | Text on dark fills set in the Diagram |
| `--pele-muted` | `#6e6e6e` | Secondary text, such as ER attribute types |
| `--pele-line` | `#1f1f1f` | Edges and arrowheads |
| `--pele-surface` | `#f3f3f3` | Node fill |
| `--pele-surface-alt` | `#e6e6e6` | Subgraph fill |
| `--pele-border` | `#8a8a8a` | Node and subgraph borders |
| `--pele-accent` | `#5b7bd5` | Highlights |
| `--pele-critical` | `#e45756` | Critical Gantt tasks and high kanban priorities |
| `--pele-radius` | `4px` | Corner radius of rounded shapes |
| `--pele-title-weight`, `--pele-heading-weight`, `--pele-bold-weight` | `bold` | Title, heading and bold text weights |
| `--pele-series-1` to `--pele-series-8` | see pele.run | Colors for pie slices, git branches and other categories |
| `--pele-font`, `--pele-font-mono` | Inter, JetBrains Mono | Label fonts. See [Fonts](#fonts). |

## Selectors

Classes: `.pele`, `.pele-flowchart`, `.pele-title`, `.pele-node`, `.pele-edge`, `.pele-edge-label`, `.pele-cluster`, `.pele-cluster-label`, `.pele-label`, `.pele-marker`, `.pele-icon`, `.pele-math`, `.pele-member`, `.pele-er-type`, `.pele-er-name`, `.pele-er-keys`, `.pele-tree-file`, `.pele-tree-dir`.

Node `data-id` carries the source id: `.pele-node[data-id="start"]`.

Checked to work: class, type plus class (`g.pele-node`), universal (`*`), exact attribute match (`[data-id="A"]`), descendant (`a b`), child (`a > b`), `:first-child`, selector lists (`a, b`) and `!important`.

Checked to have **no effect**: attribute prefix, suffix and substring matches (`^=`, `$=`, `*=`) and `:hover`. The CLI does not warn about a selector that matches nothing.

## Color scheme

`--color-scheme light|dark` (default `light`) chooses which values apply. Put dark values in a media query:

```css
.pele { --pele-bg: #fffcf0; --pele-fg: #100f0f; }

@media (prefers-color-scheme: dark) {
  .pele { --pele-bg: #100f0f; --pele-fg: #cecdc3; }
}
```

Only `@media (prefers-color-scheme: light)` and `(... : dark)` are understood. Under the chosen scheme, values in the matching query override the base values. Any other `@` rule is ignored.

## Fonts

Two fonts are embedded: Inter (text) and JetBrains Mono (class members, ER attributes and tree files). Fonts installed on the machine are never used, so output is the same everywhere.

`--pele-font`, `--pele-font-mono` and `font-family` may name `Inter`, `JetBrains Mono`, `serif`, `sans-serif` or `monospace`. Any other family is dropped, the embedded font is used instead, and a warning is added to the status message.

## Precedence

Colors fixed in the Diagram itself (`style`, `classDef`, `linkStyle`) win over the Style block. Use the `--pele-*` variables to theme everything else. A rule that targets a node with a fixed color has no effect on that color.

## Known limitations

- `%%{init: ...}%%` and Mermaid `themeVariables` are ignored by pele.run, so use `--pele-*` variables instead.
- A `var()` with no declared value and no fallback becomes empty.
- Text width is estimated by pele.run rather than measured, so labels may look slightly loose or tight.
