//! Parser for the diagram language. See docs/dsl.md.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::model::{
    is_arrow_word, Diagram, DiagramError, DiagramType, Edge, Item, Member, ERD_FLAGS, STATUS_WORDS, TYPES,
};
use crate::parse_story::StoryLines;

pub(crate) enum Tok<'a> {
    Word(&'a str),
    /// A quoted string, without the quotes and the escapes.
    Str(Cow<'a, str>),
}

impl<'a> Tok<'a> {
    pub(crate) fn word(&self) -> Option<&'a str> {
        match self {
            Tok::Word(w) => Some(w),
            Tok::Str(_) => None,
        }
    }

    pub(crate) fn is_str(&self) -> bool {
        matches!(self, Tok::Str(_))
    }

    fn is_arrow(&self) -> bool {
        matches!(self, Tok::Word(w) if is_arrow_word(w))
    }

    pub(crate) fn text(&self) -> &str {
        match self {
            Tok::Word(w) => w,
            Tok::Str(s) => s,
        }
    }
}

// The tokenizer reads bytes. All delimiters are ASCII, and a byte of a
// longer UTF-8 sequence is never equal to an ASCII byte.
fn tokenize<'a>(line: &'a str, ln: usize, errs: &mut Vec<String>, out: &mut Vec<Tok<'a>>) {
    let b = line.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        match b[i] {
            b' ' | b'\t' => i += 1,
            b'#' => break,
            b'"' => {
                i += 1;
                // The text from `seg` to `i` is not yet in `owned`.
                let mut seg = i;
                let mut owned: Option<String> = None;
                let mut closed = false;
                while i < n {
                    if b[i] == b'\\' && i + 1 < n {
                        // The character after the backslash is literal.
                        owned.get_or_insert_with(String::new).push_str(&line[seg..i]);
                        seg = i + 1;
                        i += 2;
                    } else if b[i] == b'"' {
                        closed = true;
                        break;
                    } else {
                        i += 1;
                    }
                }
                let tail = &line[seg..i];
                if closed {
                    i += 1;
                } else {
                    errs.push(format!("{ln}: the string has no closing quote"));
                }
                out.push(Tok::Str(match owned {
                    Some(mut s) => {
                        s.push_str(tail);
                        Cow::Owned(s)
                    }
                    None => Cow::Borrowed(tail),
                }));
            }
            _ => {
                let mut j = i;
                while j < n && b[j] != b' ' && b[j] != b'\t' && b[j] != b'"' {
                    j += 1;
                }
                out.push(Tok::Word(&line[i..j]));
                i = j;
            }
        }
    }
}

fn indent_of(line: &str) -> i64 {
    let mut w = 0;
    for &c in line.as_bytes() {
        match c {
            b' ' => w += 1,
            b'\t' => w += 2,
            _ => break,
        }
    }
    w
}

fn is_ancestor(items: &[Item], a: usize, b: usize) -> bool {
    let mut p = items[b].parent;
    while let Some(q) = p {
        if q == a {
            return true;
        }
        p = items[q].parent;
    }
    false
}

/// Index in `STATUS_WORDS`. Index 0 is the empty word, so it is not a match.
fn status_of(word: &str) -> Option<usize> {
    STATUS_WORDS.iter().skip(1).position(|&s| s == word).map(|i| i + 1)
}

fn attach(d: &mut Diagram, idx: usize, parent: Option<usize>) {
    d.items[idx].parent = parent;
    match parent {
        None => d.roots.push(idx),
        Some(p) => d.items[p].children.push(idx),
    }
}

fn detach(d: &mut Diagram, idx: usize) {
    let list = match d.items[idx].parent {
        None => &mut d.roots,
        Some(p) => &mut d.items[p].children,
    };
    if let Some(at) = list.iter().position(|&x| x == idx) {
        list.remove(at);
    }
}

