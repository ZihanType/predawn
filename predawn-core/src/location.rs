use std::{fmt, ops::Deref, panic};

use snafu::GenerateImplicitData;

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location(&'static panic::Location<'static>);

impl Location {
    #[track_caller]
    #[inline]
    pub const fn caller() -> Self {
        Self(panic::Location::caller())
    }

    #[inline]
    pub const fn file(&self) -> &'static str {
        self.0.file()
    }

    #[inline]
    pub const fn line(&self) -> u32 {
        self.0.line()
    }

    #[inline]
    pub const fn column(&self) -> u32 {
        self.0.column()
    }
}

impl fmt::Display for Location {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<&'static panic::Location<'static>> for Location {
    #[inline]
    fn from(location: &'static panic::Location<'static>) -> Self {
        Self(location)
    }
}

impl From<Location> for &'static panic::Location<'static> {
    #[inline]
    fn from(Location(location): Location) -> &'static panic::Location<'static> {
        location
    }
}

impl Deref for Location {
    type Target = panic::Location<'static>;

    #[inline]
    fn deref(&self) -> &'static Self::Target {
        self.0
    }
}

impl GenerateImplicitData for Location {
    #[track_caller]
    #[inline]
    fn generate() -> Self {
        Self::caller()
    }
}
