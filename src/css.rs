use std::collections::HashMap;

use anyhow::{Result, bail};

/// A parsed Style block: the custom properties declared on the root (`.pele` or `:root`),
/// and every other rule re-serialized as plain CSS for resvg to apply.
#[derive(Debug, Default, PartialEq)]
pub struct StyleBlock {
    pub vars: HashMap<String, String>,
    pub rules: String,
}

pub fn parse(css: &str) -> Result<StyleBlock> {
    let css = strip_comments(css);
    let mut block = StyleBlock::default();
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
        if !selector.starts_with('@') {
            let is_root = matches!(selector, ".pele" | ":root");
            let mut kept = Vec::new();
            for declaration in body.split(';').map(str::trim).filter(|d| !d.is_empty()) {
                let Some((name, value)) = declaration.split_once(':') else {
                    bail!("invalid Style block: `{declaration}` is not `name: value`");
                };
                let (name, value) = (name.trim(), value.trim());
                if name.is_empty() || value.is_empty() {
                    bail!("invalid Style block: `{declaration}` is not `name: value`");
                }
                if name.starts_with("--") {
                    if is_root {
                        block.vars.insert(name.to_string(), value.to_string());
                    }
                } else {
                    kept.push(format!("{name}:{value}"));
                }
            }
            if !kept.is_empty() {
                block.rules.push_str(&format!("{selector}{{{}}}\n", kept.join(";")));
            }
        }
        rest = rest[close + 1..].trim();
    }
    Ok(block)
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
        let vars = parse(".pele { --pele-bg: #fffcf0; --pele-accent: #205ea6; }").unwrap().vars;
        assert_eq!(vars["--pele-bg"], "#fffcf0");
        assert_eq!(vars["--pele-accent"], "#205ea6");
    }

    #[test]
    fn keeps_nested_var_references_verbatim() {
        let vars = parse(":root { --brand: #123; --pele-accent: var(--brand, red) }").unwrap().vars;
        assert_eq!(vars["--pele-accent"], "var(--brand, red)");
    }

    #[test]
    fn keeps_other_rules_and_ignores_comments() {
        let block = parse("/* hi */ .pele-node { stroke-width: 2 } .pele { --pele-fg: #000 }").unwrap();
        assert_eq!(block.vars.len(), 1);
        assert_eq!(block.rules, ".pele-node{stroke-width:2}\n");
    }

    #[test]
    fn keeps_compound_ancestor_and_attribute_selectors() {
        let block = parse(".pele-node[data-id=\"A\"] .pele-label { fill: var(--pele-accent) }").unwrap();
        assert_eq!(block.rules, ".pele-node[data-id=\"A\"] .pele-label{fill:var(--pele-accent)}\n");
    }

    #[test]
    fn keeps_plain_declarations_from_the_root_rule() {
        let block = parse(".pele { --pele-bg: #fff; stroke-width: 3 }").unwrap();
        assert_eq!(block.vars.len(), 1);
        assert_eq!(block.rules, ".pele{stroke-width:3}\n");
    }

    #[test]
    fn skips_at_rules_for_now() {
        let block = parse("@media (prefers-color-scheme: dark) { .pele { --pele-bg: #000 } }").unwrap();
        assert_eq!(block, StyleBlock::default());
    }

    #[test]
    fn rejects_malformed_blocks() {
        assert!(parse(".pele { --pele-bg: #fff").is_err());
        assert!(parse(".pele { --pele-bg }").is_err());
        assert!(parse("{ --pele-bg: #fff }").is_err());
        assert!(parse("--pele-bg: #fff;").is_err());
    }
}
