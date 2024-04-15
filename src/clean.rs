use std::error::Error;

// Based on https://github.com/shepmaster/snafu
// Copyright (c) 2019- Jake Goulding
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

/// Provides the cleaned_errors method on errors, which returns
/// an iterator that removes duplicated error messages from the error
/// and its sources.
pub trait CleanedErrors {
    fn cleaned_errors<'a, 'b, 'c>(&'a self) -> CleanedErrorText<'b, 'c>
    where
        'a: 'b;
}

impl<T: Error + 'static> CleanedErrors for T {
    fn cleaned_errors<'a, 'b, 'c>(&'a self) -> CleanedErrorText<'b, 'c>
    where
        'a: 'b,
    {
        CleanedErrorText::new(self)
    }
}

/// An iterator that removes duplicated error messages from errors.
///
/// Some errors return both a source and include the source message in
/// their own error message. This causes duplication with reporters that
/// print the whole chain.
///
/// This iterator checks if the error message contains the source
/// error's message as a suffix and removes it.
pub struct CleanedErrorText<'a, 'b>(Option<CleanedErrorTextStep<'a, 'b>>);

struct CleanedErrorTextStep<'a, 'b> {
    error: &'a (dyn Error + 'b),
    /// error text extracted by the previous item and saved here to avoid calling to_string twice
    error_text: String,
}

impl<'a, 'b> CleanedErrorTextStep<'a, 'b> {
    fn new(err: &'a (dyn Error + 'b)) -> Self {
        Self {
            error: err,
            error_text: err.to_string(),
        }
    }
}

impl<'a, 'b> CleanedErrorText<'a, 'b> {
    pub fn new(err: &'a (dyn Error + 'b)) -> Self {
        Self(Some(CleanedErrorTextStep::new(err)))
    }
}

impl<'a, 'b> Iterator for CleanedErrorText<'a, 'b> {
    /// The original error, the cleaned display string, and whether it has been cleaned
    type Item = (&'a (dyn Error + 'b), String, bool);

    fn next(&mut self) -> Option<Self::Item> {
        let step = self.0.take()?;
        let error_text = step.error_text;
        let err = step.error;

        match err.source() {
            Some(source) => {
                let source_text = source.to_string();
                let (cleaned_text, cleaned) = error_text
                    .strip_suffix(&source_text)
                    .map(|text| {
                        let text = text.trim_end();
                        (text.strip_suffix(':').unwrap_or(text).to_owned(), true)
                    })
                    .unwrap_or_else(|| (error_text, false));

                self.0 = Some(CleanedErrorTextStep {
                    error: source,
                    error_text: source_text,
                });
                Some((err, cleaned_text, cleaned))
            }
            None => Some((err, error_text, false)),
        }
    }
}
