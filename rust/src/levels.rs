//! Levels: a node can have a detail level, from a `detail` block or from a
//! `>file` link. This module splits the source into levels, adds the context
//! of the parent level to each detail level, and sets the rolled-up status.
//! See docs/design/zoom.md.

use std::path::{Path, PathBuf};

use crate::model::{Diagram, DiagramError, DiagramType, Edge, Item, Zoom};
use crate::parse::{indent_of, parse, tokenize, Tok};

/// A path has four levels at most: the top level and three detail levels.
pub const MAX_LEVELS: usize = 4;

/// The id of the frame group that holds the parts of a detail level.
pub const FRAME: &str = "^";

#[derive(Debug)]
pub struct Level {
    /// The ids of the zoomed nodes from the top level down. Empty for the top.
    pub path: Vec<String>,
    pub d: Diagram,
    /// The parent level, and the index of the zoomed node in it.
    pub parent: Option<(usize, usize)>,
}

impl Level {
    /// The path as text: `cart/svc`. The top level gives an empty text.
    pub fn key(&self) -> String {
        self.path.join("/")
    }
}

/// A `detail` block of a source text.
struct Block {
    line: usize,
    id: String,
    dtype: Option<DiagramType>,
    title: Option<String>,
    /// A source text with the same line numbers as the file: the header at
    /// the `detail` line, the body without its indent, and empty lines.
    body: String,
}

/// A block that is open: the block, the indent of its first body line
/// (-1 before it), and its body lines with their line indexes.
type OpenBlock = (Block, i64, Vec<(usize, String)>);

/// Splits `text` into the text of its own level and its `detail` blocks.
/// Both keep the line numbers of `text`.
fn split(text: &str, errs: &mut Vec<String>) -> (String, Vec<Block>) {
    let lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut own: Vec<&str> = Vec::with_capacity(lines.len());
    let mut blocks: Vec<Block> = Vec::new();
    let mut open: Option<OpenBlock> = None;
    let mut header = false;
    let mut toks: Vec<Tok> = Vec::new();
    let mut quiet: Vec<String> = Vec::new();
    for (li, &line) in lines.iter().enumerate() {
        let ln = li + 1;
        toks.clear();
        quiet.clear();
        tokenize(line, ln, &mut quiet, &mut toks);
        if toks.is_empty() {
            // Keep the text: in a class, `# name` is a protected member, not a comment.
            match open.as_mut() {
                Some((_, base, body)) if *base >= 0 && indent_of(line) >= *base && !line.trim().is_empty() => {
                    body.push((li, dedent(line, *base)));
                    own.push("");
                }
                Some(_) => own.push(""),
                None => own.push(line),
            }
            continue;
        }
        let indent = indent_of(line);
        if let Some((_, base, body)) = open.as_mut() {
            if indent > 0 {
                if *base < 0 {
                    *base = indent;
                }
                if indent < *base {
                    errs.push(format!("{ln}: the line has less indent than the first line of the detail block"));
                } else {
                    body.push((li, dedent(line, *base)));
                }
                own.push("");
                continue;
            }
            close(open.take(), lines.len(), &mut blocks, errs);
        }
        let is_detail = header && indent == 0 && toks[0].word() == Some("detail") && !toks.iter().any(Tok::is_arrow);
        header = true;
        if !is_detail {
            own.push(line);
            continue;
        }
        errs.append(&mut quiet);
        own.push("");
        let mut b = Block { line: ln, id: String::new(), dtype: None, title: None, body: String::new() };
        match toks.get(1).and_then(Tok::word) {
            Some(id) => b.id = id.to_string(),
            None => errs.push(format!("{ln}: write a detail block as: detail <id> [type] [\"Title\"]")),
        }
        for t in toks.iter().skip(2) {
            match t {
                Tok::Str(s) if b.title.is_none() => b.title = Some(s.to_string()),
                Tok::Word(w) if b.dtype.is_none() && b.title.is_none() => match DiagramType::from_name(w) {
                    Some(dt) => b.dtype = Some(dt),
                    None => errs.push(format!(
                        "{ln}: unknown type '{w}' in the detail line. Types: {}",
                        crate::model::TYPES.join(", ")
                    )),
                },
                _ => errs.push(format!("{ln}: write a detail block as: detail <id> [type] [\"Title\"]")),
            }
        }
        open = Some((b, -1, Vec::new()));
    }
    close(open, lines.len(), &mut blocks, errs);
    (own.join("\n"), blocks)
}

