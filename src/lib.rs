mod clean;

pub use clean::{CleanedErrorText, CleanedErrors};

use core::fmt;
use std::error::Error;

/// Provides the `report` method for `std::error::Error`,
/// converting the error to a `Report`.
pub trait Reportable {
    fn report(self) -> Report<Self>
    where
        Self: std::error::Error,
        Self: std::marker::Sized;
}

impl<E: Error> Reportable for E {
    fn report(self) -> Report<Self>
    where
        Self: std::error::Error,
        Self: std::marker::Sized,
    {
        Report::new(self)
    }
}

/// AsRefError is needed because `anyhow::Error` only implements `AsRef<dyn Error>`, not `Error`,
/// but `&dyn Error` does not implement `AsRef<dyn Error>` because `AsRef` doesn't have a blanket
/// implementation (https://doc.rust-lang.org/std/convert/trait.AsRef.html#reflexivity).
pub trait AsRefError {
    fn as_ref_error(&self) -> &dyn Error;
}

impl<E: Error> AsRefError for E {
    fn as_ref_error(&self) -> &dyn Error {
        self
    }
}

// This implementation is unfortunately not possible.
/*
impl<E: AsRef<dyn Error>> AsRefError for E {
    fn as_ref_error(&self) -> &dyn Error {
        self.as_ref()
    }
}
*/

/// Report prints an error and all its sources.
///
/// Source messages will be cleaned using `CleanedErrors` to remove duplication
/// from errors that include their source's message in their own message.
///
/// The debug implementation prints each error on a separate line, while the display
/// implementation prints all errors on one line separated by a colon.
/// Using alternate formatting (`{:#}`) is identical to the debug implementation.
///
/// The debug implementation is intended for cases where errors are debug printed,
/// for example returning an error from main or using `expect` on `Result`:
///
/// ```should_panic
/// use error_report::Report;
///
/// fn func1() -> Result<(), std::io::Error> {
///     Err(std::io::Error::other("oh no!"))
/// }
///
/// fn main() -> Result<(), Report<impl std::error::Error>> {
///     func1()?;
///     Ok(())
/// }
/// ```
///
/// ```should_panic
/// # use error_report::Report;
/// let i: i8 = 256.try_into().map_err(Report::from).expect("conversion error");
/// ```
pub struct Report<E: AsRefError>(E);

impl<E: AsRefError> From<E> for Report<E> {
    fn from(value: E) -> Self {
        Self(value)
    }
}

impl<E: AsRefError> Report<E> {
    /// Construct a new `Report` from an error.
    pub fn new(err: E) -> Self {
        Self::from(err)
    }

    fn format(&self, f: &mut fmt::Formatter<'_>, multiline: bool) -> fmt::Result {
        let cleaned_texts = CleanedErrorText::new(self.0.as_ref_error())
            .filter(|(_, t, _)| !t.is_empty())
            .enumerate();

        if !multiline {
            for (i, (_, text, _)) in cleaned_texts {
                if i > 0 {
                    write!(f, ": ")?;
                }
                write!(f, "{text}")?;
            }
        } else {
            for (i, (_, text, _)) in cleaned_texts {
                if i == 0 {
                    write!(f, "{text}")?;
                } else {
                    if i == 1 {
                        write!(f, "\n\nCaused by:\n")?;
                    }
                    writeln!(f, "    {i}. {text}")?;
                }
            }
        }

        Ok(())
    }
}

impl<E: AsRefError> fmt::Debug for Report<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, true)
    }
}

impl<E: AsRefError> fmt::Display for Report<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, f.alternate())
    }
}

/// Ref holds an error type that is `AsRef<dyn Error>`, allowing it
/// to be used with `Report` without having to implement `AsRefError` on it.
pub struct Ref<E>(E);

impl<E: AsRef<dyn Error>> AsRefError for Ref<E> {
    fn as_ref_error(&self) -> &dyn Error {
        self.0.as_ref()
    }
}

impl<E: AsRef<dyn Error>> Report<Ref<E>> {
    /// Construct a new `Report` from a type that implements `AsRef<dyn Error>`.
    pub fn from_ref(value: E) -> Self {
        Self::new(Ref(value))
    }
}

impl<E: AsRef<dyn Error>> From<E> for Report<Ref<E>> {
    fn from(value: E) -> Self {
        Self::from_ref(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn test_conversion() {
        fn anyhow_fn() -> anyhow::Result<()> {
            Err(anyhow::anyhow!("oh no!"))
        }

        fn anyhow_caller() -> Result<(), super::Report<impl AsRefError>> {
            anyhow_fn().context("fn failed")?;

            Ok(())
        }

        #[derive(Debug)]
        struct CustomError();

        impl std::fmt::Display for CustomError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "custom error")
            }
        }

        impl std::error::Error for CustomError {}

        fn custom_fn() -> Result<(), CustomError> {
            Err(CustomError())
        }

        fn custom_caller() -> Result<(), super::Report<CustomError>> {
            custom_fn()?;

            Ok(())
        }

        let err = anyhow_caller().expect_err("function did not return error");

        let normal_string = format!("{}", err);
        let alt_string = format!("{:#}", err);
        let debug_string = format!("{:?}", err);

        assert_eq!(normal_string, "fn failed: oh no!");
        assert_eq!(alt_string, "fn failed\n\nCaused by:\n    1. oh no!\n");
        assert_eq!(debug_string, "fn failed\n\nCaused by:\n    1. oh no!\n");

        let err = custom_caller().expect_err("function did not return error");

        let normal_string = format!("{}", err);
        let alt_string = format!("{:#}", err);
        let debug_string = format!("{:?}", err);

        assert_eq!(normal_string, "custom error");
        assert_eq!(alt_string, "custom error");
        assert_eq!(debug_string, "custom error");

        let err = custom_fn().expect_err("function did not return error");
        let report = Report::from(&err);
        let normal_string = format!("{}", report);
        let alt_string = format!("{:#}", report);
        let debug_string = format!("{:?}", report);

        assert_eq!(normal_string, "custom error");
        assert_eq!(alt_string, "custom error");
        assert_eq!(debug_string, "custom error");
        _ = err
    }
}
