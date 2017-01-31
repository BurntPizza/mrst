
// TODO: NOT READY FOR PUBLISHING AT ALL



use std::fmt::{self, Debug, Formatter};

#[cfg(target_pointer_width = "64")]
const K: u8 = 64;

#[cfg(target_pointer_width = "32")]
const K: u8 = 32;

pub trait HashFn: Debug {
    fn hash(&self, input: usize) -> usize;
    fn size(&self) -> usize;
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

        let mut best: Option<Tree<T>> = None;

        for m in methods {
            let f = m.new(cases);
            let size = f.size();
            let children: Vec<_> = (0..size)
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

pub struct ClzSub;

struct ClzSubFn {
    bias: usize,
    size: usize,
}

impl HashFn for ClzSubFn {
    fn hash(&self, v: usize) -> usize {
        (v.leading_zeros() as usize) - self.bias
    }

    fn size(&self) -> usize {
        self.size
    }
}

impl Debug for ClzSubFn {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "clz(x) - {}", self.bias)
    }
}

impl HashMethod for ClzSub {
    fn new(&self, case_set: &[usize]) -> Box<HashFn> {
        assert!(!case_set.is_empty());
        let bias =
            case_set.iter().cloned().map(|v| ClzSubFn { bias: 0, size: 0 }.hash(v)).min().unwrap();
        let max_size = case_set.into_iter()
            .map(|&v| {
                ClzSubFn {
                        bias: bias,
                        size: 0,
                    }
                    .hash(v)
            })
            .max()
            .unwrap() + 1;
        Box::new(ClzSubFn {
            bias: bias,
            size: max_size,
        })
    }
}

/// From "Efficient Multiway Radix Search Trees"
/// https://drhanson.s3.amazonaws.com/storage/documents/mrst.pdf
pub struct ShiftMask;

impl HashMethod for ShiftMask {
    fn new(&self, case_set: &[usize]) -> Box<HashFn> {
        Box::new(Window::critical_window(case_set))
    }
}

#[derive(Copy, Clone)]
pub struct Window {
    pub l: u8,
    pub r: u8,
}

impl Window {
    pub fn is_critical(&self, cases: &[usize]) -> bool {
        let thresh = 1 << (self.l - self.r);
        let cardinality = mapped_cardinality(cases, |v| self.hash(v));

        cardinality > thresh
    }

    /// Find the longest critical window, and the most critical window of that length.
    pub fn critical_window(cases: &[usize]) -> Window {
        let mut w = Window {
            l: K - 1,
            r: K - 1,
        };
        let mut w_max = w;
        for b in (0..K).rev() {
            w.r = b;
            if w.is_critical(cases) {
                w_max = w;
            } else {
                w.l -= 1;
                if mapped_cardinality(cases, |v| w.hash(v)) >
                   mapped_cardinality(cases, |v| w_max.hash(v)) {
                    w_max = w;
                }
            }
        }

        w_max
    }
}

impl HashFn for Window {
    fn hash(&self, input: usize) -> usize {
        let width = 1 + self.l - self.r;
        let mask = (1 << width) - 1;
        (input >> self.r) & mask
    }

    fn size(&self) -> usize {
        1 << (1 + (self.l - self.r) as usize)
    }
}

impl Debug for Window {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // format!("shr {}\nand {}", w.r, 1 + w.l - w.r)
        if self.l == self.r {
            write!(f, "bit {}", self.l)
        } else {
            write!(f, "bits {} to {}", self.l, self.r)
        }
    }
}

// The 'score' of hash function `f` on dataset `c`.
// The score is higher the fewer collisions `f` produces on `c`.
fn mapped_cardinality<F>(cases: &[usize], f: F) -> usize
    where F: Fn(usize) -> usize
{
    let mut set: Vec<_> = cases.into_iter().map(|&s| f(s)).collect();
    set.sort();
    set.dedup();
    set.len()
}

// TODO: look for hash functions with few collisions
// use additional node types for this: e.g. "Node" -> "ShiftMask" or something
// and then "SubLow", "Log2", and "JumpTable"

#[cfg(test)]
mod tests {
    extern crate itertools;
    use self::itertools::*;

    use std::io::prelude::*;
    use std::fs::File;
    use std::fmt::Display;

    use super::{HashFn, ShiftMask, ClzSub, Window, Tree, Marker};

    #[test]
    fn test_val() {
        assert_eq!(Window { l: 5, r: 3 }.hash(41), 5);
    }

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
        use super::HashMethod;
        let f = ClzSub.new(&*cases);
        println!("8: {}", f.hash(8));
        println!("264: {}", f.hash(264));

        let tree = Tree::new(&*cases, &*data, &[&ShiftMask, &ClzSub]);

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
