
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

impl<T> Tree<T> {
    fn branch(children: Vec<Tree<T>>, hash_fn: Box<HashFn>) -> Self {
        Tree::Branch {
            children: children,
            hash_fn: hash_fn,
        }
    }
}

pub struct Log2;

impl HashMethod for Log2 {
    fn new(&self, case_set: &[usize]) -> Box<HashFn> {

        #[derive(Debug, Default)]
        struct Log2Fn(usize);

        impl HashFn for Log2Fn {
            fn hash(&self, v: usize) -> usize {
                v.trailing_zeros() as usize
            }

            fn size(&self) -> usize {
                self.0
            }
        }

        let max_size = 1 + case_set.into_iter().map(|&v| Log2Fn::default().hash(v)).max().unwrap();
        Box::new(Log2Fn(max_size))
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

pub fn gen_tree<T: Clone + Debug>(cases: &[usize], data: &[T], methods: &[&HashMethod]) -> Tree<T> {
    if cases.is_empty() {
        return Tree::Leaf(Marker::Default);
    }

    if cases.len() == 1 {
        return Tree::Leaf(Marker::Case(cases[0], data[0].clone()));
    }

    let hash_fn = methods.into_iter()
        .map(|m| m.new(cases))
        .max_by_key(|hf| mapped_cardinality(cases, |v| hf.hash(v)))
        .unwrap_or_else(|| panic!("Must pass at least one hash function"));

    let size = hash_fn.size();
    let mut children = Vec::with_capacity(size);

    for child_slot in 0..size {
        let (cases, data): (Vec<_>, Vec<_>) = cases.iter()
            .cloned()
            .zip(data.iter().cloned())
            .filter(|&(case, _)| hash_fn.hash(case) == child_slot)
            .unzip();

        children.push(gen_tree(&*cases, &*data, methods));
    }

    Tree::branch(children, hash_fn)
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

    use super::{HashFn, ShiftMask, Log2, gen_tree, Window, Tree, Marker};

    #[test]
    fn test_val() {
        assert_eq!(Window { l: 5, r: 3 }.hash(41), 5);
    }

    #[test]
    fn it_works() {
        let set = vec![// (1, "f0"),
                       // (2, "f1"),
                       // (4, "f2"),
                       // (8, "f3"),
                       // (16, "f4"),
                       // (32, "f5"),
                       // (64, "f6"),
                       // (128, "f7"),
                       // (256, "f8")
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
        let tree = gen_tree(&*cases, &*data, &[&ShiftMask, &Log2]);

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
