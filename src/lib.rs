
// TODO: NOT READY FOR PUBLISHING AT ALL


// From "Efficient Multiway Radix Search Trees"
// https://drhanson.s3.amazonaws.com/storage/documents/mrst.pdf

extern crate itertools;

use std::collections::HashSet;

const K: u8 = 64;
type Int = u64;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Marker {
    Case(Int, String),
    Default,
}

enum Tree {
    Node {
        l: usize,
        children: Vec<Tree>,
        w: Window,
    },
    Leaf(usize, Marker),
}

impl Tree {
    fn new(children: Vec<Tree>, w: Window) -> Self {
        Tree::Node {
            l: new_label(),
            children: children,
            w: w,
        }
    }

    fn leaf(m: Marker) -> Self {
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
    let set: HashSet<_> = c.into_iter().map(|&s| val(s, w)).collect();

    set.len() > thresh
}

fn mapped_cardinality<F>(c: &[Int], f: F) -> usize
    where F: Fn(Int) -> Int
{
    c.into_iter().map(|&s| f(s)).collect::<HashSet<_>>().len()
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

fn mrst(p: HashSet<(Int, Marker)>) -> Tree {
    if p.len() == 1 {
        return Tree::leaf(p.into_iter().next().unwrap().1);
    }

    let c = p.iter().map(|p| p.0).collect::<Vec<_>>();
    let w_max = critical_window(&*c);
    let n = w_max.l - w_max.r + 1;
    let nj = 1 << n;
    let mut children = Vec::with_capacity(nj);

    for j in 0..nj {
        let pj: HashSet<_> = p.iter()
                              .cloned()
                              .filter(|&(c, _)| val(c, w_max) as usize == j)
                              .collect();
        if pj.is_empty() {
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

    use super::{Int, Marker, Tree, Window, val, mrst};

    #[test]
    fn test_val() {
        assert_eq!(val(41, Window { l: 5, r: 3 }), 5);
    }

    #[test]
    fn it_works() {
        use std::io::prelude::*;
        use std::fs::File;

        let set = vec![
            case(8, "function 1"),
            case(16, "function 1"),
            case(33, "function 1"),
            case(37, "function 1"),
            case(41, "function 1"),
            case(60, "function 1"),

            case(144, "function 2"),
            case(264, "function 2"),
            case(291, "function 2"),

            case(1032, "function 3"),

            case(2048, "function 4"),
            case(2082, "function 4"),
        ]
                      .into_iter()
                      .collect();

        let tree = mrst(set);

        let mut f = File::create("test_graph.graphviz").unwrap();
        f.write_all(debug_print_tree(&tree).as_bytes()).unwrap();
    }

    fn case(i: Int, label: &str) -> (Int, Marker) {
        (i, Marker::Case(i, label.to_owned()))
    }

    fn debug_print_tree(tree: &Tree) -> String {
        fn id(tree: &Tree) -> String {
            match *tree {
                Tree::Leaf(id, _) => format!("N{}", id),
                Tree::Node { l, .. } => format!("N{}", l),
            }
        }

        #[allow(unused_variables)]
        fn helper(tree: &Tree) -> String {
            match *tree {
                Tree::Node { l, ref children, w } => {
                    let edges = children.iter()
                                        .map(|c| {
                                            format!("{} -> {}\n{}", id(tree), id(c), helper(c))
                                        })
                                        .join("\n");
                    format!("{} [ label = \"{}\" ]\n{}",
                            id(tree),
                            if w.l == w.r {
                                format!("bit {}", w.l)
                            } else {
                                format!("bits {} to {}", w.l, w.r)
                            },
                            edges)
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
