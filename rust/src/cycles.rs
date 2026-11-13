//! Dependency cycles. Two items are in one cycle when each item can reach
//! the other one along the edges.

use crate::model::Diagram;

pub struct Cycles {
    /// True for an edge with both ends in one cycle.
    pub edge: Vec<bool>,
    /// The count of cycles: strongly connected components with two or more items.
    pub count: usize,
}

pub fn find_cycles(d: &Diagram) -> Cycles {
    let n = d.items.len();
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in &d.edges {
        next[e.from].push(e.to);
    }
    // reach[a][b]: b is reachable from a. The diagrams are small, so a
    // search from each item is fast enough.
    let mut reach = vec![vec![false; n]; n];
    let mut stack = Vec::new();
    for (a, row) in reach.iter_mut().enumerate() {
        stack.push(a);
        while let Some(u) = stack.pop() {
            for &v in &next[u] {
                if !row[v] {
                    row[v] = true;
                    stack.push(v);
                }
            }
        }
    }
    // The component of an item is the lowest index in its cycle.
    let mut comp: Vec<Option<usize>> = vec![None; n];
    let mut count = 0;
    for a in 0..n {
        if comp[a].is_some() || !reach[a][a] {
            continue;
        }
        let mut size = 0;
        for b in a..n {
            if reach[a][b] && reach[b][a] {
                comp[b] = Some(a);
                size += 1;
            }
        }
        if size > 1 {
            count += 1;
        }
    }
    let edge = d.edges.iter().map(|e| e.from != e.to && comp[e.from].is_some() && comp[e.from] == comp[e.to]).collect();
    Cycles { edge, count }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn finds_each_cycle_once() {
        let d = parse("deps\na -> b -> c -> a\nc -> d\nd -> e -> d\ne -> f\n").unwrap();
        let c = find_cycles(&d);
        assert_eq!(c.count, 2);
        assert_eq!(c.edge, [true, true, true, false, true, true, false]);
    }

    #[test]
    fn a_tree_has_no_cycles() {
        let d = parse("deps\na -> b\na -> c\nb -> c\n").unwrap();
        let c = find_cycles(&d);
        assert_eq!(c.count, 0);
        assert!(c.edge.iter().all(|&x| !x));
    }
}
