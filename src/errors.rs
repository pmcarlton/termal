// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Thomas Junier

use std::io;

#[derive(Debug)]
pub enum TermalError {
    Io(io::Error),
    Format(String),
}

// These allow conversion to TermalError, required for main() to return Result<()> and for '?' to
// work.

impl From<io::Error> for TermalError {
    fn from(e: io::Error) -> Self {
        TermalError::Io(e)
    }
}

impl From<String> for TermalError {
    fn from(s: String) -> Self {
        TermalError::Format(s)
    }
}
