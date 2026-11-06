use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn raw() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pele-cli"))
}

/// `pele-cli render ...`
fn bin() -> Command {
    let mut command = raw();
    command.arg("render");
    command
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(bytes)))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Compare against a golden PNG; set UPDATE_GOLDEN=1 to (re)generate it.
/// Goldens are generated and compared on macOS arm64 only.
fn assert_golden(actual: &Path, golden: &str) {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(golden);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::copy(actual, &golden).unwrap();
        return;
    }
    let expected = std::fs::read(&golden).expect("missing golden PNG; run with UPDATE_GOLDEN=1");
    assert!(std::fs::read(actual).unwrap() == expected, "{} differs from golden", golden.display());
}

#[test]
fn renders_flowchart_from_file() {
    let dir = tempdir();
    let out = dir.join("out.png");
    let status = bin().arg(fixture("flowchart.mmd")).arg("-o").arg(&out).status().unwrap();
    assert!(status.success());
    assert_golden(&out, "flowchart.png");
}

#[test]
fn renders_flowchart_from_stdin() {
    let dir = tempdir();
    let out = dir.join("out.png");
    let mut child = bin().args(["-", "-o"]).arg(&out).stdin(Stdio::piped()).spawn().unwrap();
    let text = std::fs::read(fixture("flowchart.mmd")).unwrap();
    child.stdin.take().unwrap().write_all(&text).unwrap();
    assert!(child.wait().unwrap().success());
    assert_golden(&out, "flowchart.png");
}

#[test]
fn style_block_themes_diagram_but_fixed_colors_win() {
    let dir = tempdir();
    let out = dir.join("themed.png");
    let status = bin()
        .arg(fixture("themed.mmd"))
        .arg("--style")
        .arg(fixture("themed.css"))
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    assert_golden(&out, "themed.png");
}

#[test]
fn style_rules_class_ancestor_and_data_id_selectors_apply() {
    let dir = tempdir();
    let out = dir.join("selectors.png");
    let status = bin()
        .arg(fixture("themed.mmd"))
        .arg("--style")
        .arg(fixture("selectors.css"))
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    assert_golden(&out, "selectors.png");
}

#[test]
fn rule_targeting_a_fixed_color_node_loses_to_the_diagram() {
    let dir = tempdir();
    let out = dir.join("clash.png");
    let status = bin()
        .arg(fixture("themed.mmd"))
        .arg("--style")
        .arg(fixture("clash.css"))
        .arg("-o")
        .arg(&out)
        .status()
        .unwrap();
    assert!(status.success());
    assert_golden(&out, "clash.png");
}

fn render_scheme(extra: &[&str], name: &str) -> (std::process::ExitStatus, PathBuf) {
    let out = tempdir().join(name);
    let status = bin()
        .arg(fixture("flowchart.mmd"))
        .arg("--style")
        .arg(fixture("scheme.css"))
        .args(extra)
        .arg("-o")
        .arg(&out)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    (status, out)
}

#[test]
fn color_scheme_defaults_to_light_and_ignores_dark_media_query() {
    let (status, out) = render_scheme(&[], "default.png");
    assert!(status.success());
    assert_golden(&out, "scheme-light.png");
}

#[test]
fn color_scheme_light_matches_the_default() {
    let (status, out) = render_scheme(&["--color-scheme", "light"], "light.png");
    assert!(status.success());
    assert_golden(&out, "scheme-light.png");
}

#[test]
fn color_scheme_dark_applies_the_dark_media_query() {
    let (status, out) = render_scheme(&["--color-scheme", "dark"], "dark.png");
    assert!(status.success());
    assert_golden(&out, "scheme-dark.png");
}

#[test]
fn invalid_color_scheme_fails_and_writes_no_png() {
    let (status, out) = render_scheme(&["--color-scheme", "sepia"], "sepia.png");
    assert!(!status.success());
    assert!(!out.exists());
}

fn render_flowchart(extra: &[&str], name: &str) -> (std::process::ExitStatus, PathBuf) {
    let out = tempdir().join(name);
    let status = bin()
        .arg(fixture("flowchart.mmd"))
        .args(extra)
        .arg("-o")
        .arg(&out)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    (status, out)
}

fn load(path: &Path) -> resvg::tiny_skia::Pixmap {
    resvg::tiny_skia::Pixmap::load_png(path).unwrap()
}

