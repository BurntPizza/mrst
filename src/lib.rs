
use std::fmt::Debug;

pub mod methods;

pub trait HashFn: Debug {
    fn hash(&self, input: usize) -> usize;
    fn max(&self) -> usize;
}

pub trait HashMethod {
    fn new(&self, case_set: &[usize]) -> Box<HashFn>;
}

#[derive(Debug)]
pub enum Marker<T> {
    Case(usize, T),
    Default,
}

#[derive(Debug)]
pub enum Tree<T> {
    Branch {
        children: Vec<Tree<T>>,
        hash_fn: Box<HashFn>,
    },
    Leaf(Marker<T>),
}

impl<T: Clone> Tree<T> {
    fn new(cases: &[usize], data: &[T], methods: &[&HashMethod]) -> Self {
        assert!(!methods.is_empty());
        assert_eq!(cases.len(),
                   {
                       let mut sorted = cases.to_vec();
                       sorted.sort();
                       sorted.dedup();
                       sorted.len()
                   },
                   "Duplicate cases");

        Tree::new_helper(0, cases.len(), cases, data, methods).unwrap()
    }

    fn new_helper(depth: usize,
                  max_depth: usize,
                  cases: &[usize],
                  data: &[T],
                  methods: &[&HashMethod])
                  -> Option<Self> {
        if depth > max_depth {
            return None;
        }

        if cases.is_empty() {
            return Some(Tree::Leaf(Marker::Default));
        }

        if cases.len() == 1 {
            return Some(Tree::Leaf(Marker::Case(cases[0], data[0].clone())));
        }

        let mut best = None;

        for m in methods {
            let f = m.new(cases);
            let max = f.max();

            if max > cases.len() {
                continue;
            }

            let children: Vec<_> = (0..max)
                .into_iter()
                .map(|slot| {
                    let (cases, data): (Vec<_>, Vec<_>) = cases.iter()
                        .cloned()
                        .zip(data.iter().cloned())
                        .filter(|&(case, _)| f.hash(case) == slot)
                        .unzip();

                    Tree::new_helper(depth + 1, max_depth, &*cases, &*data, methods)
                })
                .collect();

            if children.iter().any(Option::is_none) {
                continue;
            }

            let children = children.into_iter().map(Option::unwrap).collect();
            let tree = Tree::branch(children, f);

            if best.is_none() || best.as_ref().map(Tree::depth).unwrap() > tree.depth() {
                best = Some(tree);
            }
        }

        best
    }

    fn branch(children: Vec<Tree<T>>, hash_fn: Box<HashFn>) -> Self {
        Tree::Branch {
            children: children,
            hash_fn: hash_fn,
        }
    }

    fn depth(&self) -> usize {
        match *self {
            Tree::Leaf(..) => 1,
            Tree::Branch { ref children, .. } => {
                1 + children.into_iter().map(Tree::depth).max().unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate itertools;
    use self::itertools::*;

    use std::io::prelude::*;
    use std::fs::File;
    use std::fmt::Display;

    use super::{HashFn, Tree, Marker};
    use super::methods::*;

    #[test]
    fn test_val() {
        assert_eq!(Window { l: 5, r: 3 }.hash(41), 5);
    }

    #[ignore]
    #[test]
    fn it_works() {
        let set = vec![// (0, "F"),
                       // (1, "f0"),
                       // (2, "f1"),
                       // (4, "f2"),
                       // (8, "f3"),
                       // (16, "f4"),
                       // (32, "f5"),
                       // (64, "f6"),
                       // (128, "f7"),
                       // (256, "f8"),
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
                       (2082, "function 4")];

        let (cases, data): (Vec<_>, Vec<_>) = set.into_iter().unzip();
        let tree = Tree::new(&*cases, &*data, &[&SubLow, &ClzSub, &ShiftMask]);
        let mut f = File::create("test_graph.graphviz").unwrap();

        f.write_all(debug_print_tree(&tree).as_bytes()).unwrap();
    }

    fn debug_print_tree<T: Display>(tree: &Tree<T>) -> String {
        fn id<T>(tree: &Tree<T>) -> String {
            format!("{}", tree as *const Tree<T> as usize)
        }

        fn helper<T: Display>(tree: &Tree<T>) -> String {
            match *tree {
                Tree::Branch { ref children, ref hash_fn } => {
                    format!("{} [ label = \"{:?}\" ]\n{}",
                            id(tree),
                            hash_fn,
                            children.iter()
                                .map(|c| format!("{} -> {}\n{}", id(tree), id(c), helper(c)))
                                .join("\n"))
                }
                Tree::Leaf(ref m) => {
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
