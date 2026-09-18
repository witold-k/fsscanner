// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! A small fixed-size thread pool built on crossbeam channels.

use crossbeam_channel::{unbounded, Sender};
use std::error::Error;
use std::fmt;
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadPoolError {
    NoWorkers,
    Disconnected,
    WorkerPanicked,
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWorkers => write!(f, "thread pool requires at least one worker"),
            Self::Disconnected => write!(f, "thread pool work channel disconnected"),
            Self::WorkerPanicked => write!(f, "thread pool worker panicked"),
        }
    }
}

impl Error for ThreadPoolError {}

/// A fixed-size pool of persistent worker threads.
pub struct ThreadPool {
    workers: Vec<thread::JoinHandle<()>>,
    sender: Option<Sender<Job>>,
}

impl ThreadPool {
    /// Creates a pool with exactly `num_threads` workers.
    pub fn new(num_threads: usize) -> Result<Self, ThreadPoolError> {
        if num_threads == 0 {
            return Err(ThreadPoolError::NoWorkers);
        }

        let (sender, receiver) = unbounded::<Job>();
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

        Ok(Self {
            workers,
            sender: Some(sender),
        })
    }

    /// Queues a job for execution.
    pub fn execute<F>(&self, job: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .as_ref()
            .ok_or(ThreadPoolError::Disconnected)?
            .send(Box::new(job))
            .map_err(|_| ThreadPoolError::Disconnected)
    }

    /// Waits until all queued jobs finish and reports worker panics.
    pub fn join(mut self) -> Result<(), ThreadPoolError> {
        self.sender.take();
        let mut panicked = false;

        for worker in self.workers.drain(..) {
            panicked |= worker.join().is_err();
        }

        if panicked {
            Err(ThreadPoolError::WorkerPanicked)
        } else {
            Ok(())
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
