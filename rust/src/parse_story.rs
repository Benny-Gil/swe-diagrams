//! The `story` blocks of the source. A story line starts a block, and each
//! line with more indent is one step, or steps joined with `&`.

use std::collections::HashMap;

use crate::model::{Diagram, DiagramType, StepDef, StoryDef};
use crate::parse::Tok;

const STEP_FORM: &str = "write a step as: a -> b \"caption\", and join steps at the same time with &";

/// One step as the source writes it. The ids resolve after the last line,
/// because a node line can come after the story.
struct RawStep {
    from: String,
    to: String,
    hollow: bool,
    caption: String,
}

struct RawStory {
    title: String,
    line: usize,
    /// The step lines. `None` is a line with a format error.
    beats: Vec<(usize, Option<Vec<RawStep>>)>,
}

#[derive(Default)]
pub(crate) struct StoryLines {
    stories: Vec<RawStory>,
    /// The indent of the open story line.
    open: Option<i64>,
}

impl StoryLines {
    /// Takes the line when it is a story line or a step line. A line
    /// without more indent than the story line closes the story.
    pub(crate) fn take(&mut self, toks: &[Tok], indent: i64, ln: usize) -> bool {
        if let Some(base) = self.open {
            if indent > base {
                let steps = step_line(toks);
                if let Some(s) = self.stories.last_mut() {
                    s.beats.push((ln, steps));
                }
                return true;
            }
            self.open = None;
        }
        if toks.len() == 2 && toks[0].word() == Some("story") && toks[1].is_str() {
            self.stories.push(RawStory { title: toks[1].text().to_string(), line: ln, beats: Vec::new() });
            self.open = Some(indent);
            return true;
        }
        false
    }

    /// Finds the edge of each step and puts the stories in the diagram.
    pub(crate) fn resolve(self, d: &mut Diagram, by_id: &HashMap<&str, usize>, errs: &mut Vec<String>) {
        for raw in self.stories {
            let ln = raw.line;
            if d.dtype == DiagramType::Timeline {
                errs.push(format!("{ln}: a timeline diagram has no stories, because it has no edges"));
                continue;
            }
            if let Some(first) = d.stories.iter().find(|s| s.title == raw.title) {
                errs.push(format!("{ln}: the story \"{}\" is already on line {}", raw.title, first.line));
                continue;
            }
            if raw.beats.is_empty() {
                errs.push(format!(
                    "{ln}: the story \"{}\" has no steps. Indent the steps under the story line",
                    raw.title
                ));
                continue;
            }
            let mut story = StoryDef { title: raw.title, line: ln, beats: Vec::new() };
            // The edge of the step before. A pair of nodes with more than one
            // edge, as in a seq diagram, takes the next edge after it.
            let mut cursor: Option<usize> = None;
            for (sl, steps) in raw.beats {
                let Some(steps) = steps else {
                    errs.push(format!("{sl}: {STEP_FORM}"));
                    continue;
                };
                let mut beat = Vec::new();
                for s in steps {
                    let ends = [&s.from, &s.to].map(|id| (id, by_id.get(id.as_str()).copied()));
                    if let Some((id, _)) = ends.iter().find(|(_, i)| i.is_none()) {
                        errs.push(format!("{sl}: the step names '{id}', which is not a node of the diagram"));
                        continue;
                    }
                    let (a, b) = (ends[0].1.unwrap_or_default(), ends[1].1.unwrap_or_default());
                    match find_edge(d, a, b, cursor) {
                        Some((edge, back)) => {
                            cursor = Some(edge);
                            beat.push(StepDef { edge, back, hollow: s.hollow, caption: s.caption, line: sl });
                        }
                        None => errs.push(format!(
                            "{sl}: no edge joins '{}' and '{}'. A step must follow an edge of the diagram",
                            s.from, s.to
                        )),
                    }
                }
                story.beats.push(beat);
            }
            d.stories.push(story);
        }
    }
}

