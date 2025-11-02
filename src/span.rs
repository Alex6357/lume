// src/span.rs
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file: String,
}

impl Span {
    pub fn new(start: usize, end: usize, file: impl Into<String>) -> Self {
        Self {
            start,
            end,
            file: file.into(),
        }
    }
}
