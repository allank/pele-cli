use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pele-cli"))
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