/// The steps of one line, or `None` for a format error.
fn step_line(toks: &[Tok]) -> Option<Vec<RawStep>> {
    let mut out = Vec::new();
    for part in toks.split(|t| t.word() == Some("&")) {
        let (ids, caption) = match part {
            [a, w, b] => ([a, w, b], String::new()),
            [a, w, b, c] if c.is_str() => ([a, w, b], c.text().to_string()),
            _ => return None,
        };
        let from = ids[0].word()?;
        let to = ids[2].word()?;
        let hollow = match ids[1].word()? {
            "->" => false,
            "-->" => true,
            _ => return None,
        };
        out.push(RawStep { from: from.to_string(), to: to.to_string(), hollow, caption });
    }
    Some(out)
}

/// The edge between items `a` and `b`, and true when the step goes against
/// its direction. An edge in the direction of the step wins, then the first
/// edge after `cursor`, then the first edge in line order.
fn find_edge(d: &Diagram, a: usize, b: usize, cursor: Option<usize>) -> Option<(usize, bool)> {
    let joins = |k: usize| {
        let e = &d.edges[k];
        (e.from == a && e.to == b) || (e.from == b && e.to == a)
    };
    let along = |k: usize| {
        let e = &d.edges[k];
        (e.from == a && e.to == b) || e.bidir
    };
    let after = |k: usize| cursor.is_none_or(|c| k > c);
    let all: Vec<usize> = (0..d.edges.len()).filter(|&k| joins(k)).collect();
    let pick = all
        .iter()
        .copied()
        .find(|&k| along(k) && after(k))
        .or_else(|| all.iter().copied().find(|&k| along(k)))
        .or_else(|| all.iter().copied().find(|&k| after(k)))
        .or_else(|| all.first().copied())?;
    Some((pick, d.edges[pick].from != a))
}

#[cfg(test)]
mod tests {
    use crate::parse::parse;

    #[test]
    fn a_step_finds_its_edge_in_either_direction() {
        let d = parse(
            "arch
a -> b
b -> c \"x\"
story \"S\"
  a -> b \"go\"
  c --> b & b --> a
",
        )
        .unwrap();
        let s = &d.stories[0];
        assert_eq!(s.title, "S");
        assert_eq!(s.beats.len(), 2);
        let b0 = &s.beats[0][0];
        assert_eq!((b0.edge, b0.back, b0.hollow, b0.caption.as_str()), (0, false, false, "go"));
        let b1: Vec<(usize, bool, bool)> = s.beats[1].iter().map(|x| (x.edge, x.back, x.hollow)).collect();
        assert_eq!(b1, [(1, true, true), (0, true, true)]);
    }

    #[test]
    fn a_seq_step_takes_the_next_message() {
        let d = parse(
            "seq
a -> b \"one\"
b --> a
a -> b \"two\"
story \"S\"
  a -> b
  b --> a
  a -> b
",
        )
        .unwrap();
        let edges: Vec<usize> = d.stories[0].beats.iter().map(|b| b[0].edge).collect();
        assert_eq!(edges, [0, 1, 2]);
    }

    #[test]
    fn a_story_closes_at_a_line_with_less_indent() {
        let d = parse("arch\na -> b\nstory \"S\"\n  a -> b\nc\n").unwrap();
        assert_eq!(d.items.len(), 3);
        assert_eq!(d.stories[0].beats.len(), 1);
    }

    #[test]
    fn bad_steps_give_errors() {
        let err = parse(
            "arch
a -> b
c
story \"S\"
  a -> c
  a -> zz
  a => b
story \"S\"
  a -> b
story \"T\"
",
        )
        .unwrap_err();
        assert_eq!(
            err.errors,
            [
                "5: no edge joins 'a' and 'c'. A step must follow an edge of the diagram",
                "6: the step names 'zz', which is not a node of the diagram",
                "7: write a step as: a -> b \"caption\", and join steps at the same time with &",
                "8: the story \"S\" is already on line 4",
                "10: the story \"T\" has no steps. Indent the steps under the story line",
            ]
        );
    }
}
