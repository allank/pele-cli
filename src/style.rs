use std::collections::HashMap;

use crate::css::StyleBlock;

const DEFAULT_BG: &str = "#fff";

/// Resolve every `var()` in a pele.run SVG so resvg, which ignores CSS custom
/// properties, sees plain values, and add the background pele.run never paints
/// (unless `transparent`).
pub fn resolve(svg: &str, style: &StyleBlock, transparent: bool) -> String {
    let mut vars = style.vars.clone();
    for (name, value) in root_declarations(svg) {
        vars.entry(name).or_insert(value);
    }
    let bg = vars
        .get("--pele-bg")
        .map(|v| substitute(v, &vars))
        .unwrap_or_else(|| DEFAULT_BG.to_string());
    let resolved = substitute(&inject_rules(svg, &style.rules), &vars);
    let resolved = rx_style_to_attr(&resolved);
    if transparent {
        resolved
    } else {
        add_background(&resolved, &bg)
    }
}

/// Custom property declarations (`--name:value`) in the root element's `style` attribute.
fn root_declarations(svg: &str) -> Vec<(String, String)> {
    let Some(start) = svg.find(" style=\"") else {
        return Vec::new();
    };
    let rest = &svg[start + 8..];
    let Some(end) = rest.find('"') else {
        return Vec::new();
    };
    split_declarations(&rest[..end])
        .into_iter()
        .filter(|(name, _)| name.starts_with("--"))
        .collect()
}

