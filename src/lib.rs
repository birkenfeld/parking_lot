// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! This library provides implementations of `Mutex`, `RwLock`, `Condvar` and
//! `Once` that are smaller, faster and more flexible than those in the Rust
//! standard library. It also exposes a low-level API for creating your own
//! efficient synchronization primitives.
//!
//! # Features
//!
//! The primitives provided by this library have several advantages over those
//! in the Rust standard library:
//!
//! 1. `Mutex`, `Condvar` and `Once` only require 1 byte of storage space, and
//!    `RwLock` only requires 1 word of storage space. On the other hand the
//!    standard library primitives require a dynamically allocated `Box` to hold
//!    OS-specific synchronization primitives. The small size of `Mutex` in
//!    particular encourages the use of fine-grained locks to increase
//!    parallelism.
//! 2. Since they consist of just a single atomic variable, have constant
//!    initializers and don't need destructors, these primitives can be used as
//!     `static` global variables. The standard library primitives require
//!    dynamic initialization and thus need to be lazily initialized with
//!    `lazy_static!`.
//! 3. Uncontended lock acquisition and release is done through fast inline
//!    paths which only require a single atomic operation.
//! 4. Microcontention (a contended lock with a short critical section) is
//!    efficiently handled by spinning a few times while trying to acquire a
//!    lock.
//! 5. The locks are adaptive and will suspend a thread after a few failed spin
//!    attempts. This makes the locks suitable for both long and short critical
//!    sections.
//!
//! # The parking lot
//!
//! To keep these primitives small, all thread queuing and suspending
//! functionality is offloaded to the *parking lot*. The idea behind this is
//! based on the Webkit [`WTF::ParkingLot`]
//! (https://webkit.org/blog/6161/locking-in-webkit/) class, which essentially
//! consists of a hash table mapping of lock addresses to queues of parked
//! (sleeping) threads. The Webkit parking lot was itself inspired by Linux
//! [futexes](http://man7.org/linux/man-pages/man2/futex.2.html), but it is more
//! powerful since it allows invoking callbacks while holding a queue lock.
//!
//! *Parking* refers to suspending the thread while simultaneously enqueuing it
//! on a queue keyed by some address. *Unparking* refers to dequeuing a thread
//! from a queue keyed by some address and resuming it. The parking lot API
//! consists of just 3 functions:
//!
//! ```rust,ignore
//! unsafe fn park(key: usize,
//!                validate: &mut FnMut() -> bool,
//!                before_sleep: &mut FnMut(),
//!                timeout: Option<Instant>)
//!                -> bool
//! ```
//!
//! This function performs the following steps:
//!
//! 1. Lock the queue associated with `key`.
//! 2. Call `validate`, if it returns `false`, unlock the queue and return.
//! 3. Add the current thread to the queue.
//! 4. Unlock the queue.
//! 5. Call `before_sleep`.
//! 6. Sleep until we are unparked or `timeout` is reached.
//! 7. Return `true` if we were unparked by another thread, `false` otherwise.
//!
//! ```rust,ignore
//! unsafe fn unpark_one(key: usize,
//!                      callback: &mut FnMut(UnparkResult))
//!                      -> UnparkResult
//! ```
//!
//! This function will unpark a single thread from the queue associated with
//! `key`. The `callback` function is invoked while holding the queue lock but
//! before the thread is unparked. The `UnparkResult` indicates whether the
//! queue was empty and, if not, whether there are still threads remaining in
//! the queue.
//!
//! ```rust,ignore
//! unsafe fn unpark_all(key: usize) -> usize
//! ```
//!
//! This function will unpark all threads in the queue associated with `key`. It
//! returns the number of threads that were unparked.
//!
//! # Building custom synchronization primitives
//!
//! Building custom synchronization primitives is very simple since
//! `parking_lot` takes care of all the hard parts for you. The most commmon
//! case for a custom primitive would be to integrate a `Mutex` inside another
//! data type. Since a mutex only requires 2 bits, it can share space with other
//! data. For example, one could create an `ArcMutex` type that combines the
//! atomic reference count and the two mutex bits in the same atomic word.

#![warn(missing_docs)]
#![feature(extended_compare_and_swap, const_fn, integer_atomics)]

// Spin limit from JikesRVM & Webkit experiments
const SPIN_LIMIT: usize = 40;

mod thread_parker;
mod word_lock;
mod parking_lot;
mod raw_mutex;
mod raw_rwlock;
mod condvar;
mod mutex;
mod rwlock;
mod once;

pub use once::{Once, OnceState};
pub use parking_lot::{UnparkResult, park, unpark_one, unpark_all};
pub use mutex::{Mutex, MutexGuard};
pub use condvar::{Condvar, WaitTimeoutResult};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
