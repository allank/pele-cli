use anyhow::{Result, anyhow};
use rquickjs::{CatchResultExt, Context, Runtime};

/// pele.run rejected the Diagram (syntax error, unknown type, over its limits).
#[derive(Debug)]
pub struct DiagramError(pub String);

impl std::fmt::Display for DiagramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DiagramError {}

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
            .map_err(|e| {
                let text = e.to_string();
                // Drop the JS stack trace; the first part is pele.run's own message.
                let message = text.split("\n    at ").next().unwrap_or(&text).trim();
                anyhow::Error::new(DiagramError(message.trim_start_matches("Error: ").to_string()))
            })
    })
}
