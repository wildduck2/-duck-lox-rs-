use crate::{
  logger::Log,
  lox::{
    types::{CompilerError, LoxError},
    Lox,
  },
  scanner::Scanner,
};

use super::token::{
  types::{Literal, TokenType},
  Token,
};

impl Scanner {
  /// Scans the entire source string, producing tokens.
  ///
  /// Iterates through the source, advancing one character at a time,
  /// matching characters to token types, and pushing tokens onto `self.tokens`.
  /// Handles single-character tokens, two-character operators, whitespace, and line counting.
  /// At the end, pushes an EOF token.
  pub fn scan_tokens(&mut self, lox: &mut Lox) -> () {
    while !self.is_at_end() {
      self.start = self.current;
      let c = self.advance();

      let token_type = match c {
        // Handling Grouping chars.
        '(' => Some(TokenType::LeftParen),
        ')' => Some(TokenType::RightParen),
        '{' => Some(TokenType::LeftBrace),
        '}' => Some(TokenType::RightBrace),

        // Handle Dot and the decimals starting with .
        '.' => {
          if let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
              self.advance();

              while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                  self.advance();
                  continue;
                }
                break;
              }
              Some(TokenType::Number)
            } else {
              Some(TokenType::Dot)
            }
          } else {
            Some(TokenType::Number)
          }
        },

        // Handle Mathematical Operators
        '-' => Some(TokenType::Minus),
        '*' => Some(TokenType::Star),
        '+' => Some(TokenType::Plus),
        '%' => Some(TokenType::Modulus),
        '/' => {
          if self.match_char('/') {
            // Handle single-line comment
            while let Some(ch) = self.peek() {
              if ch == '\n' {
                break;
              }
              self.advance();
            }
            Some(TokenType::Comment)
          } else if self.match_char('*') {
            // Handle multi-line comment
            while !self.is_at_end() {
              if self.peek() == Some('*') && self.peek_next() == Some('/') {
                // Consume the '*' and '/'
                self.advance();
                self.advance();
                break;
              }
              let ch = self.advance();
              if ch == '\n' {
                self.line += 1;
                self.column = 0;
              }
            }

            if self.is_at_end() {
              // Unterminated multi-line comment
              lox.has_error = true;
              lox.log_language(
                Log::Error(LoxError::CompileError(CompilerError::SyntaxError)),
                "Unterminated multi-line comment",
                &format!("line: {}:{}", self.line, self.column),
              );
            }

            Some(TokenType::Comment)
          } else {
            // It's just a '/'
            Some(TokenType::Divide)
          }
        },

        // Handle end of statement terminator
        ';' => {
          if self.match_char('\n') && self.tokens[self.tokens.len() - 1].lexeme == String::from(';')
          {
            // Getting the the rest of the line to show it in the error
            let snippet: String = self.source[self.current..]
              .chars()
              .take_while(|&c| c != '\n')
              .collect();

            while let Some(ch) = self.peek() {
              if ch == '\n' {
                break;
              }
              self.advance();
            }

            lox.has_error = true;
            Lox::log_language(
              lox,
              Log::Error(LoxError::CompileError(CompilerError::SyntaxError)),
              &format!("Expect ';' after expression. Found ';{}' instead.", snippet),
              &format!("{}:{}", self.line, self.column),
            );
            Lox::log_language(
              lox,
              Log::Info,
              &format!(
                "Please make sure the end of your expression is followed by a single semicolon.",
              ),
              &format!("{}:{}", self.line, self.column),
            );

            None
          } else {
            Some(TokenType::Semicolon)
          }
        },

        // Handle Comma sperator
        ',' => Some(TokenType::Comma),

        // Handle possible two-character tokens (e.g., !=, ==, <=, >=)
        '!' => {
          if self.match_char('=') {
            self.current += 1;
            Some(TokenType::BangEqual)
          } else {
            Some(TokenType::Bang)
          }
        },
        '=' => {
          if self.match_char('=') {
            self.current += 1;
            Some(TokenType::EqualEqual)
          } else {
            Some(TokenType::Equal)
          }
        },
        '<' => {
          if self.match_char('=') {
            self.current += 1;
            Some(TokenType::LessEqual)
          } else {
            Some(TokenType::Less)
          }
        },
        '>' => {
          if self.match_char('=') {
            self.current += 1;
            Some(TokenType::GreaterEqual)
          } else {
            Some(TokenType::Greater)
          }
        },

        // Ignore whitespace characters
        ' ' | '\r' | '\t' => None,

        // Handle strings
        // TODO: handle the numbers inside of string
        '"' | '\'' | '`' => {
          let mut s = String::new();
          while let Some(next) = self.peek() {
            if self.is_at_end() {
              lox.has_error = true;
              break;
            }
            if next == '\n' {
              self.line += 1;
              self.advance();
              continue;
            }

            if next == '"' || next == '\'' || next == '`' {
              self.advance();
              break;
            }
            s.push(next);
            self.advance();
            continue;
          }

          // Check if the string is not valid of not and throw error in the language
          if lox.has_error {
            lox.log_language(
              Log::Error(LoxError::CompileError(CompilerError::SyntaxError)),
              &format!(
                "Unexpected character: `{}` String must have pairs of `{}`",
                c, c
              ),
              &format!("line: {}:{}", self.line - 1, self.column + 1),
            );
            None
          } else {
            Some(TokenType::String)
          }
        },

        // Newline increments line counter
        '\n' => {
          self.line += 1;
          self.column = 0;
          None
        },

        // Handle identifiers and keywords
        'a'..='z' | 'A'..='Z' | '_' => Some(self.tokenize_identifier()),

        // Handle integers and decimals
        '0'..='9' => Some(self.tokenize_number()),

        // Default case: unrecognized characters
        _ => {
          lox.has_error = true;
          lox.log_language(
            Log::Error(LoxError::CompileError(CompilerError::SyntaxError)),
            &format!("Unexpected character: {}", c),
            &format!("line: {}:{}", self.line, self.column + 1),
          );
          None
        },
      };

      // If a token type was matched, create and push a new token with the current lexeme
      if let Some(ttype) = token_type {
        let lexeme = self.current_lexeme().to_string();

        // Ignore the comments token.
        match ttype {
          TokenType::Comment => {
            print!("Comment: {}", lexeme);
            ()
          },
          // Getting the string value only
          TokenType::String => self.add_token(ttype, lexeme[1..lexeme.len() - 1].to_string()),
          // Handling the `0` before and after a `.` decimal
          TokenType::Number => {
            let number = if lexeme.ends_with('.') {
              format!(
                "{}",
                lexeme.split('.').nth(0).expect("Failed to get the number")
              )
            } else if lexeme.starts_with('.') {
              format!("{}{}", "0", lexeme)
            } else {
              lexeme
            };
            self.add_token(ttype, number)
          },
          _ => self.add_token(ttype, lexeme),
        }
      }
    }

    // Add EOF token at the end of scanning
    self.add_token(TokenType::Eof, "EOF".to_string());
  }

  fn tokenize_number(&mut self) -> TokenType {
    while let Some(c) = self.peek() {
      if c.is_ascii_digit() {
        self.advance();
      } else {
        if self.match_char('.') {
          self.advance();
        } else {
          break;
        }
      }
    }
    TokenType::Number
  }

  fn tokenize_identifier(&mut self) -> TokenType {
    /*
    Consume the rest of the identifier: letters, digits, or underscores
    this is an application of `maximal munch` principle:

    `When two lexical grammar rules can both match a chunk of code that the scanner is
    looking at, whichever one matches the most characters wins.`

    By doing this i prevent the collision of "identifires" and "keywords"

    @example
    ```rs
      var or_not = true;
    ```
    This var have a "or" which is a keyword in the language by consuming all the
    "ascii_alphanumeric" i get all the chars as keywords and match them, and by doing this
    i avoid the collision and yes, you're happy.
    */
    while let Some(c) = self.peek() {
      if c.is_ascii_alphanumeric() || c == '_' {
        self.advance();
      } else {
        break;
      }
    }

    let identifier = self.current_lexeme();

    // Match keywords here; add more keywords as needed
    match identifier {
      "var" => TokenType::Var,
      "fun" => TokenType::Fun,
      "return" => TokenType::Return,
      "if" => TokenType::If,
      "else" => TokenType::Else,
      "for" => TokenType::For,
      "while" => TokenType::While,
      "print" => TokenType::Print,
      "break" => TokenType::Break,
      "continue" => TokenType::Continue,
      "class" => TokenType::Class,
      "this" => TokenType::This,
      "true" => TokenType::True,
      "false" => TokenType::False,
      "nil" => TokenType::Nil,
      "or" => TokenType::Or,
      "and" => TokenType::And,
      "super" => TokenType::Super,
      _ => TokenType::Identifier,
    }
  }

  /// Returns true if the scanner has reached the end of the source input.
  ///
  /// This is based on the byte index `current` compared to the total byte length of `source`.
  ///
  /// Q: Why do not you use the '\0' here like "C" and "Java"
  /// A: Simply enough because the way "Rust" handles the strings is different, there's not '\0'
  /// char at the end of the string.
  fn is_at_end(&self) -> bool {
    self.current >= self.source.len()
  }

  /// Returns the next character without advancing the scanner.
  ///
  /// Returns `None` if at the end of input.
  fn peek(&self) -> Option<char> {
    if self.is_at_end() {
      Some('\0')
    } else {
      Some(self.source[self.current..].chars().next().unwrap())
    }
  }

  /// Returns the next character without advancing the scanner.
  ///
  /// Returns `None` if at the end of input.
  fn peek_next(&self) -> Option<char> {
    if self.is_at_end() {
      Some('\0')
    } else {
      Some(self.source[self.current + 1..].chars().next().unwrap())
    }
  }

  /// Checks if the next character matches the expected character.
  ///
  /// If it matches, returns `true`.
  /// Otherwise, returns `false`.
  fn match_char(&mut self, expected: char) -> bool {
    if self.is_at_end() {
      return false;
    }
    if self.source[self.current..].chars().next().unwrap() != expected {
      return false;
    }
    true
  }

  /// Consumes the next character in the source and advances the scanner.
  ///
  /// Returns the character and moves the `current` byte index forward by the UTF-8 length of the character.
  fn advance(&mut self) -> char {
    if self.is_at_end() {
      return '\0';
    }

    let ch = self.source[self.current..].chars().next().unwrap();
    self.current += ch.len_utf8();
    self.column += 1;
    ch
  }

  /// Returns the current lexeme as a slice of the source string.
  ///
  /// The lexeme spans from the `start` byte index to the `current` byte index.
  fn current_lexeme(&mut self) -> &str {
    let lexeme = &self.source[self.start..self.current];
    lexeme
  }

  /// Helper function to add a token to the token list.
  ///
  /// Takes a vector of tokens, token type, and lexeme string, creates a new `Token`
  /// with a default `Literal::Nil` value and current line number, then pushes it.
  fn add_token(&mut self, token_type: TokenType, lexeme: String) -> () {
    let literal = Scanner::get_literal_type(&token_type);
    self.tokens.push(Token::new(
      token_type,
      lexeme,
      literal,
      self.line,
      self.column + 1,
    ));
  }

  /// Helper function to get the `Literal` that corresponds to the `TokenType`
  fn get_literal_type(token_type: &TokenType) -> Literal {
    match token_type {
      TokenType::Number => Literal::Number,
      TokenType::String => Literal::String,
      TokenType::True => Literal::Boolean,
      TokenType::False => Literal::Boolean,
      _ => Literal::Nil,
    }
  }
}
