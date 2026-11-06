mod css;
mod raster;
mod render;
mod style;

use std::io::Read;
use std::path::{Path, PathBuf};

use clap::error::{ContextKind, ErrorKind};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use murli::clap::{GlobalArgs, emit_agents_md, emit_describe, handle_builtins};
use murli::{AgentError, OutputFormat, VALID_OUTPUT_FORMATS, Writer};
use serde_json::json;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render a Mermaid Diagram to a PNG.
#[derive(Parser)]
#[command(name = "pele-cli", version)]
struct Cli {
    #[command(flatten)]
    murli: GlobalArgs,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a Diagram to a PNG file
    Render(RenderArgs),

    /// Describe this CLI's commands and flags as JSON, for agents
    Describe {
        /// Print Markdown for an AGENTS.md instead of JSON
        #[arg(long)]
        agents_md: bool,
    },
}

#[derive(clap::Args)]
struct RenderArgs {
    /// Diagram file, or `-` to read from stdin
    #[arg(default_value = "-")]
    input: String,

    /// The Diagram as text, instead of a file or stdin
    #[arg(long, conflicts_with = "input")]
    diagram_text: Option<String>,

    /// Output PNG path
    #[arg(short, long = "out", required = true)]
    out: PathBuf,

    /// Style block: CSS following the pele.run theming reference
    #[arg(long)]
    style: Option<PathBuf>,

    /// The Style block as text, instead of a --style file
    #[arg(long, conflicts_with = "style")]
    style_text: Option<String>,

    /// Which of the Style block's light or dark values apply
    #[arg(long, value_enum, default_value_t)]
    color_scheme: css::ColorScheme,

    /// Pixel density: the PNG is this many times the diagram's natural size
    #[arg(long, default_value_t = 2.0, value_parser = parse_scale)]
    scale: f32,

    /// Leave the background transparent instead of painting --pele-bg
    #[arg(long)]
    transparent: bool,
}

fn parse_scale(text: &str) -> Result<f32, String> {
    let scale: f32 = text.parse().map_err(|_| format!("`{text}` is not a number"))?;
    if scale.is_finite() && scale > 0.0 {
        Ok(scale)
    } else {
        Err(format!("`{text}` must be a number greater than zero"))
    }
}

/// The command tree with Murli's flags made global, so `--agent`, `--dry-run` and the
/// rest work after the subcommand as well as before it.
fn command() -> clap::Command {
    Cli::command().mut_args(|arg| match arg.get_id().as_str() {
        "agent" | "schema" | "force" | "dry_run" | "output" | "profile" => arg.global(true),
        _ => arg,
    })
}

fn main() {
    let cli = match command().try_get_matches().and_then(|m| Cli::from_arg_matches(&m)) {
        Ok(cli) => cli,
        Err(err) => fail_parse(err),
    };
    handle_builtins(&cli.murli, &command(), None);

    let mut writer = match build_writer(&cli.murli) {
        Ok(writer) => writer,
        Err(err) => new_writer(false, None).write_error(err),
    };

    match cli.command {
        Some(Commands::Render(args)) => match render_command(&args, &mut writer, cli.murli.dry_run) {
            Ok(()) => writer.flush(),
            Err(err) => writer.write_error(err),
        },
        Some(Commands::Describe { agents_md }) => {
            if agents_md {
                emit_agents_md(&command(), VERSION);
            } else {
                emit_describe(&command(), VERSION);
            }
        }
        None => writer.write_error(AgentError {
            field: "command".into(),
            ..AgentError::user_error(
                "no command given",
                "Run `pele-cli render <diagram> -o <out.png>`, or `pele-cli describe` to see all commands.",
            )
        }),
    }
}

fn new_writer(agent: bool, format: Option<OutputFormat>) -> Writer {
    Writer::new(agent, format, false, false, VERSION)
}

fn build_writer(args: &GlobalArgs) -> Result<Writer, AgentError> {
    let format = match args.output.as_deref() {
        None => None,
        Some(name) => Some(OutputFormat::from_str(name).ok_or_else(|| AgentError {
            field: "--output".into(),
            valid_values: VALID_OUTPUT_FORMATS.iter().map(|v| v.to_string()).collect(),
            ..AgentError::user_error(
                &format!("`{name}` is not a valid output format"),
                "Use one of: json, ndjson, text.",
            )
        })?),
    };
    Ok(Writer::new(args.agent, format, args.force, args.dry_run, VERSION))
}

