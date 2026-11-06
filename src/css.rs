use std::collections::HashMap;

use anyhow::{Result, bail};

/// Custom properties (`--name: value`) declared in a Style block's `.pele` or `:root` rules.
/// Other rules are ignored for now.
pub fn parse_variables(css: &str) -> Result<HashMap<String, String>> {
    let css = strip_comments(css);
    let mut vars = HashMap::new();
    let mut rest = css.trim();
    while !rest.is_empty() {
        let Some(open) = rest.find('{') else {
            bail!("invalid Style block: expected `{{` after `{}`", rest.lines().next().unwrap_or(""));
        };
        let selector = rest[..open].trim();
        if selector.is_empty() {
            bail!("invalid Style block: rule with no selector");
        }
        let Some(close) = matching_brace(rest, open) else {
            bail!("invalid Style block: missing `}}` for `{selector}`");
        };
        let body = &rest[open + 1..close];
        if matches!(selector, ".pele" | ":root") {
            for declaration in body.split(';').map(str::trim).filter(|d| !d.is_empty()) {
                let Some((name, value)) = declaration.split_once(':') else {
                    bail!("invalid Style block: `{declaration}` is not `name: value`");
                };
                let (name, value) = (name.trim(), value.trim());
                if name.is_empty() || value.is_empty() {
                    bail!("invalid Style block: `{declaration}` is not `name: value`");
                }
                if name.starts_with("--") {
                    vars.insert(name.to_string(), value.to_string());
                }
            }
        }
        rest = rest[close + 1..].trim();
    }
    Ok(vars)
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, ch) in text[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_custom_properties_from_pele_rule() {
        let vars = parse_variables(".pele { --pele-bg: #fffcf0; --pele-accent: #205ea6; }").unwrap();
        assert_eq!(vars["--pele-bg"], "#fffcf0");
        assert_eq!(vars["--pele-accent"], "#205ea6");
    }

    #[test]
    fn keeps_nested_var_references_verbatim() {
        let vars = parse_variables(":root { --brand: #123; --pele-accent: var(--brand, red) }").unwrap();
        assert_eq!(vars["--pele-accent"], "var(--brand, red)");
    }

    #[test]
    fn ignores_comments_and_other_rules() {
        let vars = parse_variables("/* hi */ .other { color: red } .pele { --pele-fg: #000 }").unwrap();
        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn rejects_malformed_blocks() {
        assert!(parse_variables(".pele { --pele-bg: #fff").is_err());
        assert!(parse_variables(".pele { --pele-bg }").is_err());
        assert!(parse_variables("{ --pele-bg: #fff }").is_err());
        assert!(parse_variables("--pele-bg: #fff;").is_err());
    }
}