fn add(d: &mut Diagram, id: &str, group: bool, parent: Option<usize>, ln: usize, implicit: bool) -> usize {
    let idx = d.items.len();
    d.items.push(Item {
        id: id.to_string(),
        label: id.to_string(),
        sub: String::new(),
        kind: 0,
        status: 0,
        group,
        parent: None,
        children: Vec::new(),
        line: ln,
        implicit,
        icon: String::new(),
        vendor: false,
        members: Vec::new(),
    });
    attach(d, idx, parent);
    idx
}

/// Adds a member line to the item `owner`. `None` is an item with an error.
fn add_member(d: &mut Diagram, owner: Option<usize>, toks: &[Tok], ln: usize, errs: &mut Vec<String>) {
    let mut words: Vec<String> = toks.iter().map(|t| t.text().to_string()).collect();
    let mut status = 0;
    if words.len() > 1 {
        if let Some(s) = status_of(&words[words.len() - 1]) {
            status = s;
            words.pop();
        }
    }
    if d.dtype == DiagramType::Erd {
        if words.len() < 2 {
            errs.push(format!("{ln}: write a field as: name type flags status"));
            return;
        }
        for w in &words[2..] {
            if !ERD_FLAGS.contains(&w.as_str()) {
                errs.push(format!("{ln}: unknown flag '{w}'. Flags: {}", ERD_FLAGS.join(" ")));
            }
        }
    }
    if let Some(owner) = owner {
        d.items[owner].members.push(Member { text: words.join(" "), words, status, line: ln });
    }
}