/// The line without the first `base` columns of indent.
fn dedent(line: &str, base: i64) -> String {
    let mut w = 0;
    let mut at = 0;
    for (k, c) in line.char_indices() {
        if w >= base {
            break;
        }
        match c {
            ' ' => w += 1,
            '\t' => w += 2,
            _ => break,
        }
        at = k + 1;
    }
    line[at..].to_string()
}

fn close(open: Option<OpenBlock>, n: usize, blocks: &mut Vec<Block>, errs: &mut Vec<String>) {
    let Some((mut b, _, body)) = open else { return };
    if b.id.is_empty() {
        return;
    }
    if body.is_empty() {
        errs.push(format!("{}: the detail block of '{}' has no body. Indent its lines", b.line, b.id));
        return;
    }
    let mut out = vec![String::new(); n];
    // The parser reads the type from the first line. The caller sets it.
    out[b.line - 1] = "\u{0}".to_string();
    for (li, text) in body {
        out[li] = text;
    }
    b.body = out.join("\n");
    blocks.push(b);
}

/// Reads source texts into levels, the top level first, in depth-first order.
struct Loader<'a> {
    levels: Vec<Level>,
    errs: Vec<String>,
    /// The files of the open links, to find a cycle.
    files: Vec<PathBuf>,
    read: &'a dyn Fn(&Path) -> std::io::Result<String>,
}

/// Where a level hangs: the parent level, the node, and the title.
struct Spot {
    parent: usize,
    node: usize,
    title: Option<String>,
    /// The type word of a `detail` block.
    dtype: Option<DiagramType>,
}

impl Loader<'_> {
    /// `prefix` goes before each error, so an error in a linked file names
    /// the link line and the file.
    fn load(&mut self, text: &str, file: Option<&Path>, at: Option<Spot>, prefix: &str) {
        let mut errs = Vec::new();
        let (own, blocks) = split(text, &mut errs);
        let mut own = own;
        // A detail body takes the type of the parent unless the block names one.
        if let Some(s) = at.as_ref().filter(|_| own.contains('\u{0}')) {
            let parent_type = self.levels[s.parent].d.dtype;
            let dt = s.dtype.unwrap_or(parent_type);
            own = own.replacen('\u{0}', dt.name(), 1);
        }
        let d = match parse(&own) {
            Ok(d) => Some(d),
            Err(e) => {
                errs.extend(e.errors);
                None
            }
        };
        let Some(mut d) = d else {
            self.fail(prefix, errs);
            return;
        };
        let path = match &at {
            None => Vec::new(),
            Some(s) => {
                let mut p = self.levels[s.parent].path.clone();
                p.push(self.levels[s.parent].d.items[s.node].id.clone());
                p
            }
        };
        match &at {
            None => {
                for it in d.items.iter().filter(|it| it.id.starts_with('^')) {
                    errs.push(format!(
                        "{}: '{}' names a node of the parent level, but the top level has no parent",
                        it.line, it.id
                    ));
                }
            }
            Some(s) => {
                let pd = &self.levels[s.parent].d;
                let node = &pd.items[s.node];
                d.title = s.title.clone().unwrap_or_else(|| {
                    if d.title.is_empty() {
                        node.label.clone()
                    } else {
                        d.title.clone()
                    }
                });
                add_context(&mut d, pd, s.node, &mut errs);
            }
        }
        let me = self.levels.len();
        // Check the blocks against the nodes of this level.
        let mut wanted: Vec<(usize, Option<&Block>)> = Vec::new();
        for b in &blocks {
            match d.items.iter().position(|it| it.id == b.id && !it.zoom.context) {
                None => errs.push(format!("{}: detail '{}': this level has no node '{}'", b.line, b.id, b.id)),
                Some(i) if d.items[i].group => errs.push(format!(
                    "{}: detail '{}': '{}' is a group. Only a node can have a detail level",
                    b.line, b.id, b.id
                )),
                Some(i) if !d.items[i].zoom.link.is_empty() => errs.push(format!(
                    "{}: '{}' has a file link on line {}, so it cannot also have a detail block",
                    b.line, b.id, d.items[i].line
                )),
                Some(i) if wanted.iter().any(|w| w.0 == i) => {
                    errs.push(format!("{}: '{}' already has a detail block", b.line, b.id))
                }
                Some(i) => wanted.push((i, Some(b))),
            }
        }
        for (i, it) in d.items.iter().enumerate() {
            if !it.zoom.link.is_empty() {
                wanted.push((i, None));
            }
        }
        if !wanted.is_empty() && path.len() + 1 >= MAX_LEVELS {
            let line = wanted.iter().map(|w| w.1.map_or(d.items[w.0].line, |b| b.line)).min().unwrap_or(1);
            errs.push(format!("{line}: too many levels. A diagram has {MAX_LEVELS} levels at most"));
            wanted.clear();
        }
        // The levels below still load, so one run reports all errors.
        self.fail(prefix, errs);
        wanted.sort_by_key(|w| w.0);
        for &(i, _) in &wanted {
            d.items[i].zoom.detail = true;
        }
        self.levels.push(Level { path, d, parent: at.map(|s| (s.parent, s.node)) });
        for (i, block) in wanted {
            let spot = |title: Option<String>, dtype| Spot { parent: me, node: i, title, dtype };
            match block {
                Some(b) => self.load(&b.body, file, Some(spot(b.title.clone(), b.dtype)), prefix),
                None => {
                    let it = &self.levels[me].d.items[i];
                    let (link, line) = (it.zoom.link.clone(), it.line);
                    self.load_link(&link, line, file, spot(None, None), prefix);
                }
            }
        }
    }

    fn load_link(&mut self, link: &str, line: usize, file: Option<&Path>, at: Spot, prefix: &str) {
        let dir = file.and_then(Path::parent).unwrap_or(Path::new(""));
        let target = dir.join(link);
        let key = target.canonicalize().unwrap_or_else(|_| target.clone());
        if self.files.contains(&key) {
            self.errs.push(format!("{prefix}{line}: the file link '{link}' makes a cycle of files"));
            return;
        }
        let text = match (self.read)(&target) {
            Ok(text) => text,
            Err(_) => {
                self.errs.push(format!("{prefix}{line}: cannot read the linked file '{link}'"));
                return;
            }
        };
        self.files.push(key);
        self.load(&text, Some(&target), Some(at), &format!("{prefix}{line}: in {link}:"));
        self.files.pop();
    }

    fn fail(&mut self, prefix: &str, mut errs: Vec<String>) {
        errs.sort_by_key(|e| e.split(':').next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(0));
        self.errs.extend(errs.into_iter().map(|e| format!("{prefix}{e}")));
    }
}

