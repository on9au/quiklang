//! # Lexer
//!
//! The lexer is responsible for converting the source code into a stream of tokens.
//!
//! ## Logic
//!
//! The lexer works by iterating over the source code character by character.
//!
//! The lexer recognizes different types of tokens:
//! - Keywords
//! - Identifiers
//! - Literals
//! - Operators
//! - Symbols/Delimiters
//! - End of File

use std::{
    iter::Peekable,
    ops::Index,
    str::{Chars, FromStr},
};

use quiklang_common::{
    data_structs::tokens::{Keyword, Operator, Symbol, TokenType},
    errors::{lexer::LexerError, parser::ParserError, CompilerError, Span},
    CompilationReport,
};

/// Lexer States
#[derive(Debug, PartialEq, Eq)]
enum LexingState {
    Normal,
    InString,
    InInterpolation,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct StringInfo {
    pub start_pos: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub buffer: String,
    pub complex_string: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub token: TokenType,
    pub span: Span,
}

/// Custom made struct to implement line and col.
struct CharStream<'a> {
    chars: Peekable<Chars<'a>>,
    current_pos: usize, // Byte offset from the beginning
    line: usize,
    col: usize,
}

impl<'a> CharStream<'a> {
    fn new(source_code: &'a str) -> Self {
        CharStream {
            chars: source_code.chars().peekable(),
            current_pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(c) = self.chars.next() {
            let c_len = c.len_utf8();
            self.current_pos += c_len;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn get_position(&self) -> (usize, usize, usize) {
        (self.current_pos, self.line, self.col)
    }
}

#[derive(Debug)]
pub struct Tokens {
    tokens: Vec<Token>,
    index: usize,
}

impl Tokens {
    pub fn new(tokens: Vec<Token>) -> Self {
        Tokens { tokens, index: 0 }
    }

    pub fn not_eof(&self) -> bool {
        self.tokens
            .get(self.index)
            .is_some_and(|t| t.token != TokenType::EOF)
    }

    pub fn at(&self) -> &Token {
        &self.tokens[self.index]
    }

    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index + 1)
    }

    pub fn eat(&mut self) -> &Token {
        let token = &self.tokens[self.index];
        self.index += 1;
        token
    }

    // amazing name for this function B))
    pub fn un_eat(&mut self) {
        self.index -= 1;
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn expect(&mut self, expected: TokenType, report: &mut CompilationReport) -> Option<()> {
        if self.at().token != expected {
            report.add_error(CompilerError::ParserError(ParserError::UnexpectedToken {
                expected: expected.to_string(),
                found: self.at().token.clone(),
                span: self.at().span,
                suggestion: vec![],
            }));
            self.eat();
            return None;
        }
        self.eat();
        Some(())
    }
}

impl Index<usize> for Tokens {
    type Output = Token;

    fn index(&self, index: usize) -> &Self::Output {
        &self.tokens[index]
    }
}

impl Iterator for Tokens {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else {
            None
        }
    }
}

/// Tokenizes the input source code and returns a list of tokens.
/// Errors are pushed into the `report`.
pub fn tokenize(source_code: &str, file_id: usize, report: &mut CompilationReport) -> Tokens {
    let mut chars = CharStream::new(source_code);
    let mut tokens = Vec::new();
    let mut state = LexingState::Normal;

    // Temporary storage for string literals
    let mut string_info = StringInfo {
        start_pos: 0,
        start_line: 0,
        start_col: 0,
        buffer: String::new(),
        complex_string: false,
    };

    while let Some(&c) = chars.peek_char() {
        match state {
            LexingState::Normal => {
                tokenize_normally(
                    c,
                    file_id,
                    report,
                    &mut chars,
                    &mut tokens,
                    &mut state,
                    &mut string_info,
                );
            }
            LexingState::InString => {
                // Inside a string literal
                if let Some(&c) = chars.peek_char() {
                    match c {
                        '"' => {
                            // End of string literal
                            chars.next_char(); // Consume '"'
                            tokens.push(Token {
                                token: TokenType::StringLiteral(string_info.buffer.clone()),
                                span: Span::new(
                                    file_id,
                                    string_info.start_pos,
                                    chars.current_pos,
                                    string_info.start_line,
                                    string_info.start_col,
                                ),
                            });
                            if string_info.complex_string {
                                // Insert Complex String End to indicate the end of an interpolated string
                                tokens.push(Token {
                                    token: TokenType::ComplexStringEnd,
                                    span: Span::new(
                                        file_id,
                                        chars.current_pos,
                                        chars.current_pos,
                                        chars.line,
                                        chars.col,
                                    ),
                                });
                            }
                            string_info.buffer.clear(); // Clear the buffer
                            string_info.complex_string = false; // Reset complex string flag
                            state = LexingState::Normal;
                            continue;
                        }
                        '$' => {
                            chars.next_char(); // Consume '$'

                            // Check for presence of '{' after '$'
                            if let Some(&'{') = chars.peek_char() {
                                // Start of interpolation
                                chars.next_char(); // Consume '{'
                                string_info.complex_string = true; // Set complex string flag
                                if !string_info.buffer.is_empty() {
                                    // Insert Complex String Start to indicate an interpolated string
                                    tokens.push(Token {
                                        token: TokenType::ComplexStringStart,
                                        span: Span::new(
                                            file_id,
                                            string_info.start_pos,
                                            string_info.start_pos,
                                            string_info.start_line,
                                            string_info.start_col,
                                        ),
                                    });
                                    // Emit the string segment before '{'
                                    tokens.push(Token {
                                        token: TokenType::StringLiteral(string_info.buffer.clone()),
                                        span: Span::new(
                                            file_id,
                                            string_info.start_pos,
                                            chars.current_pos - 1,
                                            string_info.start_line,
                                            string_info.start_col,
                                        ),
                                    });
                                    string_info.buffer.clear();
                                }
                                tokens.push(Token {
                                    token: TokenType::StringInterpolationStart,
                                    span: Span::new(
                                        file_id,
                                        chars.current_pos - 1,
                                        chars.current_pos,
                                        chars.line,
                                        chars.col,
                                    ),
                                });
                                state = LexingState::InInterpolation;
                                continue;
                            } else {
                                // Treat '$' as a regular character
                                string_info.buffer.push('$');
                                continue;
                            }
                        }
                        '\\' => {
                            // Handle escape sequences
                            chars.next_char(); // Consume '\\'
                            if let Some(&escaped_char) = chars.peek_char() {
                                let escaped = match escaped_char {
                                    // 'n' => '\n',
                                    // 't' => '\t',
                                    // 'r' => '\r',
                                    // '"' => '"',
                                    // '\\' => '\\',
                                    // '$' => '$',
                                    '\\' => {
                                        chars.next_char(); // Consume '\\'
                                        '\\'
                                    }
                                    '\"' => {
                                        chars.next_char(); // Consume '\"'
                                        '\"'
                                    }
                                    '\'' => {
                                        chars.next_char(); // Consume '\''
                                        '\''
                                    }
                                    'n' => {
                                        chars.next_char(); // Consume 'n'
                                        '\n'
                                    }
                                    't' => {
                                        chars.next_char(); // Consume 't'
                                        '\t'
                                    }
                                    'r' => {
                                        chars.next_char(); // Consume 'r'
                                        '\r'
                                    }
                                    '0' => {
                                        chars.next_char(); // Consume '0'
                                        '\0'
                                    }
                                    'x' => {
                                        // Hexadecimal escape sequence
                                        chars.next_char(); // Consume 'x'
                                        let mut hex = String::new();
                                        while let Some(&c) = chars.peek_char() {
                                            if c.is_ascii_hexdigit() {
                                                hex.push(c);
                                                chars.next_char();
                                            } else {
                                                break;
                                            }
                                        }
                                        match u32::from_str_radix(&hex, 16) {
                                            Ok(value) => match std::char::from_u32(value) {
                                                Some(c) => c,
                                                None => {
                                                    report.add_error(
                                                            LexerError::InvalidEscapeSequence {
                                                                span: Span::new(
                                                                    file_id,
                                                                    string_info.start_pos,
                                                                    chars.current_pos,
                                                                    string_info.start_line,
                                                                    string_info.start_col,
                                                                ),
                                                                suggestions: vec![
                                                                    "The escape sequence is not a valid Unicode code point."
                                                                        .to_string(),
                                                                ],
                                                            }
                                                            .into(),
                                                        );
                                                    continue;
                                                }
                                            },
                                            Err(_) => {
                                                report.add_error(
                                                    LexerError::InvalidEscapeSequence {
                                                        span: Span::new(
                                                            file_id,
                                                            string_info.start_pos,
                                                            chars.current_pos,
                                                            string_info.start_line,
                                                            string_info.start_col,
                                                        ),
                                                        suggestions: vec![
                                                            "The escape sequence is not a valid hexadecimal number."
                                                                .to_string(),
                                                        ],
                                                    }
                                                    .into(),
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                    'u' => {
                                        // Unicode escape sequence
                                        chars.next_char(); // Consume 'u'
                                        let mut hex = String::new();
                                        while let Some(&c) = chars.peek_char() {
                                            if c.is_ascii_hexdigit() {
                                                hex.push(c);
                                                chars.next_char();
                                            } else {
                                                break;
                                            }
                                        }
                                        match u32::from_str_radix(&hex, 16) {
                                            Ok(value) => match std::char::from_u32(value) {
                                                Some(c) => c,
                                                None => {
                                                    report.add_error(
                                                            LexerError::InvalidEscapeSequence {
                                                                span: Span::new(
                                                                    file_id,
                                                                    string_info.start_pos,
                                                                    chars.current_pos,
                                                                    string_info.start_line,
                                                                    string_info.start_col,
                                                                ),
                                                                suggestions: vec![
                                                                    "The escape sequence is not a valid Unicode code point."
                                                                        .to_string(),
                                                                ],
                                                            }
                                                            .into(),
                                                        );
                                                    continue;
                                                }
                                            },
                                            Err(_) => {
                                                report.add_error(
                                                    LexerError::InvalidEscapeSequence {
                                                        span: Span::new(
                                                            file_id,
                                                            string_info.start_pos,
                                                            chars.current_pos,
                                                            string_info.start_line,
                                                            string_info.start_col,
                                                        ),
                                                        suggestions: vec![
                                                            "The escape sequence is not a valid hexadecimal number."
                                                                .to_string(),
                                                        ],
                                                    }
                                                    .into(),
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                    'U' => {
                                        // Unicode Extended escape sequence
                                        chars.next_char(); // Consume 'U'
                                        let mut hex = String::new();
                                        while let Some(&c) = chars.peek_char() {
                                            if c.is_ascii_hexdigit() {
                                                hex.push(c);
                                                chars.next_char();
                                            } else {
                                                break;
                                            }
                                        }
                                        match u32::from_str_radix(&hex, 16) {
                                            Ok(value) => match std::char::from_u32(value) {
                                                Some(c) => c,
                                                None => {
                                                    report.add_error(
                                                            LexerError::InvalidEscapeSequence {
                                                                span: Span::new(
                                                                    file_id,
                                                                    string_info.start_pos,
                                                                    chars.current_pos,
                                                                    string_info.start_line,
                                                                    string_info.start_col,
                                                                ),
                                                                suggestions: vec![
                                                                    "The escape sequence is not a valid Unicode code point."
                                                                        .to_string(),
                                                                ],
                                                            }
                                                            .into(),
                                                        );
                                                    continue;
                                                }
                                            },
                                            Err(_) => {
                                                report.add_error(
                                                    LexerError::InvalidEscapeSequence {
                                                        span: Span::new(
                                                            file_id,
                                                            string_info.start_pos,
                                                            chars.current_pos,
                                                            string_info.start_line,
                                                            string_info.start_col,
                                                        ),
                                                        suggestions: vec![
                                                            "The escape sequence is not a valid hexadecimal number."
                                                                .to_string(),
                                                        ],
                                                    }
                                                    .into(),
                                                );
                                                continue;
                                            }
                                        }
                                    }
                                    '$' => '$',
                                    e => {
                                        report.add_error(
                                            LexerError::InvalidEscapeSequence {
                                                span: Span::new(
                                                    file_id,
                                                    string_info.start_pos,
                                                    chars.current_pos,
                                                    string_info.start_line,
                                                    string_info.start_col,
                                                ),
                                                suggestions: vec![format!(
                                                    "'{}' is not a valid escape sequence.",
                                                    e
                                                )],
                                            }
                                            .into(),
                                        );
                                        continue;
                                    }
                                };
                                string_info.buffer.push(escaped);
                            } else {
                                // Unterminated string literal
                                report.add_error(
                                    LexerError::UnterminatedStringLiteral {
                                        span: Span::new(
                                            file_id,
                                            string_info.start_pos,
                                            chars.current_pos,
                                            string_info.start_line,
                                            string_info.start_col,
                                        ),
                                        suggestions: vec![
                                            "You may have forgotten to close the string with '\"'."
                                                .to_string(),
                                        ],
                                    }
                                    .into(),
                                );
                                state = LexingState::Normal;
                                break;
                            }
                            continue;
                        }
                        _ => {
                            // Regular character inside string
                            string_info.buffer.push(c);
                            chars.next_char(); // Consume character
                            continue;
                        }
                    }
                } else {
                    // Unterminated string literal
                    report.add_error(
                        LexerError::UnterminatedStringLiteral {
                            span: Span::new(
                                file_id,
                                string_info.start_pos,
                                chars.current_pos,
                                string_info.start_line,
                                string_info.start_col,
                            ),
                            suggestions: vec![
                                "You may have forgotten to close the string with '\"'.".to_string(),
                            ],
                        }
                        .into(),
                    );
                    state = LexingState::Normal;
                    break;
                }
            }
            LexingState::InInterpolation => {
                // Inside an interpolation expression
                // Lex tokens until the closing '}'
                if is_skippable(c) {
                    chars.next_char();
                    continue;
                }

                match c {
                    '}' => {
                        chars.next_char(); // Consume '}'
                        tokens.push(Token {
                            token: TokenType::StringInterpolationEnd,
                            span: Span::new(
                                file_id,
                                chars.current_pos - 1,
                                chars.current_pos,
                                chars.line,
                                chars.col,
                            ),
                        });
                        state = LexingState::InString;
                        continue;
                    }
                    _ => {
                        // Lex tokens inside interpolation
                        tokenize_normally(
                            c,
                            file_id,
                            report,
                            &mut chars,
                            &mut tokens,
                            &mut state,
                            &mut string_info,
                        );
                    }
                }
            }
        }
    }

    // Handle any remaining state
    match state {
        LexingState::InString => {
            // Unterminated string literal
            report.add_error(
                LexerError::UnterminatedStringLiteral {
                    span: Span::new(
                        file_id,
                        string_info.start_pos,
                        chars.current_pos,
                        string_info.start_line,
                        string_info.start_col,
                    ),
                    suggestions: vec![
                        "You may have forgotten to close the string with '\"'.".to_string()
                    ],
                }
                .into(),
            );
            tokens.push(Token {
                token: TokenType::EOF,
                span: Span::new(
                    file_id,
                    chars.current_pos,
                    chars.current_pos,
                    chars.line,
                    chars.col,
                ),
            });
        }
        LexingState::InInterpolation => {
            // Unterminated interpolation
            report.add_error(
                LexerError::UnterminatedInterpolation {
                    span: Span::new(
                        file_id,
                        string_info.start_pos,
                        chars.current_pos,
                        string_info.start_line,
                        string_info.start_col,
                    ),
                    suggestions: vec![
                        "You may have forgotten to close the interpolation with '}'.".to_string(),
                    ],
                }
                .into(),
            );
            tokens.push(Token {
                token: TokenType::EOF,
                span: Span::new(
                    file_id,
                    chars.current_pos,
                    chars.current_pos,
                    chars.line,
                    chars.col,
                ),
            });
        }
        _ => {
            // Normal or other states
            tokens.push(Token {
                token: TokenType::EOF,
                span: Span::new(
                    file_id,
                    chars.current_pos,
                    chars.current_pos,
                    chars.line,
                    chars.col,
                ),
            });
        }
    }

    // Optional: Print tokens for debugging
    // println!("{:#?}", tokens);

    Tokens::new(tokens)
}

fn tokenize_normally(
    c: char,
    file_id: usize,
    report: &mut CompilationReport,
    chars: &mut CharStream,
    tokens: &mut Vec<Token>,
    state: &mut LexingState,
    string_info: &mut StringInfo,
) {
    if is_skippable(c) {
        chars.next_char();
        return;
    }

    let (start_pos, start_line, start_col) = chars.get_position();

    match c {
        '/' => {
            // Handle comments or division
            chars.next_char(); // Consume '/'
            if let Some(&next_char) = chars.peek_char() {
                if next_char == '/' {
                    // Single-line comment: skip until end of line
                    chars.next_char(); // Consume second '/'
                    while let Some(c) = chars.next_char() {
                        if c == '\n' {
                            break;
                        }
                    }
                } else if next_char == '*' {
                    // Multi-line comment: skip until '*/'
                    chars.next_char(); // Consume '*'
                    let mut found_end = false;
                    while let Some(c) = chars.next_char() {
                        if c == '*' {
                            if let Some(&'/') = chars.peek_char() {
                                chars.next_char(); // Consume '/'
                                found_end = true;
                                break;
                            }
                        }
                    }
                    if !found_end {
                        report.add_error(CompilerError::LexerError(
                            LexerError::UnterminatedMultiLineComment {
                                span: Span::new(
                                    file_id,
                                    start_pos,
                                    chars.current_pos,
                                    start_line,
                                    start_col,
                                ),
                                suggestions: vec![
                                    "You may have forgotten to close the comment with '*/'."
                                        .to_string(),
                                ],
                            },
                        ));
                    }
                    return;
                } else {
                    // It's a division operator
                    tokens.push(Token {
                        token: TokenType::Operator(Operator::Divide),
                        span: Span::new(
                            file_id,
                            start_pos,
                            chars.current_pos,
                            start_line,
                            start_col,
                        ),
                    });
                    return;
                }
            } else {
                // It's a division operator at EOF
                tokens.push(Token {
                    token: TokenType::Operator(Operator::Divide),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                });
            }
        }
        '"' => {
            *state = LexingState::InString;
            chars.next_char(); // Consume '"'
            string_info.start_pos = start_pos;
            string_info.start_line = start_line;
            string_info.start_col = start_col;
        }
        c if is_identifier_start(c) => {
            let identifier = lex_identifier(chars);
            let end_pos = chars.current_pos;
            let _end_line = chars.line;
            let _end_col = chars.col;
            let token_type = match Keyword::from_str(&identifier) {
                Ok(keyword) => TokenType::Keyword(keyword),
                Err(_) => TokenType::Identifier(identifier),
            };
            tokens.push(Token {
                token: token_type,
                span: Span::new(file_id, start_pos, end_pos, start_line, start_col),
            });
        }
        c if c.is_ascii_digit() => {
            match lex_number(chars, file_id, start_pos, start_line, start_col, report) {
                Ok(token) => tokens.push(token),
                Err(_) => {
                    // Error already reported
                }
            }
        }
        '+' | '-' | '*' | '=' | '!' | '>' | '<' | '&' | '|' | '@' | '^' | '~' | '.' | ':' | '?' => {
            match tokenize_operator_or_symbol(
                chars, file_id, start_pos, start_line, start_col, c, report,
            ) {
                Ok(Some(token)) => tokens.push(token),
                Ok(None) => (), // Token was already handled (e.g., comments)
                Err(_) => {
                    // Error already reported
                }
            }
        }
        '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' => {
            let symbol = match c {
                '(' => Symbol::LeftParen,
                ')' => Symbol::RightParen,
                '{' => Symbol::LeftBrace,
                '}' => Symbol::RightBrace,
                '[' => Symbol::LeftBracket,
                ']' => Symbol::RightBracket,
                ',' => Symbol::Comma,
                ';' => Symbol::Semicolon,
                _ => unreachable!(),
            };
            chars.next_char(); // Consume symbol
            tokens.push(Token {
                token: TokenType::Symbol(symbol),
                span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
            });
        }
        _ => {
            // Unrecognized character
            report.add_error(CompilerError::LexerError(
                LexerError::UnrecognizedCharacter {
                    character: c,
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                    suggestion: vec![],
                },
            ));
            chars.next_char(); // Skip the unrecognized character
        }
    }
}

fn is_skippable(c: char) -> bool {
    c.is_whitespace()
}

fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn lex_identifier(chars: &mut CharStream) -> String {
    let mut identifier = String::new();
    while let Some(&c) = chars.peek_char() {
        if c.is_alphanumeric() || c == '_' {
            identifier.push(c);
            chars.next_char();
        } else {
            break;
        }
    }
    identifier
}

fn lex_number(
    chars: &mut CharStream,
    file_id: usize,
    start_pos: usize,
    start_line: usize,
    start_col: usize,
    report: &mut CompilationReport,
) -> Result<Token, ()> {
    let mut number = String::new();
    let mut has_dot = false;

    while let Some(&c) = chars.peek_char() {
        if c.is_ascii_digit() {
            number.push(c);
            chars.next_char();
        } else if c == '.' {
            has_dot = true;
            number.push(c);
            chars.next_char();
        } else {
            break;
        }
    }

    if has_dot {
        match number.parse::<f64>() {
            Ok(value) => Ok(Token {
                token: TokenType::FloatLiteral(value),
                span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
            }),
            Err(_) => {
                // Check if float has multiple decimal points
                if number.matches('.').count() > 1 {
                    report.add_error(CompilerError::LexerError(LexerError::InvalidNumberFormat {
                        invalid_string: number,
                        span: Span::new(
                            file_id,
                            start_pos,
                            chars.current_pos,
                            start_line,
                            start_col,
                        ),
                        suggestions: vec!["You may have multiple decimal points.".to_string()],
                    }));
                } else {
                    report.add_error(CompilerError::LexerError(LexerError::InvalidNumberFormat {
                        invalid_string: number,
                        span: Span::new(
                            file_id,
                            start_pos,
                            chars.current_pos,
                            start_line,
                            start_col,
                        ),
                        suggestions: vec!["Your number may be too large or too small.".to_string()],
                    }));
                }
                Err(())
            }
        }
    } else {
        match number.parse::<i64>() {
            Ok(value) => Ok(Token {
                token: TokenType::IntegerLiteral(value),
                span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
            }),
            Err(_) => {
                report.add_error(CompilerError::LexerError(LexerError::InvalidNumberFormat {
                    invalid_string: number,
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                    suggestions: vec!["Your number may be too large or too small.".to_string()],
                }));
                Err(())
            }
        }
    }
}

fn tokenize_operator_or_symbol(
    chars: &mut CharStream,
    file_id: usize,
    start_pos: usize,
    start_line: usize,
    start_col: usize,
    current_char: char,
    report: &mut CompilationReport,
) -> Result<Option<Token>, ()> {
    let _ = report;
    chars.next_char(); // Consume the current character
    match current_char {
        '!' => {
            if let Some(&'=') = chars.peek_char() {
                chars.next_char(); // Consume '='
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::NotEqual),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else if let Some(&'!') = chars.peek_char() {
                chars.next_char(); // Consume '!'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::DoubleExclamation),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::LogicalNot),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '?' => {
            if let Some(&'?') = chars.peek_char() {
                chars.next_char(); // Consume '?'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::DoubleQuestion),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::QuestionMark),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '=' => {
            if let Some(&'=') = chars.peek_char() {
                chars.next_char(); // Consume '='
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::Equal),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::Assign),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '<' => {
            if let Some(&'=') = chars.peek_char() {
                chars.next_char(); // Consume '='
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::LessOrEqual),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else if let Some(&'<') = chars.peek_char() {
                chars.next_char(); // Consume '<'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::ShiftLeft),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::LessThan),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '>' => {
            if let Some(&'=') = chars.peek_char() {
                chars.next_char(); // Consume '='
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::GreaterOrEqual),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else if let Some(&'>') = chars.peek_char() {
                chars.next_char(); // Consume '<'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::ShiftRight),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::GreaterThan),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '&' => {
            if let Some(&'&') = chars.peek_char() {
                chars.next_char(); // Consume '&'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::And),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::BitwiseAnd),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '|' => {
            if let Some(&'|') = chars.peek_char() {
                chars.next_char(); // Consume '|'
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::Or),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::BitwiseOr),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '-' => {
            if let Some(&'>') = chars.peek_char() {
                chars.next_char(); // Consume '>'
                Ok(Some(Token {
                    token: TokenType::Symbol(Symbol::Arrow),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Operator(Operator::Subtract),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        '.' => {
            if let Some(&'.') = chars.peek_char() {
                chars.next_char(); // Consume second '.'
                if let Some(&'=') = chars.peek_char() {
                    chars.next_char(); // Consume '='
                    Ok(Some(Token {
                        token: TokenType::Operator(Operator::RangeInclusive),
                        span: Span::new(
                            file_id,
                            start_pos,
                            chars.current_pos,
                            start_line,
                            start_col,
                        ),
                    }))
                } else {
                    Ok(Some(Token {
                        token: TokenType::Operator(Operator::RangeExclusive),
                        span: Span::new(
                            file_id,
                            start_pos,
                            chars.current_pos,
                            start_line,
                            start_col,
                        ),
                    }))
                }
            } else {
                Ok(Some(Token {
                    token: TokenType::Symbol(Symbol::Dot),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        ':' => {
            if let Some(&':') = chars.peek_char() {
                chars.next_char(); // Consume second ':'
                Ok(Some(Token {
                    token: TokenType::Symbol(Symbol::DoubleColon),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            } else {
                Ok(Some(Token {
                    token: TokenType::Symbol(Symbol::Colon),
                    span: Span::new(file_id, start_pos, chars.current_pos, start_line, start_col),
                }))
            }
        }
        _ => {
            // Handle other operators if necessary
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use quiklang_common::FileStore;

    use super::*;

    /// Helper function to create a test file and add it to the FileStore.
    fn setup_file_store(source: &str, file_name: &str, file_store: &mut FileStore) -> usize {
        file_store.add_file(file_name.to_string(), source.to_string())
    }

    /// Helper function to assert tokens.
    fn assert_tokens(actual: &[Token], expected: &[TokenType], _file_id: usize) {
        assert_eq!(actual.len(), expected.len(), "Number of tokens mismatch.");
        for (i, (act, exp)) in actual.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                &act.token, exp,
                "Token mismatch at index {}: expected {:?}, got {:?}",
                i, exp, act.token
            );
        }
    }

    /// Test case for successful tokenization of a simple function.
    #[test]
    fn test_simple_function() {
        let source = r#"
            fun main() {
                let x = 42;
                let y = "hello";
            }
        "#;
        let file_name = "test_simple_function.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        // Define expected tokens
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Fun),
            TokenType::Identifier("main".to_string()),
            TokenType::Symbol(Symbol::LeftParen),
            TokenType::Symbol(Symbol::RightParen),
            TokenType::Symbol(Symbol::LeftBrace),
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("x".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::IntegerLiteral(42),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("y".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::StringLiteral("hello".to_string()),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::Symbol(Symbol::RightBrace),
            TokenType::EOF,
        ];

        // Assert no errors
        assert!(
            !report.has_errors(),
            "Unexpected lexer errors: {:?}",
            report.errors
        );

        // Assert tokens
        assert_tokens(&tokens, &expected_tokens, file_id);
    }

    /// Test case for handling unrecognized characters.
    #[test]
    fn test_unrecognized_character() {
        let source = r#"
            let x = 10;
            let y = $invalid;
        "#;
        let file_name = "test_unrecognized_character.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        // Define expected tokens (excluding the unrecognized character)
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("x".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::IntegerLiteral(10),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("y".to_string()),
            TokenType::Operator(Operator::Assign),
            // The '$' character is unrecognized and should be skipped with an error
            TokenType::Identifier("invalid".to_string()),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::EOF,
        ];

        // Assert that one error was reported
        assert_eq!(report.errors.len(), 1, "Expected one lexer error.");

        if let CompilerError::LexerError(LexerError::UnrecognizedCharacter {
            character,
            span,
            suggestion,
        }) = &report.errors[0]
        {
            assert_eq!(*character, '$');
            assert_eq!(span.line, 3);
            assert_eq!(span.col, 21);
            assert_eq!(*suggestion, Vec::<String>::new());
        } else {
            panic!("Expected UnrecognizedCharacter lexer error.");
        }

        // Assert tokens (the lexer skips the '$' and continues)
        assert_tokens(&tokens, &expected_tokens, file_id);
    }

    /// Test case for handling unterminated string literals.
    #[test]
    fn test_unterminated_string_literal() {
        let source = r#"
            let message = "Hello, World!;"#;
        let file_name = "test_unterminated_string_literal.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        // Define expected tokens (the string is unterminated, so it should not be present)
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("message".to_string()),
            TokenType::Operator(Operator::Assign),
            // The unterminated string literal should trigger an error and skip
            // Depending on implementation, it might not include a StringLiteral token
            // We'll assume it does not include the invalid string
            // Alternatively, adjust according to your lexer's behavior
            // For this example, we'll assume it does not include a StringLiteral
            // and proceeds to EOF
            TokenType::EOF,
        ];

        // Assert that one error was reported
        assert_eq!(report.errors.len(), 1, "Expected one lexer error.");

        if let CompilerError::LexerError(LexerError::UnterminatedStringLiteral { span, .. }) =
            &report.errors[0]
        {
            assert_eq!(span.line, 2);
            assert_eq!(span.col, 27);
        } else {
            panic!("Expected UnterminatedStringLiteral lexer error.");
        }

        // Assert tokens
        assert_tokens(&tokens, &expected_tokens, file_id);
    }

    /// Test case for handling multi-line comments.
    #[test]
    fn test_multi_line_comments() {
        let source = r#"
            /* This is a
               multi-line comment */
            let x = 100;
        "#;
        let file_name = "test_multi_line_comments.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        // Define expected tokens
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("x".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::IntegerLiteral(100),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::EOF,
        ];

        // Assert no errors
        assert!(
            !report.has_errors(),
            "Unexpected lexer errors: {:?}",
            report.errors
        );

        // Assert tokens
        assert_tokens(&tokens, &expected_tokens, file_id);
    }

    /// Test case for handling invalid number formats.
    #[test]
    fn test_invalid_number_format() {
        let source = r#"
            let x = 12.34.56;
        "#;
        let file_name = "test_invalid_number_format.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        // Define expected tokens (the invalid number is split or causes an error)
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("x".to_string()),
            TokenType::Operator(Operator::Assign),
            // The invalid number should trigger an error and skip the invalid part
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::EOF,
        ];

        // Assert that one error was reported for the second dot
        assert_eq!(report.errors.len(), 1, "Expected one lexer error.");

        if let CompilerError::LexerError(LexerError::InvalidNumberFormat {
            invalid_string,
            span,
            ..
        }) = &report.errors[0]
        {
            assert_eq!(invalid_string, "12.34.56");
            assert_eq!(span.line, 2);
            assert_eq!(span.col, 21);
        } else {
            panic!("Expected InvalidNumberFormat lexer error.");
        }

        // Assert tokens
        assert_tokens(&tokens, &expected_tokens, file_id);
    }

    /// Test case for handling string interpolation (assuming it's partially implemented).
    #[test]
    fn test_string_interpolation() {
        let source = r#"
            let name = "Alice";
            let greeting = "Hello, ${name}!";
        "#;
        let file_name = "test_string_interpolation.quik";
        let mut file_store = FileStore::default();
        let file_id = setup_file_store(source, file_name, &mut file_store);
        let mut report = CompilationReport {
            file_store: file_store.clone(),
            ..Default::default()
        };

        let tokens = tokenize(
            &file_store.get_file(file_id).unwrap().source,
            file_id,
            &mut report,
        )
        .tokens;

        println!("{:#?}", tokens);

        // Define expected tokens (assuming string interpolation is not fully implemented)
        let expected_tokens = vec![
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("name".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::StringLiteral("Alice".to_string()),
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::Keyword(Keyword::Let),
            TokenType::Identifier("greeting".to_string()),
            TokenType::Operator(Operator::Assign),
            TokenType::ComplexStringStart,
            TokenType::StringLiteral("Hello, ".to_string()),
            TokenType::StringInterpolationStart,
            TokenType::Identifier("name".to_string()),
            TokenType::StringInterpolationEnd,
            TokenType::StringLiteral("!".to_string()),
            TokenType::ComplexStringEnd,
            TokenType::Symbol(Symbol::Semicolon),
            TokenType::EOF,
        ];

        // Assert tokens
        assert_tokens(&tokens, &expected_tokens, file_id);
    }
}
