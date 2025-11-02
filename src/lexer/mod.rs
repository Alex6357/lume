// src/lexer/mod.rs
pub mod token;

use crate::{error::LumeError, span::Span};
use std::iter::Peekable;
pub use token::Token;

pub fn lex(source: &str, file: &str) -> Result<Vec<(Token, Span)>, LumeError> {
    let mut lexer = Lexer::new(source, file);
    lexer.lex()
}

struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<std::str::CharIndices<'a>>,
    pos: usize,
    file: String,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, file: &str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            pos: 0,
            file: file.into(),
        }
    }

    fn lex(&mut self) -> Result<Vec<(Token, Span)>, LumeError> {
        let mut tokens = Vec::new();
        while let Some((start, ch)) = self.chars.next() {
            self.pos = start;
            match ch {
                ' ' | '\t' | '\n' | '\r' => continue,
                '0'..='9' => {
                    let num = self.read_number(ch, start)?;
                    tokens.push((Token::Int(num), self.span(start, self.pos)));
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let ident = self.read_ident(ch, start)?;
                    let token = token::keyword_or_ident(&ident);
                    tokens.push((token, self.span(start, self.pos)));
                }
                '"' => {
                    let s = self.read_string(start)?;
                    tokens.push((Token::Str(s), self.span(start, self.pos)));
                }
                '=' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        tokens.push((Token::EqEq, self.span(start, start + 2)));
                    } else if self.peek() == Some('>') {
                        //    self.peek().isSomeAnd([c] => c == '>') {
                        tokens.push((Token::Eq, self.span(start, start + 1)));
                    }
                }
                // TODO: 其他符号（+, -, (, ), {, }, ;, => 等）
                _ => {
                    return Err(LumeError::Lexical {
                        msg: format!("unexpected character: '{}'", ch),
                        span: self.span(start, start + ch.len_utf8()),
                    });
                }
            }
        }
        tokens.push((Token::Eof, self.span(self.source.len(), self.source.len())));
        Ok(tokens)
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(start, end, &self.file)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn read_number(&mut self, first: char, start: usize) -> Result<i64, LumeError> {
        let mut num_str = first.to_string();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.chars.next(); // 消费字符
            } else {
                break;
            }
        }
        self.pos = start + num_str.len();
        num_str.parse().map_err(|_| LumeError::Lexical {
            msg: "invalid integer literal".into(),
            span: self.span(start, self.pos),
        })
    }
    fn read_ident(&mut self, first: char, start: usize) -> Result<String, LumeError> {
        let mut ident = first.to_string();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.chars.next(); // 消费字符
            } else {
                break;
            }
        }
        self.pos = start + ident.len();
        Ok(ident)
    }

    fn read_string(&mut self, start: usize) -> Result<String, LumeError> {
        let mut s = String::new();
        while let Some((_, ch)) = self.chars.next() {
            if ch == '"' {
                self.pos += 1;
                return Ok(s);
            }
            s.push(ch);
        }
        Err(LumeError::Lexical {
            msg: "unterminated string literal".into(),
            span: self.span(start, self.pos),
        })
    }
}