/// Reads all levels of a source. `file` is the path of the source, for the
/// file links. Without a path, a link is relative to the current directory.
pub fn load(src: &str, file: Option<&Path>) -> Result<Vec<Level>, DiagramError> {
    load_with(src, file, &|p| std::fs::read_to_string(p))
}

/// `load` with another way to read a linked file.
pub fn load_with(
    src: &str,
    file: Option<&Path>,
    read: &dyn Fn(&Path) -> std::io::Result<String>,
) -> Result<Vec<Level>, DiagramError> {
    let mut l = Loader { levels: Vec::new(), errs: Vec::new(), files: Vec::new(), read };
    if let Some(f) = file {
        l.files.push(f.canonicalize().unwrap_or_else(|_| f.to_path_buf()));
    }
    l.load(src, file, None, "");
    if !l.errs.is_empty() {
        return Err(DiagramError { errors: l.errs });
    }
    rollup(&mut l.levels);
    Ok(l.levels)
}

/// The parent edges of node `node`: the edge, the other end, and true when
/// the edge comes in.
fn neighbours(pd: &Diagram, node: usize) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    for (k, e) in pd.edges.iter().enumerate() {
        if e.from == node && e.to != node {
            out.push((k, e.to, false));
        } else if e.to == node && e.from != node {
            out.push((k, e.from, true));
        }
    }
    out
}

fn new_item(id: &str, line: usize) -> Item {
    Item {
        id: id.to_string(),
        label: id.to_string(),
        sub: String::new(),
        kind: 0,
        status: 0,
        group: false,
        parent: None,
        children: Vec::new(),
        line,
        implicit: false,
        icon: String::new(),
        vendor: false,
        members: Vec::new(),
        zoom: Zoom::default(),
    }
}

