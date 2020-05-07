//! This crate provides a heapless sliding window. Sliding windows are used to hold the last N samples of a data stream.
//!
#![cfg_attr(not(test), no_std)]

pub use generic_array::typenum;
pub use generic_array::typenum::consts;

pub trait Producer {
    type Item;

    fn clear(&mut self);
    fn insert(&mut self, t: Self::Item) -> Option<Self::Item>;
}

pub trait Reader {
    type Item;
    type WindowSize: Size<Self::Item>;

    fn full(&self) -> bool;
    fn count(&self) -> usize;
    fn iter(&self) -> Iter<Self::Item, Self::WindowSize>;
    fn iter_unordered(&self) -> UnorderedIter<Self::Item, Self::WindowSize>;
}

mod wrapping {
    pub trait WrappingExt {
        type Rhs;
        type Output;
        fn wrapping_add_limited(self, r: Self::Rhs, max: Self::Rhs) -> Self::Output;
        fn wrapping_add1_limited(self, max: Self::Rhs) -> Self::Output;
    }

    impl WrappingExt for usize {
        type Rhs = Self;
        type Output = Self;
        fn wrapping_add_limited(self, r: Self::Rhs, max: Self::Rhs) -> Self::Output {
            match self.checked_add(r) {
                Some(v) => v % max,
                None => (r - (usize::MAX - self)) % max
            }
        }

        fn wrapping_add1_limited(self, max: Self::Rhs) -> Self::Output {
            if self == max - 1 { 0 } else { self + 1 }
        }
    }

    #[cfg(test)]
    mod test {
        use super::WrappingExt;

        #[test]
        pub fn sanity_check() {
            let vector: &[(usize, usize, usize, usize)] = &[
                (5, 1, 10, 6),
                (5, 5, 10, 0),
                (5, 6, 10, 1),
                (5, 16, 10, 1),
                (usize::MAX, usize::MAX, usize::MAX, 0),
                (usize::MAX, 1, usize::MAX, 1),
                (usize::MAX - 1, 2, usize::MAX, 1)
            ];

            for &(lhs, rhs, limit, expectation) in vector.iter() {
                assert_eq!(expectation, lhs.wrapping_add_limited(rhs, limit), "({} + {}) mod {} == {}", lhs, rhs, limit, expectation);
            }
        }

        #[test]
        pub fn sanity_check_increment() {
            let vector: &[(usize, usize, usize)] = &[
                (5, 10, 6),
                (9, 10, 0)
            ];

            for &(lhs, limit, expectation) in vector.iter() {
                assert_eq!(lhs.wrapping_add_limited(1, limit), lhs.wrapping_add1_limited(limit), "({} + 1) mod {} == {}", lhs, limit, expectation);
            }
        }
    }
}

use generic_array::{GenericArray, ArrayLength};
use wrapping::WrappingExt as _;
pub trait Size<I>: ArrayLength<Option<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<Option<I>> {}

pub struct SlidingWindow<IT, N>
    where N: Size<IT> {
    items: GenericArray<Option<IT>, N>,
    write_idx: usize
}

impl<IT, N> core::ops::Index<usize> for SlidingWindow<IT, N>
    where
        N: Size<IT> {
    type Output = IT;
    fn index(&self, idx: usize) -> &Self::Output {
        let read_from = if self.full() {
            self.write_idx.wrapping_add_limited(idx, N::to_usize())
        } else {
            idx % N::to_usize()
        };

        self.items[read_from].as_ref().unwrap()
    }
}

pub struct Iter<'a, IT, N>
    where N: Size<IT> {
    window: &'a SlidingWindow<IT, N>,
    start: usize,
    offset: usize,
    count: usize
}

impl<'a, IT, N> Iterator for Iter<'a, IT, N>
    where N: Size<IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < self.count {
            let read_from = self.start.wrapping_add_limited(self.offset, N::to_usize());
            self.offset += 1;

            self.window.items[read_from].as_ref()
        } else {
            None
        }
    }
}

pub struct UnorderedIter<'a, IT, N>
    where N: Size<IT> {
    window: &'a SlidingWindow<IT, N>,
    offset: usize,
    count: usize
}

impl<'a, IT, N> Iterator for UnorderedIter<'a, IT, N>
    where N: Size<IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < self.count {
            let read_from = self.offset;
            self.offset += 1;

            self.window.items[read_from].as_ref()
        } else {
            None
        }
    }
}

impl<IT, N> SlidingWindow<IT, N>
    where N: Size<IT> {
    pub fn new() -> Self {
        Self {
            items: GenericArray::default(),
            write_idx: 0
        }
    }
}

impl<IT, N> Producer for SlidingWindow<IT, N>
    where N: Size<IT> {
    type Item = IT;

    fn insert(&mut self, t: Self::Item) -> Option<Self::Item> {
        let old = self.items[self.write_idx].replace(t);
        self.write_idx = self.write_idx.wrapping_add1_limited(N::to_usize());

        old
    }

    fn clear(&mut self) {
        self.write_idx = 0;
        for i in 0..N::to_usize() {
            self.items[i] = None;
        }
    }
}

impl<IT, N> Reader for SlidingWindow<IT, N>
    where N: Size<IT> {
    type Item = IT;
    type WindowSize = N;

    fn full(&self) -> bool {
        match self.items[self.write_idx] {
            Some(_) => true,
            None => false
        }
    }

    fn count(&self) -> usize {
        if self.full() {
            N::to_usize()
        } else {
            self.write_idx
        }
    }

    fn iter(&self) -> Iter<Self::Item, Self::WindowSize> {
        Iter {
            window: self,
            start: if self.full() { self.write_idx } else { 0 },
            offset: 0,
            count: self.count()
        }
    }

    fn iter_unordered(&self) -> UnorderedIter<Self::Item, Self::WindowSize> {
        UnorderedIter {
            window: self,
            offset: 0,
            count: self.count()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::consts::*;

    #[test]
    fn basics() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);

        assert_eq!(1, sw[0]);

        assert_eq!(3, sw.count());
        assert_eq!(false, sw.full());

        assert_eq!(None, sw.insert(4));

        assert_eq!(1, sw[0]);
        assert_eq!(4, sw.count());
        assert_eq!(true, sw.full());

        assert_eq!(Some(1), sw.insert(5));

        assert_eq!(2, sw[0]);
        assert_eq!(4, sw.count());
        assert_eq!(true, sw.full());

        sw.clear();

        assert_eq!(0, sw.count());
        assert_eq!(false, sw.full());
    }

    #[test]
    fn iter() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);
        sw.insert(4);
        sw.insert(5);
        sw.insert(6);

        assert_eq!(&3, sw.iter().next().unwrap()); // first element is the oldest
        assert_eq!(18, sw.iter().sum());
    }

    #[test]
    fn unordered_iter() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);
        sw.insert(4);
        sw.insert(5);
        sw.insert(6);

        assert_eq!(&5, sw.iter_unordered().next().unwrap()); // first element is not the oldest
        assert_eq!(18, sw.iter_unordered().sum());
    }
}