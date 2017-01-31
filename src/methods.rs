
use std::fmt::{self, Debug, Formatter};

use {HashMethod, HashFn};


#[cfg(target_pointer_width = "64")]
const K: u8 = 64;

#[cfg(target_pointer_width = "32")]
const K: u8 = 32;

pub struct SubLow;

struct SubLowFn {
    bias: usize,
    max: usize,
}

impl HashFn for SubLowFn {
    fn hash(&self, v: usize) -> usize {
        v - self.bias
    }

    fn max(&self) -> usize {
        self.max
    }
}

impl Debug for SubLowFn {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "x - {}", self.bias)
    }
}

impl HashMethod for SubLow {
    fn new(&self, cases: &[usize]) -> Box<HashFn> {
        assert!(!cases.is_empty());
        let bias = cases.iter().cloned().min().unwrap();
        let max = cases.into_iter()
            .map(|&v| {
                SubLowFn {
                        bias: bias,
                        max: 0,
                    }
                    .hash(v)
            })
            .max()
            .unwrap() + 1;

        Box::new(SubLowFn {
            bias: bias,
            max: max,
        })
    }
}

pub struct ClzSub;

struct ClzSubFn {
    bias: usize,
    max: usize,
}

impl HashFn for ClzSubFn {
    fn hash(&self, v: usize) -> usize {
        (v.leading_zeros() as usize) - self.bias
    }

    fn max(&self) -> usize {
        self.max
    }
}

impl Debug for ClzSubFn {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "clz(x) - {}", self.bias)
    }
}

impl HashMethod for ClzSub {
    fn new(&self, case: &[usize]) -> Box<HashFn> {
        assert!(!case.is_empty());
        let bias =
            case.iter().cloned().map(|v| ClzSubFn { bias: 0, max: 0 }.hash(v)).min().unwrap();
        let max = case.into_iter()
            .map(|&v| {
                ClzSubFn {
                        bias: bias,
                        max: 0,
                    }
                    .hash(v)
            })
            .max()
            .unwrap() + 1;
        Box::new(ClzSubFn {
            bias: bias,
            max: max,
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
        let cardinality = self.mapped_cardinality(cases);

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
                if w.mapped_cardinality(cases) > w_max.mapped_cardinality(cases) {
                    w_max = w;
                }
            }
        }

        w_max
    }

    fn mapped_cardinality(&self, cases: &[usize]) -> usize {
        let mut set: Vec<_> = cases.into_iter().map(|&s| self.hash(s)).collect();
        set.sort();
        set.dedup();
        set.len()
    }
}

impl HashFn for Window {
    fn hash(&self, input: usize) -> usize {
        let width = 1 + self.l - self.r;
        let mask = (1 << width) - 1;
        (input >> self.r) & mask
    }

    fn max(&self) -> usize {
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
