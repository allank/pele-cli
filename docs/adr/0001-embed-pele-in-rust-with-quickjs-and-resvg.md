# Embed pele.run in Rust with QuickJS and resvg, not WASM

pele.run is plain JavaScript that outputs SVG only, with no WASM build and no PNG output. We run its bundled `pele.global.js` in an in-process `rquickjs` engine and rasterize the SVG with `resvg`, all compiled into one Rust binary. We rejected Go with `wazero` because Go's SVG rasterizers are weaker than `resvg`.

## Consequences

`resvg` ignores CSS `var()`, so the CLI resolves the Style block and every `var()` in the SVG itself before rasterizing, and paints the `--pele-bg` background rectangle that pele.run never emits.
