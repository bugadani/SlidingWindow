SlidingWindow [![crates.io](https://img.shields.io/crates/v/sliding_window.svg)](https://crates.io/crates/sliding_window)
=============

Sliding windows are used to hold the N most recent samples of a data stream.

[Documentation](https://docs.rs/sliding_window/0.1.0/sliding_window/)

Example
-------

```rust
use sliding_window::*;
use sliding_window::typenum::consts::*;

// Create a SlidingWindow with a window size of 4 elements
let mut sw: SlidingWindow<_, U4> = SlidingWindow::new();

// Insert some data
sw.insert(1);
sw.insert(2);
sw.insert(3);
sw.insert(4);

// The 0 index always returns the oldest element in the window
assert_eq!(1, sw[0]);

// When full, inserting a new element removes and returns the oldest
assert_eq!(Some(1), sw.insert(5));
```