/// Turn a clap usage error into a Murli error. Help and version output pass through.
fn fail_parse(err: clap::Error) -> ! {
    if !err.use_stderr() {
        err.exit();
    }
    let field = err
        .get(ContextKind::InvalidArg)
        .map(|arg| arg.to_string().split(" <").next().unwrap_or_default().to_string())
        .unwrap_or_default();
    // The first paragraph of clap's message, on one line: it names the offending argument.
    let rendered = err.render().to_string();
    let message = rendered
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    let message = message.trim_start_matches("error: ");
    let suggestion = match err.kind() {
        ErrorKind::MissingRequiredArgument => "Add the missing argument; run `pele-cli render --help` for usage.",
        _ => "Run `pele-cli --help` for usage.",
    };
    new_writer(false, None).write_error(AgentError {
        field,
        ..AgentError::user_error(message, suggestion)
    })
}

fn render_command(args: &RenderArgs, writer: &mut Writer, dry_run: bool) -> Result<(), AgentError> {
    let diagram = match &args.diagram_text {
        Some(text) => text.clone(),
        None => read_diagram(&args.input)?,
    };
    let (style_source, style_field) = match (&args.style, &args.style_text) {
        (Some(path), _) => {
            let text = std::fs::read_to_string(path).map_err(|e| io_error("Style block", "--style", path, &e))?;
            (Some(text), "--style")
        }
        (None, Some(text)) => (Some(text.clone()), "--style-text"),
        (None, None) => (None, "--style"),
    };
    let style_block = match style_source {
        Some(text) => css::parse(&text, args.color_scheme).map_err(|e| AgentError {
            field: style_field.into(),
            ..AgentError::user_error(&format!("{e:#}"), "Fix the Style block and run again.")
        })?,
        None => css::StyleBlock::default(),
    };

    let svg = render::diagram_to_svg(&diagram).map_err(|e| match e.downcast_ref::<render::DiagramError>() {
        Some(diagram_error) => AgentError {
            field: "input".into(),
            ..AgentError::user_error(
                &format!("pele.run could not render the Diagram: {diagram_error}"),
                "Fix the Diagram syntax; running the same Diagram again will fail the same way.",
            )
        },
        None => AgentError::tool_error(&format!("{e:#}")),
    })?;
    let svg = style::resolve(&svg, &style_block, args.transparent);
    let png = raster::svg_to_png(&svg, args.scale).map_err(|e| match e.downcast_ref::<raster::ScaleError>() {
        Some(scale_error) => AgentError {
            field: "--scale".into(),
            ..AgentError::user_error(&scale_error.to_string(), "Use a smaller --scale.")
        },
        None => AgentError::tool_error(&format!("{e:#}")),
    })?;

    let result = json!({ "path": args.out, "width": png.width, "height": png.height });
    if dry_run {
        let text = format!("Would write {}x{} PNG to {}.", png.width, png.height, args.out.display());
        writer.write_plan(&with_warnings(text, &style_block.warnings), &result);
        return Ok(());
    }
    std::fs::write(&args.out, &png.bytes).map_err(|e| io_error("output file", "--out", &args.out, &e))?;
    let text = format!("Wrote {} ({}x{}).", args.out.display(), png.width, png.height);
    writer.write_success(&with_warnings(text, &style_block.warnings), &result);
    Ok(())
}

/// Murli envelopes have no warnings field, so warnings travel in the human message.
fn with_warnings(text: String, warnings: &[String]) -> String {
    warnings.iter().fold(text, |text, warning| format!("{text} Warning: {warning}."))
}

fn read_diagram(input: &str) -> Result<String, AgentError> {
    if input == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|e| AgentError::tool_error(&format!("failed to read the Diagram from stdin: {e}")))?;
        Ok(text)
    } else {
        std::fs::read_to_string(input).map_err(|e| io_error("Diagram", "input", Path::new(input), &e))
    }
}

/// A file error becomes `not_found`, `permission` or `tool_error` by its kind.
fn io_error(what: &str, field: &str, path: &Path, err: &std::io::Error) -> AgentError {
    let message = format!("failed to access the {what} {}: {err}", path.display());
    let error = match err.kind() {
        std::io::ErrorKind::NotFound => AgentError::not_found(&message, "Check that the path exists."),
        std::io::ErrorKind::PermissionDenied => AgentError::permission(&message, "Check the file's permissions."),
        _ => AgentError::tool_error(&message),
    };
    AgentError { field: field.into(), ..error }
}
