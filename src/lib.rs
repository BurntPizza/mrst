
// TODO: NOT READY FOR PUBLISHING AT ALL


// From "Efficient Multiway Radix Search Trees"
// https://drhanson.s3.amazonaws.com/storage/documents/mrst.pdf

extern crate itertools;
use itertools::*;

const K: u8 = 64;
type Int = u64;

struct CaseSet<'a>(Vec<Int>, Vec<&'a str>);

impl<'a> From<Vec<(Int, &'a str)>> for CaseSet<'a> {
    fn from(mut v: Vec<(Int, &'a str)>) -> Self {
        v.sort_by_key(|kv| kv.0);
        v.dedup();
        let (cases, labels) = v.into_iter().unzip();
        CaseSet(cases, labels)
    }
}

enum Marker<'a> {
    Case(Int, &'a str),
    Default,
}

enum Tree<'a> {
    Node {
        id: usize,
        children: Vec<Tree<'a>>,
        w: Window,
    },
    Leaf(usize, Marker<'a>),
}

impl<'a> Tree<'a> {
    fn new(children: Vec<Tree<'a>>, w: Window) -> Self {
        Tree::Node {
            id: new_label(),
            children: children,
            w: w,
        }
    }

    fn leaf(m: Marker<'a>) -> Self {
        Tree::Leaf(new_label(), m)
    }
}

fn new_label() -> usize {
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

    static COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;

    COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Copy, Clone)]
struct Window {
    l: u8,
    r: u8,
}

fn val(s: Int, w: Window) -> Int {
    let width = 1 + w.l - w.r;
    let mask = (1 << width) - 1;
    (s >> w.r) & mask
}

fn is_critical(w: Window, c: &[Int]) -> bool {
    let thresh = 1 << (w.l - w.r);
    let mut set = c.into_iter().map(|&s| val(s, w)).collect_vec();
    set.sort();
    set.dedup();

    set.len() > thresh
}

fn mapped_cardinality<F>(c: &[Int], f: F) -> usize
    where F: Fn(Int) -> Int
{
    let mut set = c.into_iter().map(|&s| f(s)).collect_vec();
    set.sort();
    set.dedup();
    set.len()
}

fn critical_window(c: &[Int]) -> Window {
    let mut w = Window {
        l: K - 1,
        r: K - 1,
    };
    let mut w_max = w;

    for b in (0..K).rev() {
        w.r = b;
        if is_critical(w, c) {
            w_max = w;
        } else {
            w.l -= 1;
            if mapped_cardinality(c, |s| val(s, w)) > mapped_cardinality(c, |s| val(s, w_max)) {
                w_max = w;
            }
        }
    }

    w_max
}

fn mrst<'a, I: Into<CaseSet<'a>>>(p: I) -> Tree<'a> {
    let p = p.into();
    let (cases, labels) = (p.0, p.1);

    if cases.len() == 1 {
        return Tree::leaf(Marker::Case(cases[0], labels[0]));
    }

    let w_max = critical_window(&*cases);
    let n = 1 + w_max.l - w_max.r;
    let size = 1 << n;
    let mut children = Vec::with_capacity(size);

    for j in 0..size {
        let (cases, labels) = cases.iter()
                                   .cloned()
                                   .zip(labels.iter().cloned())
                                   .filter(|&(i, _)| val(i, w_max) == j as Int)
                                   .unzip();

        let pj = CaseSet(cases, labels);

        if pj.0.is_empty() {
            children.push(Tree::leaf(Marker::Default));
        } else {
            children.push(mrst(pj));
        }
    }

    Tree::new(children, w_max)
}

#[cfg(test)]
mod tests {
    use itertools::*;

    use std::io::prelude::*;
    use std::fs::File;

    use super::{Marker, Tree, Window, val, mrst};

    #[test]
    fn test_val() {
        assert_eq!(val(41, Window { l: 5, r: 3 }), 5);
    }

    #[test]
    fn it_works() {

        let set = vec![
            (8, "function 1"),
            (16, "function 1"),
            (33, "function 1"),
            (37, "function 1"),
            (41, "function 1"),
            (60, "function 1"),

            (144, "function 2"),
            (264, "function 2"),
            (291, "function 2"),

            (1032, "function 3"),

            (2048, "function 4"),
            (2082, "function 4"),
        ];

        let tree = mrst(set);

        let mut f = File::create("test_graph.graphviz").unwrap();
        f.write_all(debug_print_tree(&tree).as_bytes()).unwrap();
    }

    fn debug_print_tree(tree: &Tree) -> String {
        fn id(tree: &Tree) -> String {
            match *tree {
                Tree::Leaf(id, _) => format!("N{}", id),
                Tree::Node { id, .. } => format!("N{}", id),
            }
        }

        #[allow(unused_variables)]
        fn helper(tree: &Tree) -> String {
            match *tree {
                Tree::Node { ref children, w, .. } => {
                    format!("{} [ label = \"{}\" ]\n{}",
                            id(tree),
                            if w.l == w.r {
                                format!("bit {}", w.l)
                            } else {
                                format!("bits {} to {}", w.l, w.r)
                            },
                            children.iter()
                                    .map(|c| format!("{} -> {}\n{}", id(tree), id(c), helper(c)))
                                    .join("\n"))
                }
                Tree::Leaf(_, ref m) => {
                    format!("{} [ label = \"{}\" ]",
                            id(tree),
                            match *m {
                                Marker::Case(case, ref name) => format!("{}?\n{}", case, name),
                                Marker::Default => format!("Default"),
                            })
                }
            }
        }

        format!("digraph G{} {{\n{}\n}}\n", id(tree), helper(tree))
    }
}
