use std::error::Error;

use crate::location::Location;

pub enum NextError<'a> {
    Ext(&'a dyn ErrorExt),
    Std(&'a dyn Error),
    None,
}

pub trait ErrorExt: Error {
    fn entry(&self) -> (Location, NextError<'_>);

    fn error_stack(&self) -> Box<[Box<str>]> {
        let mut stack = Vec::new();

        let mut next_error = {
            let (location, next_error) = self.entry();
            stack.push(format!("0: {self}, at {location}").into_boxed_str());
            next_error
        };

        loop {
            let idx = stack.len();

            match next_error {
                NextError::Ext(e) => {
                    next_error = {
                        let (location, next_error) = e.entry();
                        stack.push(format!("{idx}: {e}, at {location}").into_boxed_str());
                        next_error
                    };
                    continue;
                }
                NextError::Std(e) => {
                    stack.push(format!("{idx}: {e}").into_boxed_str());
                    break;
                }
                NextError::None => break,
            }
        }

        stack.into_boxed_slice()
    }
}
