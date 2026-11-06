# pele-cli

A command-line tool that turns Mermaid diagram text into a PNG image, using the pele.run renderer.

## Language

**Diagram**:
The Mermaid source text describing one chart, given to the tool as input.
_Avoid_: Mermaid code, code block, graph

**Style block**:
An optional, separate block of CSS that restyles a rendered Diagram.
_Avoid_: Theme, stylesheet, skin

**pele.run**:
The third-party library that renders a Diagram to SVG.
_Avoid_: Pele, the renderer, the library

**Color scheme**:
Which of a Style block's light or dark values apply to a render; `light` unless chosen otherwise.
_Avoid_: Theme, mode

**Render**:
Turn a Diagram, plus an optional Style block, into a PNG file.
_Avoid_: Generate, convert, export
