// src/lexer/token.rs
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Let,
    Mut,
    Func,
    If,
    Else,
    Match,
    Case,
    On,
    Own,
    Throws,
    Recover,
    Return,
    Import,
    Export,

    // Literals
    Int(i64),
    Bool(bool),
    Str(String),

    // Identifiers
    Ident(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,

    // Delimiters
    /// (
    LParenthesis,
    /// )
    RParenthesis,
    /// {
    LBrace,
    /// }
    RBrace,
    /// [
    LBracket,
    /// ]
    RBracket,
    /// ,
    Comma,
    /// ;
    Semicolon,
    /// .
    Dot,
    /// :
    Colon,
    /// ->
    Arrow,

    // Special
    Question, // ?
    FatArrow, // =>
    Eof,
}

pub fn keyword_or_ident(ident: &str) -> Token {
    match ident {
        "let" => Token::Let,
        "mut" => Token::Mut,
        "func" => Token::Func,
        "if" => Token::If,
        "else" => Token::Else,
        "match" => Token::Match,
        "case" => Token::Case,
        "on" => Token::On,
        "own" => Token::Own,
        "throws" => Token::Throws,
        "recover" => Token::Recover,
        "return" => Token::Return,
        "import" => Token::Import,
        "export" => Token::Export,
        "true" => Token::Bool(true),
        "false" => Token::Bool(false),
        _ => Token::Ident(ident.into()),
    }
}
