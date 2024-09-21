use std::fmt::Display;

use crate::location::Location;

#[derive(Debug, Default)]
pub struct ErrorStack(Vec<Box<str>>);

impl ErrorStack {
    pub fn push<T: Display>(&mut self, error: &T, location: &Location) {
        let i = self.0.len();

        self.0
            .push(format!("{i}: {error}, at {location}").into_boxed_str());
    }

    pub fn push_without_location<T: Display>(&mut self, error: &T) {
        let i = self.0.len();

        self.0.push(format!("{i}: {error}").into_boxed_str());
    }

    pub fn finish(self) -> Box<[Box<str>]> {
        self.0.into_boxed_slice()
    }
}
