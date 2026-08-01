// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! A fast, lock-free thread pool built on top of crossbeam channels.
//!
//! This module provides a simple task-scheduling [`ThreadPool`] that distributes closures
//! across a fixed set of persistent worker threads. It utilizes crossbeam's unbounded
//! channels to maximize throughput during high-frequency parallel processing spikes.

use crossbeam_channel::{unbounded, Sender};
use std::thread;

/// A thread pool that manages a persistent set of worker threads.
///
/// Tasks are submitted via the [`execute`](ThreadPool::execute) method and processed
/// concurrently using a lock-free work-distribution channel.
pub struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: Option<Sender<Box<dyn FnOnce() + Send + 'static>>>,
}

impl ThreadPool {
    /// Creates a new `ThreadPool` spawning the specified number of worker threads.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - The exact number of worker OS threads to spawn into the pool.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # // Use fsscanner crate space context for doc test resolution
    /// use fsscanner::threadpool::ThreadPool;
    ///
    /// let pool = ThreadPool::new(4);
    /// ```
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        let receiver = std::sync::Arc::new(receiver);

        let mut workers = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let rx = receiver.clone();
            workers.push(thread::spawn(move || {
                while let Ok(job) = rx.recv() {
                    job();
                }
            }));
        }

        Self { workers, sender: Some(sender) }
    }

    /// Submits a closure task to the thread pool for execution.
    ///
    /// The task is allocated on the heap and safely shipped to the next available
    /// idle worker thread within the lock-free queue channel.
    ///
    /// # Arguments
    ///
    /// * `job` - A thread-safe closure implementing `FnOnce() + Send + 'static`.
    ///
    /// # Panics
    ///
    /// Panics if the internal channel is disconnected or uninitialized.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fsscanner::threadpool::ThreadPool;
    /// use std::sync::{Arc, Mutex};
    ///
    /// let pool = ThreadPool::new(2);
    /// let counter = Arc::new(Mutex::new(0));
    ///
    /// let counter_clone = Arc::clone(&counter);
    /// pool.execute(move || {
    ///     let mut guard = counter_clone.lock().unwrap();
    ///     *guard += 1;
    /// });
    /// ```
    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .as_ref()
            .expect("ThreadPool channel was prematurely dropped")
            .send(Box::new(job))
            .expect("Failed to send job to worker pool thread channel");
    }
}

impl Drop for ThreadPool {
    /// Dispatches teardown hooks across all active worker threads.
    ///
    /// Dropping the `ThreadPool` closes the internal sender channel. Worker threads
    /// break out of their processing loops once all previously submitted tasks are finished.
    /// This method blocks the dropping thread until every worker thread has been joined.
    fn drop(&mut self) {
        // Drop the sender so workers stop when queue empties
        self.sender.take();

        // Join all workers safely, ignoring potential single-worker panic poison values
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