pub fn parse(src: &str) -> Result<Diagram, DiagramError> {
    let mut errs: Vec<String> = Vec::new();
    let mut d = Diagram {
        dtype: DiagramType::Arch,
        title: String::new(),
        down: false,
        items: Vec::new(),
        edges: Vec::new(),
        roots: Vec::new(),
        stories: Vec::new(),
    };
    let mut stories = StoryLines::default();
    let mut by_id: HashMap<&str, usize> = HashMap::new();
    // The open groups, with the indent of each group line.
    let mut stack: Vec<(i64, Option<usize>)> = vec![(-1, None)];
    let mut header = false;
    let mut toks: Vec<Tok> = Vec::new();
    let mut ends: Vec<usize> = Vec::new();
    // The node of the last node line, and its indent. A line with more indent
    // is a member line. The node is `None` when the node line has an error.
    let mut member_of: Option<(Option<usize>, i64)> = None;

    // Some editors put a byte order mark at the start of the file.
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    for (li, raw) in src.split('\n').enumerate() {
        let ln = li + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        toks.clear();
        tokenize(line, ln, &mut errs, &mut toks);
        if toks.is_empty() {
            continue;
        }

        if !header {
            header = true;
            match toks[0].word().and_then(DiagramType::from_name) {
                Some(t) => d.dtype = t,
                None => {
                    errs.push(format!("{ln}: the first line must name a diagram type: {}", TYPES.join(", ")));
                    d.dtype = DiagramType::Arch;
                }
            }
            d.down = d.dtype.default_down();
            if toks.len() > 1 && toks[1].is_str() {
                d.title = toks[1].text().to_string();
            }
            if toks.len() > 2 || (toks.len() == 2 && !toks[1].is_str()) {
                errs.push(format!("{ln}: the first line takes a type and one optional title string"));
            }
            continue;
        }

        let indent = indent_of(line);
        if stories.take(&toks, indent, ln) {
            member_of = None;
            continue;
        }
        if let Some((owner, base)) = member_of {
            if indent > base {
                add_member(&mut d, owner, &toks, ln, &mut errs);
                continue;
            }
            member_of = None;
        }
        while stack.len() > 1 && indent <= stack[stack.len() - 1].0 {
            stack.pop();
        }
        let cur = stack[stack.len() - 1].1;

        if toks.iter().any(Tok::is_arrow) {
            if d.dtype == DiagramType::Timeline {
                errs.push(format!("{ln}: a timeline diagram has no edges"));
                continue;
            }
            // id (arrow id)+ ["label"] [status]
            // The ids are the tokens 0, 2, 4, ... and the arrows are between them.
            let mut ids = 0;
            let mut arrows = 0;
            let mut k = 0;
            let mut ok = true;
            while k < toks.len() {
                if toks[k].is_str() || toks[k].is_arrow() {
                    ok = false;
                    break;
                }
                ids += 1;
                k += 1;
                if k < toks.len() && toks[k].is_arrow() {
                    arrows += 1;
                    k += 1;
                    if k >= toks.len() {
                        ok = false;
                    }
                } else {
                    break;
                }
            }
            if !ok || arrows == 0 || ids != arrows + 1 {
                errs.push(format!("{ln}: write an edge as: a -> b \"label\" status"));
                continue;
            }
            let allowed = d.dtype.arrows();
            if let Some(bad) = toks[..k].iter().skip(1).step_by(2).find(|t| !allowed.contains(&t.text())) {
                let list = if d.dtype == DiagramType::Erd {
                    "a--b, with each end one of 1 ? * +".to_string()
                } else {
                    allowed.join(" ")
                };
                errs.push(format!("{ln}: unknown arrow '{}'. Arrows of {}: {list}", bad.text(), d.dtype.name()));
                continue;
            }
            let mut label = "";
            let mut status = 0;
            let mut seen_label = false;
            for t in &toks[k..] {
                if t.is_str() && !seen_label {
                    label = t.text();
                    seen_label = true;
                } else if let Some(s) = t.word().and_then(status_of) {
                    status = s;
                } else {
                    errs.push(format!("{ln}: unknown word '{}' after the edge", t.text()));
                }
            }
            ends.clear();
            for t in toks[..k].iter().step_by(2) {
                let Tok::Word(id) = *t else { unreachable!() };
                let found = match by_id.get(id) {
                    Some(&found) => found,
                    None => {
                        let new = add(&mut d, id, false, cur, ln, true);
                        by_id.insert(id, new);
                        new
                    }
                };
                ends.push(found);
            }
            for a in 0..arrows {
                let from = ends[a];
                let to = ends[a + 1];
                if d.dtype != DiagramType::Seq {
                    // A state can have a transition to itself.
                    if from == to && d.dtype != DiagramType::State {
                        errs.push(format!("{ln}: '{}' has an edge to itself", toks[2 * a].text()));
                        continue;
                    }
                    if is_ancestor(&d.items, from, to) || is_ancestor(&d.items, to, from) {
                        errs.push(format!("{ln}: an edge cannot join a group and its own member"));
                        continue;
                    }
                }
                let word = toks[2 * a + 1].text();
                let arrow = allowed[allowed.iter().position(|&x| x == word).unwrap_or_default()];
                d.edges.push(Edge {
                    from,
                    to,
                    label: label.to_string(),
                    dashed: matches!(arrow, "-->" | "..|>" | "..>"),
                    bidir: arrow == "<->",
                    main: arrow == "=>",
                    status,
                    line: ln,
                    arrow,
                });
            }
            continue;
        }

        let first = toks[0].word();
        if first == Some("dir") {
            let v = if toks.len() == 2 { toks[1].word() } else { None };
            match v {
                Some("right") => d.down = false,
                Some("down") => d.down = true,
                _ => errs.push(format!("{ln}: write the direction as: dir right, or dir down")),
            }
            continue;
        }

        let is_group = first == Some("group");
        let mut k = usize::from(is_group);
        let Some(id) = toks.get(k).and_then(Tok::word) else {
            errs.push(format!("{ln}: the line must start with an id"));
            continue;
        };
        k += 1;
        if is_group && d.dtype == DiagramType::Seq {
            errs.push(format!("{ln}: a seq diagram has no groups"));
            continue;
        }
        if is_group && d.dtype == DiagramType::Timeline && cur.is_some() {
            errs.push(format!("{ln}: a timeline group cannot hold a group"));
            continue;
        }
        let has_members = d.dtype.has_members() && !is_group;
        if has_members {
            member_of = Some((None, indent));
        }
        let idx = match by_id.get(id) {
            None => {
                let new = add(&mut d, id, is_group, cur, ln, false);
                by_id.insert(id, new);
                new
            }
            Some(&found) if d.items[found].implicit && !is_group => {
                // An edge used this id first. The declaration sets the group.
                d.items[found].implicit = false;
                d.items[found].line = ln;
                detach(&mut d, found);
                attach(&mut d, found, cur);
                found
            }
            Some(&found) => {
                errs.push(format!("{ln}: the id '{id}' is already on line {}", d.items[found].line));
                continue;
            }
        };
        if has_members {
            member_of = Some((Some(idx), indent));
        }
        let kinds = d.dtype.kinds();
        let it = &mut d.items[idx];
        let mut strings = 0;
        for t in &toks[k..] {
            let v = t.text();
            if t.is_str() {
                if strings == 0 {
                    it.label = v.to_string();
                } else if strings == 1 && !is_group {
                    it.sub = v.to_string();
                } else {
                    errs.push(format!("{ln}: too many strings"));
                }
                strings += 1;
            } else if let Some(s) = status_of(v) {
                it.status = s;
            } else if let Some(name) = v.strip_prefix('@') {
                it.icon = name.to_string();
            } else if let Some(name) = v.strip_prefix(':') {
                match kinds.iter().position(|&x| x == name) {
                    Some(kind) if !is_group => it.kind = kind,
                    None if is_group && name == "vendor" => it.vendor = true,
                    _ => {
                        let list: Vec<String> = kinds.iter().map(|x| format!(":{x}")).collect();
                        errs.push(format!("{ln}: unknown kind '{v}'. Kinds: {}", list.join(" ")));
                    }
                }
            } else {
                errs.push(format!("{ln}: unknown word '{v}'. Statuses: {}", STATUS_WORDS[1..].join(" ")));
            }
        }
        if is_group {
            stack.push((indent, Some(idx)));
        }
    }

    // A node line after an edge can move the node into a group. Check the
    // edges again, because the first check saw the old group.
    if d.dtype != DiagramType::Seq {
        for e in &d.edges {
            if is_ancestor(&d.items, e.from, e.to) || is_ancestor(&d.items, e.to, e.from) {
                errs.push(format!("{}: an edge cannot join a group and its own member", e.line));
            }
        }
    }

    stories.resolve(&mut d, &by_id, &mut errs);

    if !header {
        errs.push(format!("1: the file is empty. The first line must name a diagram type: {}", TYPES.join(", ")));
    }
    if !errs.is_empty() {
        return Err(DiagramError { errors: errs });
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> (Vec<String>, Vec<String>) {
        let mut errs = Vec::new();
        let mut toks = Vec::new();
        tokenize(line, 1, &mut errs, &mut toks);
        let shown =
            toks.iter().map(|t| if t.is_str() { format!("\"{}\"", t.text()) } else { t.text().to_string() }).collect();
        (shown, errs)
    }

    #[test]
    fn tokenizer_splits_words_and_strings() {
        let (toks, errs) = words("  a\t->b\"x y\"c # note");
        assert_eq!(toks, ["a", "->b", "\"x y\"", "c"]);
        assert!(errs.is_empty());
    }

    #[test]
    fn tokenizer_keeps_the_character_after_a_backslash() {
        let (toks, errs) = words(r#""a\"b\\c\é" "tail\"#);
        assert_eq!(toks, ["\"a\"b\\cé\"", "\"tail\\\""]);
        assert_eq!(errs, ["1: the string has no closing quote"]);
    }

    #[test]
    fn a_number_sign_inside_a_word_is_not_a_comment() {
        let (toks, _) = words("a#b #c");
        assert_eq!(toks, ["a#b"]);
    }

    #[test]
    fn a_declaration_moves_an_implicit_item() {
        let d = parse("arch\na -> b\ngroup g\n  a \"A\"\n").unwrap();
        assert_eq!(d.roots, [1, 2]);
        assert_eq!(d.items[2].children, [0]);
        assert_eq!(d.items[0].parent, Some(2));
        assert!(!d.items[0].implicit);
        assert!(d.items[1].implicit);
    }

    #[test]
    fn each_type_has_its_default_direction() {
        for (t, down) in [
            ("arch", false),
            ("plan", true),
            ("deps", true),
            ("flow", true),
            ("state", false),
            ("dataflow", false),
            ("change", false),
            ("erd", false),
            ("class", true),
            ("timeline", false),
        ] {
            assert_eq!(parse(t).unwrap().down, down, "{t}");
        }
    }

    #[test]
    fn a_kind_is_an_index_in_the_kinds_of_the_type() {
        let d = parse(
            "flow
a :decision
b :io
c
",
        )
        .unwrap();
        let kinds: Vec<&str> = d.items.iter().map(|it| d.dtype.kinds()[it.kind]).collect();
        assert_eq!(kinds, ["decision", "io", "step"]);
    }

    #[test]
    fn an_edge_keeps_its_arrow_word() {
        let d = parse(
            "class
a ..|> b
b *-- c --|> d
d ..> e
",
        )
        .unwrap();
        let arrows: Vec<&str> = d.edges.iter().map(|e| e.arrow).collect();
        assert_eq!(arrows, ["..|>", "*--", "--|>", "..>"]);
        let dashed: Vec<bool> = d.edges.iter().map(|e| e.dashed).collect();
        assert_eq!(dashed, [true, false, false, true]);
    }

    #[test]
    fn erd_arrows_join_two_end_marks() {
        let d = parse(
            "erd
users 1--* orders \"places\"
orders ?--+ items
",
        )
        .unwrap();
        assert_eq!(d.edges[0].arrow, "1--*");
        assert_eq!(d.edges[0].label, "places");
        assert_eq!(d.edges[1].arrow, "?--+");
    }

    #[test]
    fn a_state_can_have_a_transition_to_itself() {
        let d = parse(
            "state
draft -> draft \"edit\"
",
        )
        .unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!((d.edges[0].from, d.edges[0].to), (0, 0));
        assert!(parse(
            "flow
a -> a
"
        )
        .is_err());
    }

    #[test]
    fn member_lines_go_to_the_node_above() {
        let src = "erd
group s \"Schema\"
  users \"User\"
    id uuid pk
    email text uniq add
  orders
    id uuid pk
users 1--* orders
";
        let d = parse(src).unwrap();
        let users = &d.items[1];
        assert_eq!(users.members.len(), 2);
        assert_eq!(users.members[0].words, ["id", "uuid", "pk"]);
        assert_eq!(users.members[1].text, "email text uniq");
        assert_eq!(users.members[1].status, 4);
        assert_eq!(users.members[1].line, 5);
        assert_eq!(d.items[2].members.len(), 1);
        assert_eq!(d.items[2].parent, Some(0));
        assert_eq!(d.edges.len(), 1);
    }

    #[test]
    fn a_class_member_keeps_its_text() {
        let d = parse(
            "class
store :interface
  + get(key) Value
  + put(key, value)
  - apply(f) -> Value
",
        )
        .unwrap();
        let texts: Vec<&str> = d.items[0].members.iter().map(|m| m.text.as_str()).collect();
        assert_eq!(texts, ["+ get(key) Value", "+ put(key, value)", "- apply(f) -> Value"]);
        assert!(d.edges.is_empty());
    }

    #[test]
    fn only_erd_and_class_have_member_lines() {
        // In arch, an indented line under a node is a node of the same group.
        let d = parse(
            "arch
a
  b
",
        )
        .unwrap();
        assert_eq!(d.items.len(), 2);
        assert!(d.items[0].members.is_empty());
    }

    #[test]
    fn a_timeline_has_no_edges() {
        let err = parse(
            "timeline
a
b
a -> b
",
        )
        .unwrap_err();
        assert_eq!(err.errors, ["4: a timeline diagram has no edges"]);
    }
}
