//! The colour theme of an output. Each SVG writer puts the dark palette in
//! `@media(prefers-color-scheme:dark)` blocks, so one text change here sets
//! the theme of each diagram type.

const DARK_OPEN: &str = "@media(prefers-color-scheme:dark){";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    /// Both palettes. The viewer picks one.
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub fn from_name(name: &str) -> Option<Theme> {
        match name {
            "auto" => Some(Theme::Auto),
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

/// Replaces each dark block of `svg` with `f(rules)`, where `rules` is the
/// text between the braces of the block.
fn map_dark(svg: &str, f: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find(DARK_OPEN) {
        out.push_str(&rest[..at]);
        let body = &rest[at + DARK_OPEN.len()..];
        // The rules hold no nested blocks, so the first `}}` or a `}` at the
        // end of the style ends the block. Count the braces to be sure.
        let mut depth = 1;
        let mut end = body.len();
        for (k, ch) in body.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = k;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push_str(&f(&body[..end]));
        rest = body.get(end + 1..).unwrap_or("");
    }
    out.push_str(rest);
    out
}

/// The SVG with the theme applied. `Light` removes the dark rules. `Dark`
/// applies them always: they come after the light rules, so they win.
pub fn apply_theme(svg: &str, theme: Theme) -> String {
    match theme {
        Theme::Auto => svg.to_string(),
        Theme::Light => map_dark(svg, |_| String::new()),
        Theme::Dark => map_dark(svg, str::to_string),
    }
}

/// Puts `prefix` before each selector of the rules.
fn prefix_rules(rules: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(rules.len() * 2);
    for rule in rules.split_inclusive('}') {
        let Some((sel, decl)) = rule.split_once('{') else {
            out.push_str(rule);
            continue;
        };
        let sels: Vec<String> = sel.split(',').map(|s| format!("{prefix}{}", s.trim())).collect();
        out.push_str(&sels.join(","));
        out.push('{');
        out.push_str(decl);
    }
    out
}

/// The dark rules for a page that sets `data-theme` on its root element.
/// The attribute wins over the system setting. `:where()` adds no
/// specificity, so the rules keep their order of precedence.
pub fn page_theme(svg: &str) -> String {
    map_dark(svg, |rules| {
        format!(
            "{DARK_OPEN}{}}}{}",
            prefix_rules(rules, ":where(:root:not([data-theme=light])) "),
            prefix_rules(rules, ":where(:root[data-theme=dark]) ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = "<style>.a{fill:red}@media(prefers-color-scheme:dark){.a,.b{fill:blue}}.c{x:1}@media(prefers-color-scheme:dark){.c{x:2}}</style>";

    #[test]
    fn light_removes_the_dark_rules() {
        assert_eq!(apply_theme(SVG, Theme::Light), "<style>.a{fill:red}.c{x:1}</style>");
    }

    #[test]
    fn dark_applies_the_dark_rules_always() {
        assert_eq!(apply_theme(SVG, Theme::Dark), "<style>.a{fill:red}.a,.b{fill:blue}.c{x:1}.c{x:2}</style>");
    }

    #[test]
    fn auto_keeps_both_palettes() {
        assert_eq!(apply_theme(SVG, Theme::Auto), SVG);
    }

    #[test]
    fn the_page_attribute_wins_over_the_system() {
        let out = page_theme("@media(prefers-color-scheme:dark){.a,text{fill:blue}}");
        assert_eq!(
            out,
            "@media(prefers-color-scheme:dark){:where(:root:not([data-theme=light])) .a,:where(:root:not([data-theme=light])) text{fill:blue}}:where(:root[data-theme=dark]) .a,:where(:root[data-theme=dark]) text{fill:blue}"
        );
    }
}
