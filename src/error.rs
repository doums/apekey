// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IoError;

#[derive(Debug, Clone)]
pub struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl StdError for Error {}

impl Error {
    pub fn new(item: impl Into<String>) -> Error {
        Error(item.into())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Error(error)
    }
}

impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error(error.to_string())
    }
}

impl From<nom::error::Error<&str>> for Error {
    fn from(error: nom::error::Error<&str>) -> Self {
        Error(error.to_string())
    }
}
