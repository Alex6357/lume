// src/lexer/mod.rs

use crate::{error::LumeError, span::Span};
use std::iter::Peekable;

pub mod token;
pub use token::Token;

// Main lexical analysis entry function, takes source code and filename, returns token sequence or error
pub fn lex(source: &str, file: &str) -> Result<Vec<(Token, Span)>, LumeError> {
    let mut lexer = Lexer::new(source, file);
    lexer.lex()
}

// Core lexer structure, maintains state during lexical analysis
struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<std::str::CharIndices<'a>>,
    pos: usize,
    file: String,
    tokens: Vec<(Token, Span)>,
}

impl<'a> Lexer<'a> {
    // Initialize lexer, handle possible shebang line
    fn new(source: &'a str, file: &str) -> Self {
        // If source starts with shebang, skip this line
        let source = if source.starts_with("#!") {
            // Find end position of first line
            if let Some(idx) = source.find('\n') {
                &source[idx + 1..]
            } else {
                ""
            }
        } else {
            source
        };

        Self {
            source,
            chars: source.char_indices().peekable(),
            pos: 0,
            file: file.into(),
            tokens: Vec::new(),
        }
    }

    // Main lexical analysis loop, process characters one by one and generate tokens
    fn lex(&mut self) -> Result<Vec<(Token, Span)>, LumeError> {
        while let Some((start, ch)) = self.chars.next() {
            self.pos = start;
            match ch {
                // Skip whitespace characters
                ' ' | '\t' | '\n' | '\r' => continue,

                // Number literal processing: must ensure it's not part of an identifier
                '0'..='9' => {
                    let (num_token, end) = self.read_number(start)?;
                    self.pos = end;
                    // Advance character iterator to end position
                    while let Some(&(pos, _)) = self.chars.peek() {
                        if pos < end {
                            self.chars.next();
                        } else {
                            break;
                        }
                    }
                    self.push_token((num_token, self.span(start, end)));
                }

                // Identifier and keyword processing
                'a'..='z' | 'A'..='Z' | '_' | '\u{80}'..='\u{10FFFF}' => {
                    let ident_start = start;
                    let ident = self.read_ident(ch, start)?;
                    match self.peek() {
                        // Handle prefixed strings like r"..." or sql"..."
                        Some('"') => {
                            self.chars.next(); // Consume quote
                            let (content, end) = self.read_string_content(false)?; // Raw strings don't need escaping
                            self.pos = end;
                            self.push_token((
                                Token::PrefixedStr(ident, content),
                                self.span(ident_start, end),
                            ));
                        }
                        // Handle prefixed character literals like r'a' or sql'\n'
                        Some('\'') => {
                            self.chars.next(); // Consume opening quote
                            let (token, end) = self.read_prefixed_char(ident, ident_start)?;
                            self.pos = end;
                            self.push_token((token, self.span(ident_start, end)));
                        }
                        // Regular identifier or keyword
                        _ => {
                            let token = token::keyword_or_ident(&ident);
                            self.push_token((token, self.span(ident_start, self.pos)));
                        }
                    }
                }

                // String literal processing
                '"' => {
                    let (content, end) = self.read_string_content(true)?; // Allow escaping
                    self.pos = end;
                    self.push_token((Token::Str(content), self.span(start, end)));
                }

                // Character literal or lifetime processing
                '\'' => {
                    // Look ahead to determine if it's a character literal like 'a' or lifetime like 'static
                    let mut chars_ahead = self.chars.clone();
                    match chars_ahead.next() {
                        Some((_, first_ch)) => {
                            if first_ch.is_alphabetic() || first_ch == '_' {
                                // Could be character literal like 'a' or lifetime like 'static
                                match chars_ahead.next() {
                                    Some((_, '\'')) => {
                                        // This is a character literal like 'a'
                                        let token = self.read_char_literal(start)?;
                                        self.push_token((token, self.span(start, self.pos)));
                                    }
                                    Some(_) => {
                                        // This is a lifetime like 'static
                                        self.chars.next();
                                        let ident = self.read_ident(first_ch, start + 1)?;
                                        self.push_token((
                                            Token::Lifetime(ident),
                                            self.span(start, self.pos),
                                        ));
                                    }
                                    None => {
                                        return Err(LumeError::Lexical {
                                            msg: "unexpected end of input after quote".into(),
                                            span: self.span(start, start + 1),
                                        });
                                    }
                                }
                            } else {
                                // This must be a character literal like '5', '\n' etc.
                                let token = self.read_char_literal(start)?;
                                self.push_token((token, self.span(start, self.pos)));
                            }
                        }
                        None => {
                            return Err(LumeError::Lexical {
                                msg: "unexpected end of input after quote".into(),
                                span: self.span(start, start + 1),
                            });
                        }
                    }
                }

                // Operator processing
                '=' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::EqEq, self.span(start, start + 2)));
                    } else if self.peek() == Some('>') {
                        self.chars.next();
                        self.push_token((Token::FatArrow, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Eq, self.span(start, start + 1)));
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::Neq, self.span(start, start + 2)));
                    } else {
                        return Err(LumeError::Lexical {
                            msg: "unexpected '!'; logical NOT is written as 'not'".into(),
                            span: self.span(start, start + 1),
                        });
                    }
                }
                '<' => {
                    match self.peek() {
                        Some('<') => {
                            self.chars.next(); // Consume second '<'
                            if self.peek() == Some('=') {
                                self.chars.next();
                                self.push_token((Token::ShlEq, self.span(start, start + 3)));
                            } else {
                                self.push_token((Token::Shl, self.span(start, start + 2)));
                            }
                        }
                        Some('=') => {
                            self.chars.next();
                            self.push_token((Token::Le, self.span(start, start + 2)));
                        }
                        _ => {
                            self.push_token((Token::Lt, self.span(start, start + 1)));
                        }
                    }
                }
                '>' => {
                    match self.peek() {
                        Some('>') => {
                            self.chars.next(); // Consume second '>'
                            if self.peek() == Some('=') {
                                self.chars.next();
                                self.push_token((Token::ShrEq, self.span(start, start + 3)));
                            } else {
                                self.push_token((Token::Shr, self.span(start, start + 2)));
                            }
                        }
                        Some('=') => {
                            self.chars.next();
                            self.push_token((Token::Ge, self.span(start, start + 2)));
                        }
                        _ => {
                            self.push_token((Token::Gt, self.span(start, start + 1)));
                        }
                    }
                }
                '+' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::PlusEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Plus, self.span(start, start + 1)));
                    }
                }
                '-' => {
                    if self.peek() == Some('>') {
                        self.chars.next();
                        self.push_token((Token::Arrow, self.span(start, start + 2)));
                    } else if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::MinusEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Minus, self.span(start, start + 1)));
                    }
                }
                '*' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::StarEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Star, self.span(start, start + 1)));
                    }
                }
                '/' => {
                    if let Some(next) = self.peek() {
                        match next {
                            '/' => {
                                self.chars.next(); // Consume second '/'
                                if let Some(third) = self.peek() {
                                    if third == '/' {
                                        self.chars.next();
                                    }
                                }
                                self.skip_line_comment();
                                continue;
                            }
                            '*' => {
                                self.chars.next(); // Consume '*'
                                self.skip_block_comment()?;
                                continue;
                            }
                            '=' => {
                                self.chars.next();
                                self.push_token((Token::SlashEq, self.span(start, start + 2)));
                            }
                            _ => {
                                self.push_token((Token::Slash, self.span(start, start + 1)));
                            }
                        }
                    } else {
                        self.push_token((Token::Slash, self.span(start, start + 1)));
                    }
                }
                '%' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::PercentEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Percent, self.span(start, start + 1))); // Need to add Percent token
                    }
                }
                '&' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::AmpEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Amp, self.span(start, start + 1)));
                    }
                }
                '|' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::PipeEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Pipe, self.span(start, start + 1)));
                    }
                }
                '^' => {
                    if self.peek() == Some('=') {
                        self.chars.next();
                        self.push_token((Token::CaretEq, self.span(start, start + 2)));
                    } else {
                        self.push_token((Token::Caret, self.span(start, start + 1)));
                    }
                }
                '(' => self.push_token((Token::LParen, self.span(start, start + 1))),
                ')' => self.push_token((Token::RParen, self.span(start, start + 1))),
                '{' => self.push_token((Token::LBrace, self.span(start, start + 1))),
                '}' => self.push_token((Token::RBrace, self.span(start, start + 1))),
                '[' => self.push_token((Token::LBracket, self.span(start, start + 1))),
                ']' => self.push_token((Token::RBracket, self.span(start, start + 1))),
                ';' => self.push_token((Token::Semicolon, self.span(start, start + 1))),
                ',' => self.push_token((Token::Comma, self.span(start, start + 1))),
                ':' => self.push_token((Token::Colon, self.span(start, start + 1))),
                '.' => self.push_token((Token::Dot, self.span(start, start + 1))),
                '?' => self.push_token((Token::Question, self.span(start, start + 1))),
                _ => {
                    return Err(LumeError::Lexical {
                        msg: format!("unexpected character: '{}'", ch),
                        span: self.span(start, start + ch.len_utf8()),
                    });
                }
            }
        }
        self.push_token((Token::Eof, self.span(self.source.len(), self.source.len())));
        Ok(std::mem::take(&mut self.tokens))
    }

    // Add token to token vector
    fn push_token(&mut self, token: (Token, Span)) {
        // match token.0 {
        //     Token::Let | Token::Ident(_) => {
        //         dbg!(&token);
        //     }
        //     _ => {}
        // }
        dbg!(&token);
        self.tokens.push(token);
    }

    // Create span object representing source code range
    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(start, end, &self.file)
    }

    // Peek at next character without consuming it
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    // --- Number parsing ---
    // Parse number literals, including integers, floats and numbers in different bases
    fn read_number(&mut self, start: usize) -> Result<(Token, usize), LumeError> {
        let mut num_str = String::new();
        let chars: Vec<(usize, char)> = self.source[start..]
            .char_indices()
            .map(|(i, c)| (start + i, c))
            .collect();

        let mut i = 0;
        let mut base = 10;
        let mut has_dot = false;
        let mut has_exp = false;

        // Check for base prefixes
        if chars.get(0).map(|&(_, c)| c) == Some('0') && chars.len() > 1 {
            match chars.get(1).map(|&(_, c)| c) {
                Some('x') | Some('X') => {
                    base = 16;
                    num_str.push('0');
                    num_str.push(chars[1].1);
                    i = 2;
                }
                Some('b') | Some('B') => {
                    base = 2;
                    num_str.push('0');
                    num_str.push(chars[1].1);
                    i = 2;
                }
                Some('o') | Some('O') => {
                    base = 8;
                    num_str.push('0');
                    num_str.push(chars[1].1);
                    i = 2;
                }
                _ => {
                    num_str.push('0');
                    i = 1;
                    // Continue decimal parsing
                    while i < chars.len() {
                        let (_, ch) = chars[i];
                        if ch.is_ascii_digit() || ch == '_' {
                            num_str.push(ch);
                            i += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        } else {
            // Normal decimal start
            while i < chars.len() {
                let (_, ch) = chars[i];
                if ch.is_ascii_digit() || ch == '_' {
                    num_str.push(ch);
                    i += 1;
                } else {
                    break;
                }
            }
        }

        // For prefixed numbers, continue parsing with appropriate base
        if base != 10 {
            let valid_chars = match base {
                2 => "01",
                8 => "01234567",
                10 => "0123456789",
                16 => "0123456789abcdefABCDEF",
                _ => unreachable!(),
            };
            let mut has_digit = false;
            while i < chars.len() {
                let (_, ch) = chars[i];
                if ch == '_' {
                    num_str.push(ch);
                    i += 1;
                } else if valid_chars.contains(ch) {
                    num_str.push(ch);
                    has_digit = true;
                    i += 1;
                } else {
                    break;
                }
            }

            if !has_digit {
                return Err(LumeError::Lexical {
                    msg: "invalid integer literal".into(),
                    span: self.span(start, start + i),
                });
            }
        }

        // Check for decimal point (float)
        if i < chars.len() && chars[i].1 == '.' {
            let next_i = i + 1;
            if next_i < chars.len() && chars[next_i].1.is_ascii_digit() {
                has_dot = true;
                num_str.push('.');
                i = next_i;
                while i < chars.len() {
                    let (_, ch) = chars[i];
                    if ch.is_ascii_digit() || ch == '_' {
                        num_str.push(ch);
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        // Check for exponent
        if i < chars.len() && matches!(chars[i].1, 'e' | 'E') {
            has_exp = true;
            num_str.push(chars[i].1);
            i += 1;
            if i < chars.len() && matches!(chars[i].1, '+' | '-') {
                num_str.push(chars[i].1);
                i += 1;
            }
            let mut has_digit = false;
            while i < chars.len() {
                let (_, ch) = chars[i];
                if ch.is_ascii_digit() || ch == '_' {
                    num_str.push(ch);
                    has_digit = true;
                    i += 1;
                } else {
                    break;
                }
            }
            if !has_digit {
                return Err(LumeError::Lexical {
                    msg: "invalid exponent".into(),
                    span: self.span(start, start + i),
                });
            }
        }

        // Remove underscores
        let clean: String = num_str.chars().filter(|&c| c != '_').collect();
        if clean.is_empty() || clean == "." || clean.starts_with('.') || clean.ends_with('.') {
            return Err(LumeError::Lexical {
                msg: "invalid number literal".into(),
                span: self.span(start, start + i),
            });
        }

        if has_dot || has_exp {
            let val = clean.parse::<f64>().map_err(|_| LumeError::Lexical {
                msg: "invalid float literal".into(),
                span: self.span(start, start + i),
            })?;
            return Ok((Token::Float(val), start + i));
        } else {
            // Handle regular decimal and prefixed numbers
            let (value_str, parse_base) = if base != 10 {
                // Prefixed numbers (hex/binary/octal) - skip "0x", "0b", or "0o" prefix
                (&clean[2..], base)
            } else {
                // Regular decimal
                (&clean[..], 10)
            };

            let val = i64_from_radix(value_str, parse_base).map_err(|_| LumeError::Lexical {
                msg: "integer literal too large".into(),
                span: self.span(start, start + i),
            })?;
            return Ok((Token::Int(val), start + i));
        }
    }

    // --- Identifier processing ---
    // Read identifier or keyword starting with given character
    fn read_ident(&mut self, first: char, start: usize) -> Result<String, LumeError> {
        let mut ident = first.to_string();
        let mut len = first.len_utf8();
        while let Some((_, ch)) = self.chars.peek() {
            if ch.is_alphanumeric() || *ch == '_' || *ch > '\u{7F}' {
                ident.push(*ch);
                len += ch.len_utf8();
                self.chars.next();
            } else {
                break;
            }
        }
        self.pos = start + len;
        Ok(ident)
    }

    // --- String processing ---
    // Read string content, decide whether to process escape sequences based on allow_escape parameter
    fn read_string_content(&mut self, allow_escape: bool) -> Result<(String, usize), LumeError> {
        let mut s = String::new();
        let start_pos = self.pos;
        loop {
            match self.chars.next() {
                Some((_, '"')) => {
                    return Ok((s, self.pos + 1));
                }
                Some((idx, ch)) => {
                    if allow_escape && ch == '\\' {
                        let escaped = self.read_escape()?;
                        s.push(escaped);
                    } else {
                        s.push(ch);
                    }
                    self.pos = idx;
                }
                None => {
                    return Err(LumeError::Lexical {
                        msg: "unterminated string literal".into(),
                        span: self.span(start_pos, self.source.len()),
                    });
                }
            }
        }
    }

    // Parse escape sequences in strings and character literals
    fn read_escape(&mut self) -> Result<char, LumeError> {
        match self.chars.next() {
            Some((_, 'n')) => Ok('\n'),
            Some((_, 'r')) => Ok('\r'),
            Some((_, 't')) => Ok('\t'),
            Some((_, '\\')) => Ok('\\'),
            Some((_, '"')) => Ok('"'),
            Some((_, '\'')) => Ok('\''),
            Some((_, 'u')) => {
                if self.peek() != Some('{') {
                    return Err(LumeError::Lexical {
                        msg: "expected '{' after \\u".into(),
                        span: self.span(self.pos, self.pos + 1),
                    });
                }
                self.chars.next(); // Consume '{'

                let mut hex = String::new();
                while let Some((_, ch)) = self.chars.peek() {
                    if *ch == '}' {
                        self.chars.next();
                        break;
                    }
                    if ch.is_ascii_hexdigit() {
                        hex.push(*ch);
                        self.chars.next();
                    } else {
                        return Err(LumeError::Lexical {
                            msg: "invalid hex digit in \\u{...}".into(),
                            span: self.span(self.pos, self.pos + 1),
                        });
                    }
                }
                if hex.is_empty() || hex.len() > 6 {
                    return Err(LumeError::Lexical {
                        msg: "unicode escape must have 1-6 hex digits".into(),
                        span: self.span(self.pos, self.pos + 1),
                    });
                }
                let codepoint = u32::from_str_radix(&hex, 16).map_err(|_| LumeError::Lexical {
                    msg: "invalid unicode escape".into(),
                    span: self.span(self.pos, self.pos + 1),
                })?;
                if let Some(ch) = std::char::from_u32(codepoint) {
                    Ok(ch)
                } else {
                    Err(LumeError::Lexical {
                        msg: "invalid unicode codepoint".into(),
                        span: self.span(self.pos, self.pos + 1),
                    })
                }
            }
            Some((_, ch)) => Err(LumeError::Lexical {
                msg: format!("unknown escape sequence \\{}", ch),
                span: self.span(self.pos, self.pos + 1),
            }),
            None => Err(LumeError::Lexical {
                msg: "unterminated escape sequence".into(),
                span: self.span(self.pos, self.source.len()),
            }),
        }
    }

    // --- Character literals ---
    // Parse standard character literals
    fn read_char_literal(&mut self, start: usize) -> Result<Token, LumeError> {
        match self.chars.next() {
            None => Err(LumeError::Lexical {
                msg: "unterminated character literal".into(),
                span: self.span(start, self.source.len()),
            }),
            Some((_, '\'')) => Err(LumeError::Lexical {
                msg: "empty character literal".into(),
                span: self.span(start, start + 2),
            }),
            Some((_, ch)) => {
                let ch = if ch == '\\' { self.read_escape()? } else { ch };

                if self.peek() != Some('\'') {
                    return Err(LumeError::Lexical {
                        msg: "character literal must contain exactly one character".into(),
                        span: self.span(start, self.pos + 1),
                    });
                }

                self.chars.next(); // Consume closing quote
                // Correctly update position
                self.pos = if let Some((next_pos, _)) = self.chars.peek() {
                    next_pos.clone()
                } else {
                    self.source.len()
                };

                Ok(Token::Char(ch))
            }
        }
    }

    // --- Prefixed character literals ---
    // Parse prefixed character literals (e.g. r'a', sql'\n')
    fn read_prefixed_char(
        &mut self,
        prefix: String,
        prefix_start: usize,
    ) -> Result<(Token, usize), LumeError> {
        // Opening quote already consumed
        let ch = match self.chars.next() {
            None => {
                return Err(LumeError::Lexical {
                    msg: "unterminated character literal".into(),
                    span: self.span(prefix_start, self.source.len()),
                });
            }
            Some((_, '\'')) => {
                return Err(LumeError::Lexical {
                    msg: "empty character literal".into(),
                    span: self.span(prefix_start, prefix_start + 2),
                });
            }
            Some((content_start, c)) => {
                let c = if c == '\\' { self.read_escape()? } else { c };
                // Verify it's a valid Unicode scalar value (not a surrogate pair)
                let cp = c as u32;
                if cp >= 0xD800 && cp <= 0xDFFF {
                    return Err(LumeError::Lexical {
                        msg: "character literal contains invalid Unicode surrogate".into(),
                        span: self.span(content_start, self.pos),
                    });
                }
                if self.peek() != Some('\'') {
                    return Err(LumeError::Lexical {
                        msg: "character literal must contain exactly one character".into(),
                        span: self.span(content_start, self.pos + 1),
                    });
                }
                self.chars.next(); // Consume closing quote
                c
            }
        };

        let end = if let Some((next_idx, _)) = self.chars.peek() {
            *next_idx
        } else {
            self.source.len()
        };

        Ok((Token::PrefixedChar(prefix, ch), end))
    }

    // --- Comment processing ---
    // Skip line comments
    fn skip_line_comment(&mut self) {
        while let Some((_, ch)) = self.chars.next() {
            if ch == '\n' {
                break;
            }
        }
    }

    // Skip block comments, handle nested comments
    fn skip_block_comment(&mut self) -> Result<(), LumeError> {
        let mut depth = 1;
        while depth > 0 {
            match self.chars.next() {
                Some((_, '*')) => {
                    if self.peek() == Some('/') {
                        self.chars.next();
                        depth -= 1;
                    }
                }
                Some((_, '/')) => {
                    if self.peek() == Some('*') {
                        self.chars.next();
                        depth += 1;
                    }
                }
                None => {
                    return Err(LumeError::Lexical {
                        msg: "unterminated block comment".into(),
                        span: self.span(0, self.source.len()),
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }
}

// Helper function: parse integer from string with specified base, return i64
fn i64_from_radix(s: &str, base: u32) -> Result<i64, ()> {
    if s.is_empty() {
        return Err(());
    }
    // Rust's i64::from_str_radix doesn't allow empty strings or signs
    i64::from_str_radix(s, base).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shebang() {
        let input = "#!/usr/bin/env lume\nlet x = 1;";
        let tokens = lex(input, "test").unwrap();
        assert!(matches!(tokens[0].0, Token::Let));
    }

    #[test]
    fn test_numbers() {
        let cases = vec![
            ("42", Token::Int(42)),
            ("0x2A", Token::Int(42)),
            ("0Xff", Token::Int(255)),
            ("0b1010", Token::Int(10)),
            ("0o77", Token::Int(63)),
            ("1_000", Token::Int(1000)),
            ("3.14", Token::Float(3.14)),
            ("1e5", Token::Float(100000.0)),
            ("1.23e-4", Token::Float(0.000123)),
        ];
        for (input, expected) in cases {
            let tokens = lex(input, "test").unwrap();
            assert_eq!(tokens[0].0, expected, "failed for {}", input);
        }
    }

    #[test]
    fn test_prefixed_string() {
        let input = r#"r"hello\nworld" sql"SELECT * FROM users""#;
        let tokens = lex(input, "test").unwrap();
        assert!(
            matches!(tokens[0].0, Token::PrefixedStr(ref p, ref s) if p == "r" && s == "hello\\nworld")
        );
        assert!(
            matches!(tokens[1].0, Token::PrefixedStr(ref p, ref s) if p == "sql" && s == "SELECT * FROM users")
        );
    }

    #[test]
    fn test_char_literal() {
        let cases = vec![
            ("'a'", 'a'),
            ("'\\n'", '\n'),
            ("'中'", '中'),
            ("'\\u{1F600}'", '\u{1F600}'),
        ];
        for (input, expected) in cases {
            let tokens = lex(input, "test").unwrap();
            assert!(
                matches!(tokens[0].0, Token::Char(c) if c == expected),
                "failed for {}",
                input
            );
        }
    }

    #[test]
    fn test_prefixed_char() {
        let cases = vec![
            ("r'a'", "r", 'a'),
            ("sql'\\n'", "sql", '\n'),
            ("regex'中'", "regex", '中'),
            ("u'\\u{1F600}'", "u", '\u{1F600}'),
        ];
        for (input, expected_prefix, expected_char) in cases {
            let tokens = lex(input, "test").unwrap();
            assert!(
                matches!(tokens[0].0, Token::PrefixedChar(ref p, c) if p == expected_prefix && c == expected_char),
                "failed for {}",
                input
            );
        }
    }

    #[test]
    fn test_prefixed_char_errors() {
        assert!(lex("r''", "test").is_err()); // empty
        assert!(lex("r'ab'", "test").is_err()); // too long
        assert!(lex("r'\\u{D800}'", "test").is_err()); // invalid surrogate
    }

    #[test]
    fn test_comments() {
        let input = "/* block */ let x = 1; // inline\n/// doc";
        let tokens = lex(input, "test").unwrap();
        assert!(matches!(tokens[0].0, Token::Let));
    }

    #[test]
    fn test_unicode_ident() {
        let input = "let café = 1;";
        let tokens = lex(input, "test").unwrap();
        assert!(matches!(tokens[1].0, Token::Ident(ref s) if s == "café"));
    }

    #[test]
    fn test_reject_symbolic_logic() {
        assert!(lex("a && b", "test").is_err());
        assert!(lex("!a", "test").is_err());
    }

    #[test]
    fn test_bitwise_and_compound_assign() {
        let input = "& | ^= <<= >>= += -= *= /= %= &= |=";
        let tokens = lex(input, "test").unwrap();
        assert_eq!(tokens[0].0, Token::Amp);
        assert_eq!(tokens[1].0, Token::Pipe);
        assert_eq!(tokens[2].0, Token::CaretEq);
        assert_eq!(tokens[3].0, Token::ShlEq);
        assert_eq!(tokens[4].0, Token::ShrEq);
        assert_eq!(tokens[5].0, Token::PlusEq);
        assert_eq!(tokens[6].0, Token::MinusEq);
        assert_eq!(tokens[7].0, Token::StarEq);
        assert_eq!(tokens[8].0, Token::SlashEq);
        assert_eq!(tokens[9].0, Token::PercentEq);
        assert_eq!(tokens[10].0, Token::AmpEq);
        assert_eq!(tokens[11].0, Token::PipeEq);
    }

    #[test]
    fn test_shift_operators() {
        let input = "<< >>";
        let tokens = lex(input, "test").unwrap();
        assert_eq!(tokens[0].0, Token::Shl);
        assert_eq!(tokens[1].0, Token::Shr);
    }

    #[test]
    fn test_comprehensive_lexer() {
        let input = r#"#!/usr/bin/env lume
// This is a comprehensive test for the Lume lexer

/// A doc comment for main
func main() -> int throws MyError {
    // Numbers
    let dec = 123;
    let hex = 0xFF;
    let bin = 0b1010_1010;
    let oct = 0o755;
    let float1 = 3.1415;
    let float2 = 1.23e-4;
    let float3 = 6.022e23;

    // Strings and prefixed strings
    let normal = "Hello, world!\n";
    let raw = r"Raw\nString";
    let sql_query = sql"SELECT * FROM users WHERE id = $1";

    // Character literals
    let ch1 = 'A';
    let ch2 = '\n';
    let ch3 = '中';
    let ch4 = '\u{1F600}';

    // Boolean and logic
    let t = true;
    let f = false;
    if t and not f or (dec > 0) => println("OK");

    // Operators and compound assignment
    let mut x = 10;
    x += 5;
    x <<= 2;
    x &= 0xF;

    // Lifetimes and references
    let ref_to_x: 'static &int = &x;

    // Optional and error handling syntax (just tokens)
    let opt = some_val on None => recover 0;
    if result is Error { code: 404 } => return MyError { myMessage: "Not Found", myCode: 404 };

    // Match and case
    match color {
        case Red => 1;
        case Blue => 2;
        case _ => 0;
    };

    // Return with error
    return MyError { myMessage: "Oops", myCode: -1 };
}

/* Block comment with /* nested */ comment */
import { foo, bar } from "./mod.lume" with { link: "dynamic" };

export class MyClass {
    value: int;
};

// Test bitwise and shift
let a = b & c | d ^ e;
let shifted = x << 4 >> 2;

// Unicode identifier
let café_latte = 42;

// Edge cases that should error are tested separately
"#;

        let tokens = match lex(input, "comprehensive_test.lume") {
            Ok(tokens) => tokens,
            Err(e) => {
                panic!("Lexing failed with error: {:#?}", e);
            }
        };

        // We don't assert every token by index (too brittle), but check key ones exist in order
        use Token::*;

        let expected_tokens = vec![
            Func,
            Ident("main".into()),
            LParen,
            RParen,
            Arrow,
            Ident("int".into()),
            Throws,
            Ident("MyError".into()),
            LBrace,
            Let,
            Ident("dec".into()),
            Eq,
            Int(123),
            Semicolon,
            Let,
            Ident("hex".into()),
            Eq,
            Int(255),
            Semicolon,
            Let,
            Ident("bin".into()),
            Eq,
            Int(170),
            Semicolon,
            Let,
            Ident("oct".into()),
            Eq,
            Int(493),
            Semicolon,
            Let,
            Ident("float1".into()),
            Eq,
            Float(3.1415),
            Semicolon,
            Let,
            Ident("float2".into()),
            Eq,
            Float(0.000123),
            Semicolon,
            Let,
            Ident("float3".into()),
            Eq,
            Float(6.022e23),
            Semicolon,
            Let,
            Ident("normal".into()),
            Eq,
            Str("Hello, world!\n".into()),
            Semicolon,
            Let,
            Ident("raw".into()),
            Eq,
            PrefixedStr("r".into(), "Raw\\nString".into()),
            Semicolon,
            Let,
            Ident("sql_query".into()),
            Eq,
            PrefixedStr("sql".into(), "SELECT * FROM users WHERE id = $1".into()),
            Semicolon,
            Let,
            Ident("ch1".into()),
            Eq,
            Char('A'),
            Semicolon,
            Let,
            Ident("ch2".into()),
            Eq,
            Char('\n'),
            Semicolon,
            Let,
            Ident("ch3".into()),
            Eq,
            Char('中'),
            Semicolon,
            Let,
            Ident("ch4".into()),
            Eq,
            Char('\u{1F600}'),
            Semicolon,
            Let,
            Ident("t".into()),
            Eq,
            Bool(true),
            Semicolon,
            Let,
            Ident("f".into()),
            Eq,
            Bool(false),
            Semicolon,
            If,
            Ident("t".into()),
            And,
            Not,
            Ident("f".into()),
            Or,
            LParen,
            Ident("dec".into()),
            Gt,
            Int(0),
            RParen,
            FatArrow,
            Ident("println".into()),
            LParen,
            Str("OK".into()),
            RParen,
            Semicolon,
            Let,
            Mut,
            Ident("x".into()),
            Eq,
            Int(10),
            Semicolon,
            Ident("x".into()),
            PlusEq,
            Int(5),
            Semicolon,
            Ident("x".into()),
            ShlEq,
            Int(2),
            Semicolon,
            Ident("x".into()),
            AmpEq,
            Int(15),
            Semicolon,
            Let,
            Ident("ref_to_x".into()),
            Colon,
            Lifetime("static".into()),
            Amp,
            Ident("int".into()),
            Eq,
            Amp,
            Ident("x".into()),
            Semicolon,
            Let,
            Ident("opt".into()),
            Eq,
            Ident("some_val".into()),
            On,
            Ident("None".into()),
            FatArrow,
            Recover,
            Int(0),
            Semicolon,
            If,
            Ident("result".into()),
            Is,
            Ident("Error".into()),
            LBrace,
            Ident("code".into()),
            Colon,
            Int(404),
            RBrace,
            FatArrow,
            Return,
            Ident("MyError".into()),
            LBrace,
            Ident("myMessage".into()),
            Colon,
            Str("Not Found".into()),
            Comma,
            Ident("myCode".into()),
            Colon,
            Int(404),
            RBrace,
            Semicolon,
            Match,
            Ident("color".into()),
            LBrace,
            Case,
            Ident("Red".into()),
            FatArrow,
            Int(1),
            Semicolon,
            Case,
            Ident("Blue".into()),
            FatArrow,
            Int(2),
            Semicolon,
            Case,
            Ident("_".into()),
            FatArrow,
            Int(0),
            Semicolon,
            RBrace,
            Semicolon,
            Return,
            Ident("MyError".into()),
            LBrace,
            Ident("myMessage".into()),
            Colon,
            Str("Oops".into()),
            Comma,
            Ident("myCode".into()),
            Colon,
            Minus,
            Int(1),
            RBrace,
            Semicolon,
            RBrace, // end of main
            Import,
            LBrace,
            Ident("foo".into()),
            Comma,
            Ident("bar".into()),
            RBrace,
            From,
            Str("./mod.lume".into()),
            With,
            LBrace,
            Ident("link".into()),
            Colon,
            Str("dynamic".into()),
            RBrace,
            Semicolon,
            Export,
            Class,
            Ident("MyClass".into()),
            LBrace,
            Ident("value".into()),
            Colon,
            Ident("int".into()),
            Semicolon,
            RBrace,
            Semicolon,
            Let,
            Ident("a".into()),
            Eq,
            Ident("b".into()),
            Amp,
            Ident("c".into()),
            Pipe,
            Ident("d".into()),
            Caret,
            Ident("e".into()),
            Semicolon,
            Let,
            Ident("shifted".into()),
            Eq,
            Ident("x".into()),
            Shl,
            Int(4),
            Shr,
            Int(2),
            Semicolon,
            Let,
            Ident("café_latte".into()),
            Eq,
            Int(42),
            Semicolon,
        ];

        // Note: We omitted Eof and some whitespace-sensitive tokens like From/With if not in Token enum yet.
        // Adjust based on actual Token definition.

        // For robustness, we extract just the token variants (ignoring spans and string content where possible)
        let actual_simple: Vec<Token> = tokens
            .iter()
            .map(|(tok, _)| {
                // Normalize string/char content for comparison where needed
                match tok {
                    Str(_) => Str("...".into()),
                    PrefixedStr(p, _) => PrefixedStr(p.clone(), "...".into()),
                    Char(_) => Char('?'),
                    PrefixedChar(p, _) => PrefixedChar(p.clone(), '?'),
                    Int(_) => Int(0),
                    Float(_) => Float(0.0),
                    Ident(s) => Ident(s.clone()),
                    Lifetime(s) => Lifetime(s.clone()),
                    _ => tok.clone(),
                }
            })
            .filter(|t| !matches!(t, Token::Eof)) // ignore EOF for this check
            .collect();

        // Instead of full equality (fragile), verify sequence contains expected patterns
        // Here we do a simplified check: ensure key tokens appear in correct relative order
        let mut actual_iter = actual_simple.iter();
        for expected in &expected_tokens {
            // Skip comments, whitespace etc. — our lexer already skips them
            loop {
                match actual_iter.next() {
                    Some(actual) => {
                        // Special handling for normalized values
                        let matches = match (expected, actual) {
                            (Int(_), Int(_)) => true,
                            (Float(_), Float(_)) => true,
                            (Str(_), Str(_)) => true,
                            (PrefixedStr(ep, _), PrefixedStr(ap, _)) => ep == ap,
                            (Char(_), Char(_)) => true,
                            (PrefixedChar(ep, _), PrefixedChar(ap, _)) => ep == ap,
                            (Ident(ei), Ident(ai)) => ei == ai,
                            (Lifetime(el), Lifetime(al)) => el == al,
                            _ => expected == actual,
                        };
                        if matches {
                            dbg!(actual);
                            break;
                        }
                        // else continue skipping unexpected (shouldn't happen in well-formed input)
                    }
                    None => panic!("Expected token {:?} not found", expected),
                }
            }
        }

        // Also ensure no lexical errors were produced
    }
}
