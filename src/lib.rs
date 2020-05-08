//! This crate provides a heapless, fixed size sliding window.
//!
//! Sliding windows are used to hold the N most recent samples of a data stream.
//!
//! # Example
//!
//! ```rust
//! use sliding_window::*;
//! use sliding_window::typenum::consts::*;
//!
//! // Create a SlidingWindow with a window size of 4 elements
//! let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();
//!
//! // Insert some data
//! sw.insert(1);
//! sw.insert(2);
//! sw.insert(3);
//! sw.insert(4);
//!
//! // The 0 index always returns the oldest element in the window
//! assert_eq!(1, sw[0]);
//!
//! // When full, inserting a new element removes and returns the oldest
//! assert_eq!(Some(1), sw.insert(5));
//! ```
#![cfg_attr(not(test), no_std)]

pub use generic_array::typenum;

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

/// A sliding window.
///
/// Sliding windows are queues that overwrite their oldest data when full.
#[derive(Default)]
pub struct SlidingWindow<IT, N>
    where
        N: Size<IT> {
    items: GenericArray<Option<IT>, N>,
    write_idx: usize
}

impl<IT, N> core::ops::Index<usize> for SlidingWindow<IT, N>
    where
        N: Size<IT> {
    type Output = IT;
    fn index(&self, idx: usize) -> &Self::Output {
        let read_from = if self.is_full() {
            self.write_idx.wrapping_add_limited(idx, N::to_usize())
        } else {
            idx % N::to_usize()
        };

        self.items[read_from].as_ref().unwrap()
    }
}

/// Read-only iterator that returns elements in the order of insertion.
pub struct Iter<'a, IT, N>
    where
        N: Size<IT> {
    window: &'a SlidingWindow<IT, N>,
    start: usize,
    offset: usize,
    count: usize
}

impl<'a, IT, N> Iterator for Iter<'a, IT, N>
    where N:
        Size<IT> {
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

/// Read-only iterator that does not respect the order of insertion.
pub struct UnorderedIter<'a, IT, N>
    where
        N: Size<IT> {
    window: &'a SlidingWindow<IT, N>,
    offset: usize
}

impl<'a, IT, N> Iterator for UnorderedIter<'a, IT, N>
    where
        N: Size<IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset > 0 {
            self.offset -= 1;

            self.window.items[self.offset].as_ref()
        } else {
            None
        }
    }
}

impl<IT, N> SlidingWindow<IT, N>
    where
        N: Size<IT> {

    /// Returns an empty sliding window object.
    pub fn new() -> Self {
        Self {
            items: GenericArray::default(),
            write_idx: 0
        }
    }

    /// Insert an element into the window.
    ///
    /// If the window is full, this method will remove and return the oldest element.
    pub fn insert(&mut self, t: IT) -> Option<IT> {
        let old = self.items[self.write_idx].replace(t);
        self.write_idx = self.write_idx.wrapping_add1_limited(N::to_usize());

        old
    }

    /// Removes all elements from the window.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Returns `true` if the window is full.
    pub fn is_full(&self) -> bool {
        match self.items[self.write_idx] {
            Some(_) => true,
            None => false
        }
    }

    /// Returns the number of elements stored in the window.
    pub fn count(&self) -> usize {
        if self.is_full() {
            N::to_usize()
        } else {
            self.write_idx
        }
    }

    /// Returns an iterator to read from the window.
    ///
    /// The iterator starts at the oldest element and ends with the newest.
    pub fn iter(&self) -> Iter<IT, N> {
        Iter {
            window: self,
            start: if self.is_full() { self.write_idx } else { 0 },
            offset: 0,
            count: self.count()
        }
    }

    /// Returns an iterator to read from the window.
    ///
    /// This iterator starts at the beginning of the internal array instead of the oldest element
    /// so it does not return the elements in the order of insertion.
    pub fn iter_unordered(&self) -> UnorderedIter<IT, N> {
        UnorderedIter {
            window: self,
            offset: self.count()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::typenum::consts::*;

    #[test]
    fn basics() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);

        assert_eq!(1, sw[0]);

        assert_eq!(3, sw.count());
        assert_eq!(false, sw.is_full());

        assert_eq!(None, sw.insert(4));

        assert_eq!(1, sw[0]);
        assert_eq!(4, sw.count());
        assert_eq!(true, sw.is_full());

        assert_eq!(Some(1), sw.insert(5));

        assert_eq!(2, sw[0]);
        assert_eq!(4, sw.count());
        assert_eq!(true, sw.is_full());

        sw.clear();

        assert_eq!(0, sw.count());
        assert_eq!(false, sw.is_full());
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

        assert_eq!(18, sw.iter_unordered().sum());
    }
}