#[test]
fn default_scale_is_twice_the_natural_size() {
    let (_, natural) = render_flowchart(&["--scale", "1"], "natural.png");
    let (_, default) = render_flowchart(&[], "default.png");
    let (natural, default) = (load(&natural), load(&default));
    assert_eq!(default.width(), natural.width() * 2);
    assert_eq!(default.height(), natural.height() * 2);
}

#[test]
fn scale_changes_dimensions_proportionally() {
    let (_, natural) = render_flowchart(&["--scale", "1"], "natural.png");
    let (status, tripled) = render_flowchart(&["--scale", "3"], "tripled.png");
    assert!(status.success());
    let (natural, tripled) = (load(&natural), load(&tripled));
    assert_eq!(tripled.width(), natural.width() * 3);
    assert_eq!(tripled.height(), natural.height() * 3);
}

#[test]
fn background_is_opaque_by_default() {
    let (_, out) = render_flowchart(&[], "opaque.png");
    assert_eq!(load(&out).pixel(0, 0).unwrap().alpha(), 255);
}

#[test]
fn transparent_leaves_no_background_fill() {
    let (status, out) = render_flowchart(&["--transparent"], "transparent.png");
    assert!(status.success());
    let png = load(&out);
    assert_eq!(png.pixel(0, 0).unwrap().alpha(), 0);
    // The diagram itself is still drawn.
    assert!(png.pixels().iter().any(|p| p.alpha() > 0));
}

#[test]
fn invalid_scale_values_fail_and_write_no_png() {
    for bad in ["0", "-1", "abc", "NaN", "inf"] {
        let (status, out) = render_flowchart(&["--scale", bad], "bad-scale.png");
        assert!(!status.success(), "--scale {bad} should fail");
        assert!(!out.exists(), "--scale {bad} should write nothing");
    }
}

#[test]
fn missing_style_file_fails_and_writes_no_png() {
    let dir = tempdir();
    let out = dir.join("missing-style.png");
    let status = bin()
        .arg(fixture("themed.mmd"))
        .args(["--style", "/nonexistent/style.css", "-o"])
        .arg(&out)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out.exists());
}

#[test]
fn invalid_style_block_fails_and_writes_no_png() {
    let dir = tempdir();
    let bad = dir.join("bad.css");
    std::fs::write(&bad, ".pele { --pele-bg: #fff").unwrap();
    let out = dir.join("bad-style.png");
    let status = bin()
        .arg(fixture("themed.mmd"))
        .arg("--style")
        .arg(&bad)
        .arg("-o")
        .arg(&out)
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!out.exists());
}

#[test]
fn missing_output_flag_fails_and_writes_nothing() {
    let dir = tempdir();
    let status = bin().current_dir(&dir).arg(fixture("flowchart.mmd")).status().unwrap();
    assert!(!status.success());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
}

