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
    From,
    Enum,
    Class,
    With,
    Type,
    Is,

    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    PrefixedStr(String, String),
    Char(char),
    PrefixedChar(String, char),
    Bool(bool),

    // Identifiers
    Ident(String),
    Lifetime(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Eq,
    EqEq,
    Neq,
    Percent,
    PercentEq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,

    // Bitwise operators
    Amp,   // &
    Pipe,  // |
    Caret, // ^
    Tilde, // ~
    Shl,   // <<
    Shr,   // >>

    // Compound assignment
    PlusEq,  // +=
    MinusEq, // -=
    StarEq,  // *=
    SlashEq, // /=
    AmpEq,   // &=
    PipeEq,  // |=
    CaretEq, // ^=
    ShlEq,   // <<=
    ShrEq,   // >>=

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Dot,
    Colon,
    Arrow, // ->

    // Special
    Question,
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
        "from" => Token::From,
        "enum" => Token::Enum,
        "class" => Token::Class,
        "with" => Token::With,
        "type" => Token::Type,
        "is" => Token::Is,
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "true" => Token::Bool(true),
        "false" => Token::Bool(false),
        _ => Token::Ident(ident.into()),
    }
}
