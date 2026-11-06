mod css;
mod raster;
mod render;
mod style;

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

/// Render a Mermaid Diagram to a PNG.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Diagram file, or `-` to read from stdin
    #[arg(default_value = "-")]
    input: String,

    /// Output PNG path
    #[arg(short, long)]
    output: PathBuf,

    /// Style block: CSS following the pele.run theming reference
    #[arg(long)]
    style: Option<PathBuf>,
}

fn main() {
    if let Err(err) = run(Cli::parse()) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let diagram = read_diagram(&cli.input)?;
    let vars = match &cli.style {
        Some(path) => {
            let css = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read Style block {}", path.display()))?;
            css::parse_variables(&css)?
        }
        None => HashMap::new(),
    };
    let svg = render::diagram_to_svg(&diagram)?;
    let svg = style::resolve(&svg, &vars);
    let png = raster::svg_to_png(&svg)?;
    std::fs::write(&cli.output, png)
        .with_context(|| format!("failed to write {}", cli.output.display()))?;
    println!("{}", cli.output.display());
    Ok(())
}

fn read_diagram(input: &str) -> Result<String> {
    if input == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("failed to read Diagram from stdin")?;
        Ok(text)
    } else {
        std::fs::read_to_string(input).with_context(|| format!("failed to read {input}"))
    }
}