/// Copies the look of parent item `p` to context item `it`.
fn copy_look(it: &mut Item, p: &Item, pd: &Diagram, dtype: DiagramType) {
    it.label = p.label.clone();
    it.sub = p.sub.clone();
    let kind = pd.dtype.kinds().get(p.kind).copied().unwrap_or("");
    it.kind = dtype.kinds().iter().position(|&k| k == kind).unwrap_or(0);
    it.icon = p.icon.clone();
    it.zoom.context = true;
    it.implicit = false;
}

/// Adds the context of the parent level: the `^name` nodes get the look of
/// the parent node. In a graph, the parts go in a frame group, and each
/// parent edge with no `^name` goes from a context node to the frame.
fn add_context(d: &mut Diagram, pd: &Diagram, node: usize, errs: &mut Vec<String>) {
    let near = neighbours(pd, node);
    let zoomed = &pd.items[node];
    let mut ctx: Vec<(usize, usize)> = Vec::new(); // (item here, item in parent)
    for i in 0..d.items.len() {
        let Some(name) = d.items[i].id.strip_prefix('^') else { continue };
        let line = d.items[i].line;
        if !d.items[i].implicit {
            errs.push(format!(
                "{line}: '^{name}' names a node of the parent level, so it has no node line here. Use it in an edge"
            ));
            continue;
        }
        let Some(p) = pd.items.iter().position(|it| it.id == name) else {
            errs.push(format!("{line}: '^{name}': the parent level has no node '{name}'"));
            continue;
        };
        if !near.iter().any(|n| n.1 == p) {
            errs.push(format!(
                "{line}: '^{name}' has no edge to '{}' in the parent level. Context nodes: {}",
                zoomed.id,
                context_names(pd, &near)
            ));
            continue;
        }
        copy_look(&mut d.items[i], &pd.items[p], pd, d.dtype);
        // A context node stays outside the groups of the detail level.
        move_to_root(d, i);
        ctx.push((i, p));
    }
    if !errs.is_empty() || near.is_empty() || matches!(d.dtype, DiagramType::Seq | DiagramType::Timeline) {
        return;
    }
    let line = d.items.iter().map(|it| it.line).min().unwrap_or(1);
    // The frame group stands for the zoomed node.
    let frame = d.items.len();
    let mut f = new_item(FRAME, line);
    f.group = true;
    f.label = zoomed.label.clone();
    f.icon = zoomed.icon.clone();
    d.items.push(f);
    let parts: Vec<usize> = d.roots.iter().copied().filter(|&r| !d.items[r].zoom.context).collect();
    for &r in &parts {
        d.items[r].parent = Some(frame);
    }
    d.items[frame].children = parts;
    for &(k, other, incoming) in &near {
        let c = match ctx.iter().find(|c| c.1 == other) {
            Some(c) => c.0,
            None => {
                let i = d.items.len();
                let mut it = new_item(&format!("^{}", pd.items[other].id), line);
                copy_look(&mut it, &pd.items[other], pd, d.dtype);
                d.items.push(it);
                ctx.push((i, other));
                i
            }
        };
        let pe = &pd.edges[k];
        let covered =
            d.edges.iter().any(|e| (incoming || pe.bidir) && e.from == c || (!incoming || pe.bidir) && e.to == c);
        if covered {
            // A body edge with no label takes the label of the parent edge.
            for e in
                d.edges.iter_mut().filter(|e| e.label.is_empty() && (if incoming { e.from == c } else { e.to == c }))
            {
                e.label = pe.label.clone();
            }
            continue;
        }
        let arrows = d.dtype.arrows();
        let arrow = arrows
            .iter()
            .copied()
            .find(|&a| a == pe.arrow)
            .or_else(|| arrows.iter().copied().find(|&a| a == "->"))
            .unwrap_or(arrows[0]);
        let (from, to) = if incoming { (c, frame) } else { (frame, c) };
        d.edges.push(Edge {
            from,
            to,
            label: pe.label.clone(),
            dashed: pe.dashed && arrow == pe.arrow,
            bidir: pe.bidir && arrow == pe.arrow,
            main: pe.main && arrow == pe.arrow,
            status: pe.status,
            line,
            arrow,
        });
    }
    // The incoming context goes before the frame and the outgoing after it.
    let mut roots: Vec<usize> = Vec::new();
    for &(c, other) in &ctx {
        if near.iter().any(|n| n.1 == other && n.2) && !roots.contains(&c) {
            roots.push(c);
        }
    }
    roots.push(frame);
    for &(c, _) in &ctx {
        if !roots.contains(&c) {
            roots.push(c);
        }
    }
    d.roots = roots;
}

