use std::collections::HashMap;

use anyhow::{Result, bail};

/// A parsed Style block: the custom properties declared on the root (`.pele` or `:root`),
/// and every other rule re-serialized as plain CSS for resvg to apply.
#[derive(Debug, Default, PartialEq)]
pub struct StyleBlock {
    pub vars: HashMap<String, String>,
    pub rules: String,
    /// Problems that did not stop the render, such as an unavailable font family.
    pub warnings: Vec<String>,
}

/// Families the binary embeds, and the generic names resvg maps onto them.
const EMBEDDED_FAMILIES: [&str; 2] = ["Inter", "JetBrains Mono"];
const GENERIC_FAMILIES: [&str; 3] = ["serif", "sans-serif", "monospace"];

/// Keep only the families the binary can draw. An unavailable family is dropped with a
/// warning; if none are left, `fallback` is used. Names are single-quoted so they survive
/// inside the SVG's double-quoted attributes.
fn sanitize_font_family(value: &str, fallback: &str, warnings: &mut Vec<String>) -> String {
    if value.contains("var(") {
        return value.to_string();
    }
    let mut kept = Vec::new();
    for entry in value.split(',') {
        let name = entry.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        if name.is_empty() {
            continue;
        }
        let generic = GENERIC_FAMILIES.iter().any(|f| f.eq_ignore_ascii_case(name));
        let embedded = EMBEDDED_FAMILIES.iter().any(|f| f.eq_ignore_ascii_case(name));
        if generic {
            kept.push(name.to_ascii_lowercase());
        } else if embedded {
            kept.push(format!("'{name}'"));
        } else {
            warnings.push(format!("font family \"{name}\" is not embedded; using {fallback} instead"));
        }
    }
    if kept.is_empty() {
        format!("'{fallback}'")
    } else {
        kept.join(", ")
    }
}

/// Which of a Style block's light or dark values apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, clap::ValueEnum)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

impl ColorScheme {
    fn name(self) -> &'static str {
        match self {
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        }
    }
}

pub fn parse(css: &str, scheme: ColorScheme) -> Result<StyleBlock> {
    let mut block = StyleBlock::default();
    parse_into(&strip_comments(css), scheme, &mut block)?;
    let mut seen = std::collections::HashSet::new();
    block.warnings.retain(|warning| seen.insert(warning.clone()));
    Ok(block)
}