#[test]
fn invalid_diagram_fails_and_writes_no_png() {
    let dir = tempdir();
    let out = dir.join("out.png");
    let mut child = bin().args(["-", "-o"]).arg(&out).stdin(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(b"not a diagram").unwrap();
    assert!(!child.wait().unwrap().success());
    assert!(!out.exists());
}

fn tempdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pele-cli-test-{}-{:?}", std::process::id(), std::thread::current().id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Murli output contract: the success envelope goes to stdout, the error envelope to stderr.

#[test]
fn piped_success_prints_the_envelope_with_path_and_size() {
    let out = tempdir().join("env.png");
    let run = bin().arg(fixture("flowchart.mmd")).arg("-o").arg(&out).output().unwrap();
    assert!(run.status.success());
    assert!(run.stderr.is_empty());
    let envelope = json(&run.stdout);
    assert_eq!(envelope["status"], "ok");
    assert_eq!(envelope["schema_version"], "1.0");
    assert_eq!(envelope["tool_version"], env!("CARGO_PKG_VERSION"));
    assert!(envelope["message"].as_str().unwrap().starts_with("Wrote "));
    let png = load(&out);
    assert_eq!(envelope["result"]["path"], out.to_str().unwrap());
    assert_eq!(envelope["result"]["width"], png.width());
    assert_eq!(envelope["result"]["height"], png.height());
}

#[test]
fn text_output_prints_readable_status_not_json() {
    let out = tempdir().join("text.png");
    let run = bin().arg(fixture("flowchart.mmd")).args(["--output", "text", "-o"]).arg(&out).output().unwrap();
    assert!(run.status.success());
    let stdout = String::from_utf8(run.stdout).unwrap();
    assert!(stdout.starts_with("Wrote "), "{stdout}");
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

#[test]
fn murli_flags_work_after_the_subcommand_and_dry_run_writes_nothing() {
    let out = tempdir().join("dry.png");
    let run = bin().arg(fixture("flowchart.mmd")).args(["--agent", "--dry-run", "-o"]).arg(&out).output().unwrap();
    assert!(run.status.success());
    let envelope = json(&run.stdout);
    assert_eq!(envelope["status"], "plan");
    assert!(envelope["plan"]["width"].as_u64().unwrap() > 0);
    assert!(!out.exists());
}

#[test]
fn invalid_diagram_prints_a_structured_error() {
    let out = tempdir().join("bad.png");
    let mut child = bin()
        .args(["-", "-o"])
        .arg(&out)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"not a diagram").unwrap();
    let run = child.wait_with_output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    assert!(run.stdout.is_empty());
    let error = json(&run.stderr);
    assert_eq!(error["status"], "error");
    assert_eq!(error["error"], "user_error");
    assert_eq!(error["field"], "input");
    assert!(error["message"].as_str().unwrap().contains("pele.run could not render the Diagram"));
    assert!(error["suggestion"].as_str().unwrap().contains("Fix the Diagram"));
    assert!(!out.exists());
}

#[test]
fn missing_out_prints_a_structured_error() {
    let run = bin().arg(fixture("flowchart.mmd")).output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    assert!(run.stdout.is_empty());
    let error = json(&run.stderr);
    assert_eq!(error["status"], "error");
    assert_eq!(error["field"], "--out");
    assert!(error["message"].as_str().unwrap().contains("--out"));
}

#[test]
fn missing_style_file_is_a_not_found_error() {
    let out = tempdir().join("nostyle.png");
    let run = bin().arg(fixture("themed.mmd")).args(["--style", "/nonexistent/style.css", "-o"]).arg(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(5));
    let error = json(&run.stderr);
    assert_eq!(error["error"], "not_found");
    assert_eq!(error["field"], "--style");
    assert!(!out.exists());
}

#[test]
fn invalid_style_block_is_a_user_error() {
    let dir = tempdir();
    let bad = dir.join("broken.css");
    std::fs::write(&bad, ".pele { --pele-bg: #fff").unwrap();
    let out = dir.join("broken.png");
    let run = bin().arg(fixture("themed.mmd")).arg("--style").arg(&bad).arg("-o").arg(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    let error = json(&run.stderr);
    assert_eq!(error["error"], "user_error");
    assert_eq!(error["field"], "--style");
    assert!(!out.exists());
}

#[test]
fn invalid_flag_values_are_structured_errors() {
    let out = tempdir().join("flag.png");
    let run = bin().arg(fixture("flowchart.mmd")).args(["--scale", "0", "-o"]).arg(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    assert_eq!(json(&run.stderr)["status"], "error");

    let run = bin().arg(fixture("flowchart.mmd")).args(["--output", "xml", "-o"]).arg(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    let error = json(&run.stderr);
    assert_eq!(error["field"], "--output");
    assert_eq!(error["valid_values"], serde_json::json!(["json", "ndjson", "text"]));
    assert!(!out.exists());
}

#[test]
fn no_command_is_a_structured_error() {
    let run = raw().output().unwrap();
    assert_eq!(run.status.code(), Some(1));
    assert_eq!(json(&run.stderr)["field"], "command");
}

#[test]
fn describe_documents_the_render_command_and_its_flags() {
    let run = raw().arg("describe").output().unwrap();
    assert!(run.status.success());
    let described = json(&run.stdout);
    assert_eq!(described["name"], "pele-cli");
    assert_eq!(described["tool_version"], env!("CARGO_PKG_VERSION"));
    let render = &described["commands"][0];
    assert_eq!(render["name"], "render");
    let flags: Vec<&str> = render["flags"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
    for expected in ["out", "style", "color-scheme", "scale", "transparent"] {
        assert!(flags.contains(&expected), "describe is missing --{expected}: {flags:?}");
    }
    let out = render["flags"].as_array().unwrap().iter().find(|f| f["name"] == "out").unwrap();
    assert_eq!(out["required"], true);
}

#[test]
fn schema_flag_documents_the_command_tree() {
    let run = raw().arg("--schema").output().unwrap();
    assert!(run.status.success());
    let schema = json(&run.stdout);
    assert_eq!(schema["name"], "pele-cli");
    assert_eq!(schema["subcommands"][0]["name"], "render");
}