fn context_names(pd: &Diagram, near: &[(usize, usize, bool)]) -> String {
    let mut names: Vec<String> = Vec::new();
    for n in near {
        let name = format!("^{}", pd.items[n.1].id);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(" ")
    }
}

fn move_to_root(d: &mut Diagram, i: usize) {
    if let Some(p) = d.items[i].parent.take() {
        d.items[p].children.retain(|&c| c != i);
        d.roots.push(i);
    }
}

/// The status of a node from the parts of its detail level. See the table
/// in docs/design/zoom.md.
pub fn rolled_status(d: &Diagram) -> usize {
    let parts = d.items.iter().filter(|it| !it.group && !it.zoom.context);
    let (mut none, mut done, mut wip, mut plan, mut change) = (false, false, false, false, false);
    for it in parts {
        match it.status {
            0 => none = true,
            1 => done = true,
            2 => wip = true,
            3 => plan = true,
            _ => change = true,
        }
    }
    if change {
        5
    } else if !(done || wip || plan) {
        0
    } else if done && !(none || wip || plan) {
        1
    } else if plan && !(done || wip) {
        3
    } else {
        2
    }
}

/// A node with no status takes the status of its detail level. The levels
/// are in depth-first order, so a level comes after its parent.
fn rollup(levels: &mut [Level]) {
    for k in (1..levels.len()).rev() {
        let Some((p, node)) = levels[k].parent else { continue };
        if levels[p].d.items[node].status == 0 {
            levels[p].d.items[node].status = rolled_status(&levels[k].d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(src: &str) -> Vec<Level> {
        load(src, None).unwrap_or_else(|e| panic!("{e}"))
    }

    fn errors(src: &str) -> Vec<String> {
        match load(src, None) {
            Ok(_) => panic!("the source must give an error"),
            Err(e) => e.errors,
        }
    }

    const CART: &str = "arch \"Checkout\"
gw \"API gateway\"
cart \"Cart service\" \"go\"
db \"Postgres\" :store

gw => cart \"add\"
cart -> db

detail cart
  handler \"HTTP handler\" done
  svc \"Cart logic\" wip
  ^gw => handler
  handler => svc
";

    #[test]
    fn a_detail_block_makes_a_level() {
        let l = levels(CART);
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].key(), "cart");
        assert_eq!(l[1].d.title, "Cart service");
        assert_eq!(l[1].d.dtype, DiagramType::Arch);
        assert!(l[0].d.items[1].zoom.detail);
        // The block lines are not part of the top level.
        assert_eq!(l[0].d.items.len(), 3);
        // Line numbers stay the lines of the file.
        let svc = l[1].d.items.iter().find(|it| it.id == "svc").unwrap();
        assert_eq!(svc.line, 11);
    }

    #[test]
    fn the_context_comes_from_the_parent_level() {
        let l = levels(CART);
        let d = &l[1].d;
        let gw = d.items.iter().find(|it| it.id == "^gw").unwrap();
        assert_eq!(gw.label, "API gateway");
        assert!(gw.zoom.context && gw.parent.is_none());
        // The parent edge to db has no ^db, so it goes to the frame.
        let frame = d.items.iter().position(|it| it.id == FRAME).unwrap();
        let db = d.items.iter().position(|it| it.id == "^db").unwrap();
        assert!(d.edges.iter().any(|e| e.from == frame && e.to == db));
        assert_eq!(d.items[db].kind, 1);
        // The body edge from ^gw takes the label of the parent edge.
        assert_eq!(d.edges[0].label, "add");
        let root_ids: Vec<&str> = d.roots.iter().map(|&r| d.items[r].id.as_str()).collect();
        assert_eq!(root_ids, ["^gw", "^", "^db"]);
    }

    #[test]
    fn the_status_rolls_up() {
        let l = levels(CART);
        assert_eq!(l[0].d.items[1].status, 2);
        let mk = |a: &str, b: &str| {
            let src = format!("plan\nx\ndetail x\n  a {a}\n  b {b}\n");
            levels(&src)[0].d.items[0].status
        };
        assert_eq!(mk("done", "done"), 1);
        assert_eq!(mk("done", ""), 2);
        assert_eq!(mk("plan", "plan"), 3);
        assert_eq!(mk("plan", ""), 3);
        assert_eq!(mk("done", "add"), 5);
        assert_eq!(mk("", ""), 0);
        let own = levels("plan\nx done\ndetail x\n  a plan\n");
        assert_eq!(own[0].d.items[0].status, 1);
    }

    #[test]
    fn nested_blocks_and_a_type_word() {
        let src = "arch
a -> b
detail a \"A parts\"
  p -> q
  detail q class \"Q types\"
    T --|> U
";
        let l = levels(src);
        let keys: Vec<String> = l.iter().map(Level::key).collect();
        assert_eq!(keys, ["", "a", "a/q"]);
        assert_eq!(l[1].d.title, "A parts");
        assert_eq!(l[2].d.dtype, DiagramType::Class);
        assert_eq!(l[2].d.title, "Q types");
    }

    #[test]
    fn wrong_blocks_give_clear_errors() {
        assert_eq!(errors("arch\na\ndetail z\n  b\n"), ["3: detail 'z': this level has no node 'z'"]);
        assert_eq!(errors("arch\na\ndetail a\nb\n"), ["3: the detail block of 'a' has no body. Indent its lines"]);
        assert_eq!(errors("arch\na\ndetail a\n  b\ndetail a\n  c\n"), ["5: 'a' already has a detail block"]);
        assert_eq!(
            errors("arch\na -> b\n^a -> b\n")[0],
            "3: '^a' names a node of the parent level, but the top level has no parent"
        );
        assert_eq!(
            errors("arch\na -> b\nc\ndetail a\n  ^c -> x\n"),
            ["5: '^c' has no edge to 'a' in the parent level. Context nodes: ^b"]
        );
        assert_eq!(errors("arch\na\ndetail a\n  ^q -> x\n"), ["4: '^q': the parent level has no node 'q'"]);
        assert_eq!(errors("arch\na\ndetail a foo\n  x\n"), ["3: unknown type 'foo' in the detail line. Types: arch, seq, plan, deps, flow, state, dataflow, change, erd, class, timeline, journey"]);
        assert_eq!(
            errors("arch\na\ndetail a\n  b\n  detail b\n    c\n    detail c\n      d\n      detail d\n        e\n"),
            ["9: too many levels. A diagram has 4 levels at most"]
        );
        assert_eq!(
            errors("arch\na\ndetail a\n    b\n  c\n"),
            ["5: the line has less indent than the first line of the detail block"]
        );
    }

    #[test]
    fn a_file_link_reads_the_file() {
        let read = |p: &Path| -> std::io::Result<String> {
            match p.to_str() {
                Some("dir/cart.sd") => Ok("arch \"Cart parts\"\nh -> s\n".to_string()),
                Some("dir/loop.sd") => Ok("arch\nx >top.sd\n".to_string()),
                Some("dir/top.sd") => Ok("arch\nc >loop.sd\n".to_string()),
                Some("dir/bad.sd") => Ok("arch\nh h\n".to_string()),
                _ => Err(std::io::Error::other("no file")),
            }
        };
        let top = Path::new("dir/top.sd");
        let l = load_with("arch\ncart >cart.sd\n", Some(top), &read).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].d.title, "Cart parts");
        let err = load_with("arch\nc >loop.sd\n", Some(top), &read).unwrap_err();
        assert_eq!(err.errors, ["2: in loop.sd:2: the file link 'top.sd' makes a cycle of files"]);
        let err = load_with("arch\nc >bad.sd\nd >none.sd\n", Some(top), &read).unwrap_err();
        assert_eq!(
            err.errors,
            [
                "2: in bad.sd:2: unknown word 'h'. Statuses: done wip plan add mod del",
                "3: cannot read the linked file 'none.sd'"
            ]
        );
    }
    #[test]
    fn a_protected_member_is_not_a_comment() {
        let src =
            "class\nrepo\n  # conn Pool\n  - cache Map\ndetail repo class\n  q\n    # run(sql) Rows\n    + go()\n";
        let l = levels(src);
        assert_eq!(l[0].d.items[0].members.len(), 2);
        assert_eq!(l[1].d.items[0].members.len(), 2);
    }
}
