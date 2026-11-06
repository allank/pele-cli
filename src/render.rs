use anyhow::{Result, anyhow};
use rquickjs::{CatchResultExt, Context, Runtime};

const PELE_JS: &str = include_str!("../assets/pele.global.js");

/// Render Diagram text to SVG with the embedded pele.run in an in-process QuickJS engine.
pub fn diagram_to_svg(diagram: &str) -> Result<String> {
    let runtime = Runtime::new()?;
    let context = Context::full(&runtime)?;
    context.with(|ctx| {
        ctx.eval::<(), _>(PELE_JS)
            .catch(&ctx)
            .map_err(|e| anyhow!("failed to load pele.run: {e}"))?;
        ctx.globals().set("__diagram", diagram)?;
        ctx.eval::<String, _>("pele.render(__diagram).svg")
            .catch(&ctx)
            .map_err(|e| anyhow!("{e}"))
    })
}