fn split_declarations(style: &str) -> Vec<(String, String)> {
    let (mut out, mut depth, mut cur) = (Vec::new(), 0usize, String::new());
    for ch in style.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => {
                push_declaration(&mut out, &cur);
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    push_declaration(&mut out, &cur);
    out
}

fn push_declaration(out: &mut Vec<(String, String)>, decl: &str) {
    if let Some((name, value)) = decl.split_once(':') {
        out.push((name.trim().to_string(), value.trim().to_string()));
    }
}

/// Replace `var(--name[, fallback])` until none remain (a user variable wins, then a
/// declared one, then the fallback).
fn substitute(input: &str, vars: &HashMap<String, String>) -> String {
    let mut text = input.to_string();
    let mut from = 0;
    // Each replacement is scanned again from its own start, so a variable that
    // expands to another `var()` resolves too. A reference cycle is cut off by the budget.
    let mut budget = 100_000;
    while budget > 0 {
        budget -= 1;
        let Some(offset) = text[from..].find("var(") else {
            break;
        };
        let start = from + offset;
        let Some(end) = matching_paren(&text, start + 3) else {
            break;
        };
        let inner = &text[start + 4..end];
        let (name, fallback) = match inner.split_once(',') {
            Some((n, f)) => (n.trim(), Some(f.trim())),
            None => (inner.trim(), None),
        };
        let value = vars
            .get(name)
            .map(String::as_str)
            .or(fallback)
            .unwrap_or("")
            .to_string();
        text.replace_range(start..=end, &value);
        from = start;
    }
    text
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, ch) in text[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// resvg reads corner radius from the `rx` attribute, not from a `rx` property inside
/// `style`. Move it out and keep the other declarations (fixed Diagram colors live there).
fn rx_style_to_attr(svg: &str) -> String {
    const MARKER: &str = " style=\"";
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(MARKER) {
        out.push_str(&rest[..i]);
        let after = &rest[i + MARKER.len()..];
        let Some(end) = after.find('"') else {
            rest = &rest[i..];
            break;
        };
        let declarations = split_declarations(&after[..end]);
        match declarations.iter().find(|(name, _)| name == "rx") {
            Some((_, radius)) => {
                out.push_str(&format!(" rx=\"{}\"", radius.trim_end_matches("px")));
                let kept: Vec<String> = declarations
                    .iter()
                    .filter(|(name, _)| name != "rx")
                    .map(|(name, value)| format!("{name}:{value}"))
                    .collect();
                if !kept.is_empty() {
                    out.push_str(&format!(" style=\"{}\"", kept.join(";")));
                }
            }
            None => out.push_str(&rest[i..i + MARKER.len() + end + 1]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Add the Style block's rules as a `<style>` element. resvg applies them under the
/// elements' inline `style`, so fixed Diagram colors still win.
fn inject_rules(svg: &str, rules: &str) -> String {
    let Some(tag_end) = svg.find('>').filter(|_| !rules.is_empty()) else {
        return svg.to_string();
    };
    format!(
        "{}<style><![CDATA[{}]]></style>{}",
        &svg[..=tag_end],
        rules,
        &svg[tag_end + 1..]
    )
}

fn add_background(svg: &str, bg: &str) -> String {
    let Some(tag_end) = svg.find('>') else {
        return svg.to_string();
    };
    format!(
        "{}<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>{}",
        &svg[..=tag_end],
        bg,
        &svg[tag_end + 1..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_nested_vars_with_fallbacks() {
        let svg = r##"<svg style="--_s:var(--pele-surface,#f3f3f3)"><rect fill="var(--_s)"/></svg>"##;
        let out = resolve(svg, &StyleBlock::default(), false);
        assert!(!out.contains("var("));
        assert!(out.contains(r##"fill="#f3f3f3""##));
    }

    #[test]
    fn adds_default_white_background() {
        let out = resolve(r#"<svg width="1"><g/></svg>"#, &StyleBlock::default(), false);
        assert!(out.contains(r##"fill="#fff""##));
    }

    #[test]
    fn user_variables_override_defaults_and_resolve_nested() {
        let svg = r##"<svg style="--_s:var(--pele-surface,#f3f3f3)"><rect fill="var(--_s)"/></svg>"##;
        let vars = HashMap::from([
            ("--pele-surface".to_string(), "var(--brand, red)".to_string()),
            ("--brand".to_string(), "#123456".to_string()),
        ]);
        let out = resolve(svg, &StyleBlock { vars, ..Default::default() }, false);
        assert!(!out.contains("var("));
        assert!(out.contains(r##"fill="#123456""##));
    }

    #[test]
    fn rx_conversion_keeps_fixed_colors() {
        let svg = r##"<svg style="--_r:4px"><rect style="rx:var(--_r);fill:#d0a215;stroke:#ad8301;"/></svg>"##;
        let out = resolve(svg, &StyleBlock::default(), false);
        assert!(out.contains(r#"rx="4""#));
        assert!(out.contains("fill:#d0a215"));
        assert!(out.contains("stroke:#ad8301"));
    }

    #[test]
    fn rules_become_a_style_element_with_vars_resolved() {
        let style = StyleBlock {
            vars: HashMap::from([("--pele-accent".to_string(), "#205ea6".to_string())]),
            rules: ".pele-label{fill:var(--pele-accent)}\n".to_string(),
            ..Default::default()
        };
        let out = resolve(r#"<svg><g/></svg>"#, &style, false);
        assert!(out.contains(".pele-label{fill:#205ea6}"));
        assert!(!out.contains("var("));
    }

    #[test]
    fn var_with_no_value_and_no_fallback_becomes_empty() {
        let out = resolve(r#"<svg><rect fill="var(--nothing)"/></svg>"#, &StyleBlock::default(), false);
        assert!(out.contains(r#"fill="""#));
    }

    #[test]
    fn user_background_is_painted() {
        let vars = HashMap::from([("--pele-bg".to_string(), "#fffcf0".to_string())]);
        assert!(resolve(r#"<svg><g/></svg>"#, &StyleBlock { vars, ..Default::default() }, false).contains(r##"fill="#fffcf0""##));
    }

    #[test]
    fn diagram_fixed_colors_are_untouched() {
        let svg = r##"<svg style="--_s:var(--pele-surface,#f3f3f3)"><rect fill="#879a39"/></svg>"##;
        let vars = HashMap::from([("--pele-surface".to_string(), "#000000".to_string())]);
        let out = resolve(svg, &StyleBlock { vars, ..Default::default() }, false);
        assert!(out.contains(r##"fill="#879a39""##));
    }

    #[test]
    fn transparent_omits_the_background() {
        let vars = HashMap::from([("--pele-bg".to_string(), "#fffcf0".to_string())]);
        let out = resolve(r#"<svg><g/></svg>"#, &StyleBlock { vars, ..Default::default() }, true);
        assert!(!out.contains("<rect"));
    }

    #[test]
    fn rx_style_becomes_attribute() {
        let svg = r#"<svg style="--_r:4px"><rect style="rx:var(--_r)"/></svg>"#;
        assert!(resolve(svg, &StyleBlock::default(), false).contains(r#"rx="4""#));
    }
}
