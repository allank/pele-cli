# pele-cli

Render a [Mermaid](https://mermaid.js.org/) diagram to a PNG from one self-contained binary. It embeds the [pele.run](https://pele.run) renderer, so it needs no Node, no browser, no network and no installed fonts.

It is built for people and for LLM agents: readable output in a terminal, a JSON envelope when piped, and a machine-readable description of every command ([Murli](https://murli.allankent.com/)).

## Install

Download the archive for your platform from the [Releases](https://github.com/allank/pele-cli/releases) page (macOS arm64 and x86_64, Linux x86_64 and arm64), check it against its `.sha256` file, and put `pele-cli` on your `PATH`.

Or build from source with Rust:

```sh
cargo build --release   # produces target/release/pele-cli
```

## Usage

```sh
pele-cli render diagram.mmd -o diagram.png
cat diagram.mmd | pele-cli render - -o diagram.png
```

| Option | Default | Meaning |
|---|---|---|
| `<input>` | `-` | Diagram file, or `-` for stdin |
| `--diagram-text <text>` | none | The Diagram as a string, instead of `<input>` |
| `-o, --out <path>` | required | Output PNG path |
| `--style <file>` | none | [Style block](docs/theming.md): CSS to theme the diagram |
| `--style-text <css>` | none | The Style block as a string, instead of `--style` |
| `--color-scheme light\|dark` | `light` | Which of the Style block's light or dark values apply |
| `--scale <n>` | `2` | PNG size as a multiple of the diagram's natural size |
| `--transparent` | off | Leave the background transparent |
| `--dry-run` | off | Render and report the size, but write nothing |
| `--output json\|ndjson\|text` | auto | Force the output format |

One Diagram becomes one PNG per run.

You can also pass both as strings, which avoids temporary files in scripts and agents:

```sh
pele-cli render --diagram-text 'flowchart LR; A-->B' \
  --style-text '.pele { --pele-bg: #fffcf0 }' -o diagram.png
```

Use single quotes so the shell does not expand `$` or backticks. A real newline inside the quotes works; a literal `\n` does not, so for a one-liner separate statements with `;`. `--diagram-text` cannot be combined with `<input>`, and `--style-text` cannot be combined with `--style`.

When you type a Diagram straight into the terminal with `-` (rather than piping a file in), `pele-cli` keeps reading until it sees end-of-input. Press `Ctrl-D` on an empty line to submit it.

## Theming

A Style block is CSS using the [pele.run theming variables](https://pele.run/theming):

```css
.pele {
  --pele-bg: #fffcf0;
  --pele-surface: #f2f0e5;
  --pele-accent: #205ea6;
}
.pele-node[data-id="A"] .pele-label { fill: var(--pele-accent); }
```

```sh
pele-cli render diagram.mmd --style theme.css --color-scheme dark -o diagram.png
```

Colors set inside the diagram itself (`style`, `classDef`, `linkStyle`) win over the Style block. See [docs/theming.md](docs/theming.md) for every supported variable, selector and limitation.

## For scripts and agents

At a terminal the result is a line of text. When piped, stdout is a JSON envelope:

```json
{
  "status": "ok",
  "schema_version": "1.0",
  "tool_version": "0.1.0",
  "message": "Wrote diagram.png (754x176).",
  "result": { "path": "diagram.png", "width": 754, "height": 176 }
}
```

Failures print an error envelope on stderr, write no PNG, and exit non-zero:

| Failure | Exit code |
|---|---|
| Invalid Diagram or Style block, bad flag | `1` |
| A `--style` or input file does not exist | `5` |
| Permission denied on a file | `6` |
| Unexpected internal failure | `2` |

Each error names the offending `field` and a `suggestion`. Warnings, such as an unavailable font family, are appended to `message`.

`pele-cli describe` and `pele-cli --schema` print the commands and flags as JSON, and `pele-cli describe --agents-md` prints Markdown for an `AGENTS.md`.

## Notes

- Fonts: Inter and JetBrains Mono are embedded, and installed fonts are never used, so output is identical on every machine.
- `%%{init}%%` and Mermaid `themeVariables` are not honored by pele.run. Theme with the `--pele-*` variables instead.

## Development

```sh
cargo test --release
```

The end-to-end tests compare PNGs against golden images in `tests/golden`. The goldens are generated on macOS arm64, so run the suite there. To regenerate them after an intended change, run `UPDATE_GOLDEN=1 cargo test --release` and review the images.

Releases are built by the `Release` workflow: push a tag such as `v0.1.0` (it must match the version in `Cargo.toml`). Running the workflow by hand builds and tests everything without publishing.

The design decisions are in `docs/adr`, and the project vocabulary is in `GLOSSARY.md`.

## Licenses

pele-cli is released under the [MIT License](LICENSE).

It embeds [pele.run](https://github.com/obsidianmd/pele) (MIT), [Inter](https://github.com/rsms/inter) (SIL OFL) and [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) (SIL OFL). Their licenses are in `assets/`. Every release archive includes them in `licenses/`, plus `THIRD-PARTY-NOTICES.txt` for the Rust crates that are compiled in.
