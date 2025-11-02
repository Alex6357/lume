// src/error.rs
use crate::span::Span;

#[derive(Debug)]
pub enum LumeError {
    Lexical { msg: String, span: Span },
    Syntax { msg: String, span: Span },
    TypeError { msg: String, span: Span },
    OwnershipError { msg: String, span: Span },
    RuntimeError { msg: String, span: Span },
}

impl std::fmt::Display for LumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LumeError::Lexical { msg, .. } => write!(f, "Lexical error: {}", msg),
            LumeError::Syntax { msg, .. } => write!(f, "Syntax error: {}", msg),
            LumeError::TypeError { msg, .. } => write!(f, "Type error: {}", msg),
            LumeError::OwnershipError { msg, .. } => write!(f, "Ownership error: {}", msg),
            LumeError::RuntimeError { msg, .. } => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for LumeError {}