fn parse_into(css: &str, scheme: ColorScheme, block: &mut StyleBlock) -> Result<()> {
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
        if selector.starts_with('@') {
            // Only a `prefers-color-scheme` media query matching the chosen scheme applies.
            let query: String = selector.chars().filter(|c| !c.is_whitespace()).collect();
            let wanted = format!("@media(prefers-color-scheme:{})", scheme.name());
            if query.eq_ignore_ascii_case(&wanted) {
                parse_into(body, scheme, block)?;
            }
        } else {
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
                        let value = match name {
                            "--pele-font" => sanitize_font_family(value, "Inter", &mut block.warnings),
                            "--pele-font-mono" => sanitize_font_family(value, "JetBrains Mono", &mut block.warnings),
                            _ => value.to_string(),
                        };
                        block.vars.insert(name.to_string(), value);
                    }
                } else if name == "font-family" {
                    let value = sanitize_font_family(value, "Inter", &mut block.warnings);
                    kept.push(format!("{name}:{value}"));
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
    Ok(())
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

    fn parse_light(css: &str) -> Result<StyleBlock> {
        parse(css, ColorScheme::Light)
    }

    const SCHEMED: &str = ".pele { --pele-bg: #fff; --pele-fg: #111 }
        @media (prefers-color-scheme: dark) { .pele { --pele-bg: #000 } .pele-node { stroke-width: 4 } }";

    #[test]
    fn light_scheme_ignores_dark_media_query() {
        let block = parse(SCHEMED, ColorScheme::Light).unwrap();
        assert_eq!(block.vars["--pele-bg"], "#fff");
        assert_eq!(block.rules, "");
    }

    #[test]
    fn dark_scheme_applies_media_query_over_base_values() {
        let block = parse(SCHEMED, ColorScheme::Dark).unwrap();
        assert_eq!(block.vars["--pele-bg"], "#000");
        assert_eq!(block.vars["--pele-fg"], "#111");
        assert_eq!(block.rules, ".pele-node{stroke-width:4}\n");
    }

    #[test]
    fn light_media_query_applies_only_to_light() {
        let css = "@media (prefers-color-scheme: light) { .pele { --pele-bg: #eee } }";
        assert_eq!(parse(css, ColorScheme::Light).unwrap().vars["--pele-bg"], "#eee");
        assert!(parse(css, ColorScheme::Dark).unwrap().vars.is_empty());
    }

    #[test]
    fn collects_custom_properties_from_pele_rule() {
        let vars = parse_light(".pele { --pele-bg: #fffcf0; --pele-accent: #205ea6; }").unwrap().vars;
        assert_eq!(vars["--pele-bg"], "#fffcf0");
        assert_eq!(vars["--pele-accent"], "#205ea6");
    }

    #[test]
    fn keeps_nested_var_references_verbatim() {
        let vars = parse_light(":root { --brand: #123; --pele-accent: var(--brand, red) }").unwrap().vars;
        assert_eq!(vars["--pele-accent"], "var(--brand, red)");
    }

    #[test]
    fn keeps_other_rules_and_ignores_comments() {
        let block = parse_light("/* hi */ .pele-node { stroke-width: 2 } .pele { --pele-fg: #000 }").unwrap();
        assert_eq!(block.vars.len(), 1);
        assert_eq!(block.rules, ".pele-node{stroke-width:2}\n");
    }

    #[test]
    fn keeps_compound_ancestor_and_attribute_selectors() {
        let block = parse_light(".pele-node[data-id=\"A\"] .pele-label { fill: var(--pele-accent) }").unwrap();
        assert_eq!(block.rules, ".pele-node[data-id=\"A\"] .pele-label{fill:var(--pele-accent)}\n");
    }

    #[test]
    fn keeps_plain_declarations_from_the_root_rule() {
        let block = parse_light(".pele { --pele-bg: #fff; stroke-width: 3 }").unwrap();
        assert_eq!(block.vars.len(), 1);
        assert_eq!(block.rules, ".pele{stroke-width:3}\n");
    }

    #[test]
    fn skips_unrelated_at_rules() {
        let block = parse_light("@media (min-width: 600px) { .pele { --pele-bg: #000 } }").unwrap();
        assert_eq!(block, StyleBlock::default());
    }

    #[test]
    fn unavailable_font_is_dropped_with_a_warning_and_falls_back() {
        let block = parse_light(".pele { --pele-font: \"Helvetica Neue\" }").unwrap();
        assert_eq!(block.vars["--pele-font"], "'Inter'");
        assert_eq!(block.warnings, ["font family \"Helvetica Neue\" is not embedded; using Inter instead"]);
    }

    #[test]
    fn unavailable_mono_font_falls_back_to_jetbrains_mono() {
        let block = parse_light(".pele { --pele-font-mono: Menlo }").unwrap();
        assert_eq!(block.vars["--pele-font-mono"], "'JetBrains Mono'");
        assert_eq!(block.warnings, ["font family \"Menlo\" is not embedded; using JetBrains Mono instead"]);
    }

    #[test]
    fn embedded_and_generic_families_are_kept_without_warnings() {
        let block = parse_light(".pele { --pele-font: 'Inter', sans-serif; --pele-font-mono: \"JetBrains Mono\", monospace }").unwrap();
        assert_eq!(block.vars["--pele-font"], "'Inter', sans-serif");
        assert_eq!(block.vars["--pele-font-mono"], "'JetBrains Mono', monospace");
        assert!(block.warnings.is_empty());
    }

    #[test]
    fn a_missing_family_is_dropped_but_a_listed_fallback_is_kept() {
        let block = parse_light(".pele { --pele-font: Papyrus, sans-serif }").unwrap();
        assert_eq!(block.vars["--pele-font"], "sans-serif");
        assert_eq!(block.warnings.len(), 1);
    }

    #[test]
    fn font_family_declarations_in_rules_are_checked_and_warnings_deduplicated() {
        let block = parse_light(".pele-label { font-family: Papyrus } .pele-member { font-family: Papyrus }").unwrap();
        assert_eq!(block.rules, ".pele-label{font-family:'Inter'}\n.pele-member{font-family:'Inter'}\n");
        assert_eq!(block.warnings.len(), 1);
    }

    #[test]
    fn rejects_malformed_blocks() {
        assert!(parse_light(".pele { --pele-bg: #fff").is_err());
        assert!(parse_light(".pele { --pele-bg }").is_err());
        assert!(parse_light("{ --pele-bg: #fff }").is_err());
        assert!(parse_light("--pele-bg: #fff;").is_err());
    }
}
