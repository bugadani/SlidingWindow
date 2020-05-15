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

use generic_array::{GenericArray, ArrayLength, sequence::GenericSequence};
use wrapping::WrappingExt as _;
use core::mem::MaybeUninit;

pub trait Size<I>: ArrayLength<MaybeUninit<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<MaybeUninit<I>> {}

/// A sliding window.
///
/// Sliding windows are queues that overwrite their oldest data when full.
pub struct SlidingWindow<IT, N>
    where
        N: Size<IT> {
    items: GenericArray<MaybeUninit<IT>, N>,
    write_idx: usize,
    is_full: bool
}

impl<IT, N> Default for SlidingWindow<IT, N>
    where
        N: Size<IT> {

    fn default() -> Self {
        Self {
            items: GenericArray::generate(|_| MaybeUninit::uninit()),
            write_idx: 0,
            is_full: false
        }
    }
}

impl<IT, N> core::ops::Index<usize> for SlidingWindow<IT, N>
    where
        N: Size<IT> {
    type Output = IT;
    fn index(&self, idx: usize) -> &Self::Output {
        let read_from = if self.is_full {
            self.write_idx.wrapping_add_limited(idx, N::USIZE)
        } else {
            assert!(idx < self.write_idx, "Trying to access uninitialized memory");
            idx
        };

        unsafe { &*self.items[read_from].as_ptr() }
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
            let read_from = self.start.wrapping_add_limited(self.offset, N::USIZE);
            self.offset += 1;

            Some(unsafe { &*self.window.items[read_from].as_ptr() })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.offset;
        (remaining, Some(remaining))
    }
}

impl<'a, IT, N> ExactSizeIterator for Iter<'a, IT, N>
    where N:
        Size<IT> {
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        debug_assert_eq!(upper, Some(lower));
        lower
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

            Some(unsafe { &*self.window.items[self.offset].as_ptr() })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.offset;
        (remaining, Some(remaining))
    }
}

impl<'a, IT, N> ExactSizeIterator for UnorderedIter<'a, IT, N>
    where N:
        Size<IT> {
    fn len(&self) -> usize {
        let (lower, upper) = self.size_hint();
        debug_assert_eq!(upper, Some(lower));
        lower
    }
}

impl<IT, N> SlidingWindow<IT, N>
    where
        N: Size<IT> {

    /// Returns an empty sliding window object.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an element into the window.
    ///
    /// If the window is full, this method will remove and return the oldest element.
    pub fn insert(&mut self, t: IT) -> Option<IT> {
        let new: MaybeUninit<IT> = MaybeUninit::new(t);

        if !self.is_full {
            self.items[self.write_idx] = new;
            if self.write_idx == N::USIZE - 1 {
                self.write_idx = 0;
                self.is_full = true;
            } else {
                self.write_idx += 1;
            }
            None
        } else {
            let old = core::mem::replace(&mut self.items[self.write_idx], new);
            self.write_idx = self.write_idx.wrapping_add1_limited(N::USIZE);

            Some(unsafe { old.assume_init() })
        }
    }

    /// Removes all elements from the window.
    pub fn clear(&mut self) {
        let count = self.count();
        for elem in &mut self.items[0..count] {
            unsafe { core::ptr::drop_in_place(elem.as_mut_ptr()); }
        }

        *self = Self::new();
    }

    /// Returns `true` if the window is full.
    pub fn is_full(&self) -> bool {
        self.is_full
    }

    /// Returns the number of elements stored in the window.
    pub fn count(&self) -> usize {
        if self.is_full {
            N::USIZE
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

        let mut ordered = sw.iter();
        let mut unordered = sw.iter_unordered();

        assert_eq!(4, ordered.len());
        assert_eq!(4, unordered.len());

        ordered.next();
        ordered.next();

        unordered.next();
        unordered.next();

        assert_eq!(2, ordered.len());
        assert_eq!(2, unordered.len());
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

    #[test]
    #[should_panic(expected = "Trying to access uninitialized memory")]
    fn index_to_uninited() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);

        sw[3];
    }
}