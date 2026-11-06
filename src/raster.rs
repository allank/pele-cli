use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use resvg::{tiny_skia, usvg};

const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

/// Rasterize a fully resolved SVG to PNG bytes using only the embedded fonts.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_font_data(INTER_REGULAR.to_vec());
    fontdb.load_font_data(INTER_BOLD.to_vec());
    fontdb.set_sans_serif_family("Inter");
    fontdb.set_serif_family("Inter");
    fontdb.set_monospace_family("Inter");

    let options = usvg::Options {
        font_family: "Inter".to_string(),
        fontdb: Arc::new(fontdb),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).context("failed to parse rendered SVG")?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow!("diagram has zero size"))?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| anyhow!("failed to encode PNG: {e}"))
}
