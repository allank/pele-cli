use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use resvg::{tiny_skia, usvg};

const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

/// The requested `--scale` cannot produce an image.
#[derive(Debug)]
pub struct ScaleError(String);

impl std::fmt::Display for ScaleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ScaleError {}

pub struct Png {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Rasterize a fully resolved SVG to a PNG at `scale` times its natural size,
/// using only the embedded fonts.
pub fn svg_to_png(svg: &str, scale: f32) -> Result<Png> {
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
    let size = tree
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or_else(|| ScaleError(format!("--scale {scale} gives an image with no pixels")))?;
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| ScaleError(format!("--scale {scale} gives an image too large to render")))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let bytes = pixmap.encode_png().map_err(|e| anyhow!("failed to encode PNG: {e}"))?;
    Ok(Png { bytes, width: size.width(), height: size.height() })
}
