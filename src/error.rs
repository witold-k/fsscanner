// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use threadpool::ThreadPoolError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ThreadPool(ThreadPoolError),
    Processing(Vec<String>),
    Internal(&'static str),
    Callback(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::ThreadPool(error) => write!(f, "thread pool error: {error}"),
            Self::Processing(errors) => {
                write!(f, "{} processing error(s)", errors.len())?;
                for error in errors {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            }
            Self::Internal(message) => write!(f, "internal error: {message}"),
            Self::Callback(message) => write!(f, "callback error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self { Self::Io(error) }
}

impl From<ThreadPoolError> for Error {
    fn from(error: ThreadPoolError) -> Self { Self::ThreadPool(error) }
}
