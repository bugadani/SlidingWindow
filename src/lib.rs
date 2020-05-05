//! This crate provides a heapless sliding window. Sliding windows are used to hold the last N samples of a data stream.
//!
#![cfg_attr(not(test), no_std)]

pub use generic_array::typenum::consts;

pub trait Producer {
    type Item;

    fn clear(&mut self);
    fn insert(&mut self, t: Self::Item) -> Option<Self::Item>;
}

pub trait Reader {
    type Item;
    type WindowSize;

    fn full(&self) -> bool;
    fn count(&self) -> usize;
    fn iter(&self) -> Iter<Self::Item, Self::WindowSize> where Self::WindowSize: Size<Self::Item>;
    fn iter_unordered(&self) -> UnorderedIter<Self::Item, Self::WindowSize> where Self::WindowSize: Size<Self::Item>;
}

use generic_array::{GenericArray, ArrayLength};
pub trait Size<I>: ArrayLength<Option<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<Option<I>> {}

pub struct SlidingWindow<IT, N>
    where N: Size<IT> {
    items: GenericArray<Option<IT>, N>,
    write_idx: usize
}

pub struct Iter<'a, IT, N>
    where N: Size<IT> {
    window: &'a SlidingWindow<IT, N>,
    start: usize,
    offset: usize
}

impl<'a, IT, N> Iterator for Iter<'a, IT, N>
    where N: Size<IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < self.window.count() {
            let read_from = (self.start + self.offset) % N::to_usize();
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
    offset: usize
}

impl<'a, IT, N> Iterator for UnorderedIter<'a, IT, N>
    where N: Size<IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < self.window.count() {
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
        if self.write_idx == N::to_usize() - 1 {
            self.write_idx = 0;
        } else {
            self.write_idx += 1;
        }

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

    fn iter(&self) -> Iter<Self::Item, Self::WindowSize> where Self::WindowSize: Size<Self::Item> {
        Iter {
            window: self,
            start: if self.full() { self.write_idx } else { 0 },
            offset: 0
        }
    }

    fn iter_unordered(&self) -> UnorderedIter<Self::Item, Self::WindowSize> where Self::WindowSize: Size<Self::Item> {
        UnorderedIter {
            window: self,
            offset: 0
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::consts::*;

    #[test]
    fn count() {
        let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

        sw.insert(1);
        sw.insert(2);
        sw.insert(3);

        assert_eq!(3, sw.count());
        assert_eq!(false, sw.full());

        assert_eq!(None, sw.insert(4));

        assert_eq!(4, sw.count());
        assert_eq!(true, sw.full());

        assert_eq!(Some(1), sw.insert(5));

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