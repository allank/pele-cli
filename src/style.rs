use std::collections::HashMap;

const DEFAULT_BG: &str = "#fff";

/// Resolve every `var()` in a pele.run SVG so resvg, which ignores CSS custom
/// properties, sees plain values, and add the background pele.run never paints.
pub fn resolve(svg: &str, user_vars: &HashMap<String, String>) -> String {
    let mut vars = user_vars.clone();
    for (name, value) in root_declarations(svg) {
        vars.entry(name).or_insert(value);
    }
    let bg = vars
        .get("--pele-bg")
        .map(|v| substitute(v, &vars))
        .unwrap_or_else(|| DEFAULT_BG.to_string());
    let resolved = substitute(svg, &vars);
    let resolved = rx_style_to_attr(&resolved);
    add_background(&resolved, &bg)
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

/// resvg reads corner radius from the `rx` attribute, not from a `style="rx:..."` property.
fn rx_style_to_attr(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find(" style=\"rx:") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 11..];
        match after.split_once('"') {
            Some((value, tail)) => {
                out.push_str(&format!(" rx=\"{}\"", value.trim_end_matches("px")));
                rest = tail;
            }
            None => {
                rest = &rest[i..];
                break;
            }
        }
    }
    out.push_str(rest);
    out
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
        let out = resolve(svg, &HashMap::new());
        assert!(!out.contains("var("));
        assert!(out.contains(r##"fill="#f3f3f3""##));
    }

    #[test]
    fn adds_default_white_background() {
        let out = resolve(r#"<svg width="1"><g/></svg>"#, &HashMap::new());
        assert!(out.contains(r##"fill="#fff""##));
    }

    #[test]
    fn rx_style_becomes_attribute() {
        let svg = r#"<svg style="--_r:4px"><rect style="rx:var(--_r)"/></svg>"#;
        assert!(resolve(svg, &HashMap::new()).contains(r#"rx="4""#));
    }
}
