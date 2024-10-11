use std::{fmt, panic};

use macro_v::macro_v;
use snafu::GenerateImplicitData;

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    file: &'static str,
    line: u32,
    column: u32,
}

impl Location {
    #[doc(hidden)]
    #[inline]
    pub const fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }

    #[track_caller]
    #[inline]
    pub const fn caller() -> Self {
        Self::from_std(panic::Location::caller())
    }

    #[inline]
    pub const fn file(&self) -> &'static str {
        self.file
    }

    #[inline]
    pub const fn line(&self) -> u32 {
        self.line
    }

    #[inline]
    pub const fn column(&self) -> u32 {
        self.column
    }

    #[inline]
    pub const fn from_std(location: &'static panic::Location<'_>) -> Self {
        Self {
            file: location.file(),
            line: location.line(),
            column: location.column(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

impl<'a> From<&'static panic::Location<'a>> for Location {
    #[inline]
    fn from(location: &'static panic::Location<'a>) -> Self {
        Self::from_std(location)
    }
}

/// Constructs a `Location` that is unaffected by `#[track_caller]`
#[macro_v(pub)]
macro_rules! location {
    () => {
        $crate::location::Location::new(file!(), line!(), column!())
    };
}

impl GenerateImplicitData for Location {
    #[track_caller]
    #[inline]
    fn generate() -> Self {
        Self::caller()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_of_track_caller_in_location() {
        let Tuple {
            from_std: loc_from_std_by_fn,
            from_crate: loc_from_crate_by_fn,
        } = location_by_fn();

        assert_eq!(loc_from_std_by_fn, loc_from_crate_by_fn);

        let Tuple {
            from_std: loc_from_std_by_macro,
            from_crate: loc_from_crate_by_macro,
        } = location_by_macro();

        assert_ne!(loc_from_std_by_macro, loc_from_crate_by_macro);

        assert_ne!(loc_from_std_by_fn, loc_from_std_by_macro);
        assert_ne!(loc_from_std_by_fn, loc_from_crate_by_macro);
    }

    struct Tuple {
        from_std: Location,
        from_crate: Location,
    }

    #[track_caller]
    fn location_by_fn() -> Tuple {
        let from_std = Location::from_std(std::panic::Location::caller());
        let from_crate = Location::caller();

        Tuple {
            from_std,
            from_crate,
        }
    }

    #[track_caller]
    fn location_by_macro() -> Tuple {
        let from_std = Location {
            file: file!(),
            line: line!(),
            column: column!(),
        };

        let from_crate = super::location!();

        Tuple {
            from_std,
            from_crate,
        }
    }
}
