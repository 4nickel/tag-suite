use super::{import::*, error::{Error as E}};

/// A Token that yields a value
#[derive(Debug, PartialEq, Eq)]
pub enum ValueToken {
    Block,
    Expr(char, String),
}

/// A Token that represents a unary operator
#[derive(Debug, PartialEq, Eq)]
pub enum UnaryToken {
    Not,
}

/// A Token that represents a binary operator
#[derive(Debug, PartialEq, Eq)]
pub enum BinaryToken {
    And,
    Or,
}

/// A Token that represents a closing sequence
#[derive(Debug, PartialEq, Eq)]
pub enum CloseToken {
    Block,
}

/// A single Token
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
    Value(ValueToken),
    Unary(UnaryToken),
    Binary(BinaryToken),
    Close(CloseToken),
}

/// All we need for tokenizing is a peekable iterator
use std::iter::Peekable;
pub struct Tokenizer<T: Iterator> {
    pub stream: Peekable<T>,
}

impl<T: Iterator<Item=char>> Tokenizer<T> {

    /// Create a new tokenizer from a char iterator
    pub fn new(stream: T) -> Self {
        Self { stream: stream.peekable() }
    }

    /// Helper function for grabbing chars until we encounter
    /// an unescaped 'end' character.
    fn take_delimited(&mut self, end: char, esc: char) -> Res<String> {
        let mut e = false;
        let mut cur = esc;
        let take = self.stream.by_ref().take_while(|c| {
            cur = *c;
            if      cur == esc && !e { e = true; true }
            else if cur == end && !e { false }
            else                     { e = false; true }
        }).filter(|c| {
            *c != esc
        }).collect();
        if cur == end {
            Ok(take)
        } else {
            Err(E::UnclosedDelimiter { delimiter: end }.into())
        }
    }

    fn take_expr(&mut self, modifier: char, brk: (char, char), esc: char) -> Res<Token> {
        match self.stream.next() {
            Some(c) => {
                match c == brk.0 {
                    true => self.take_delimited(brk.1, esc).map(|s| {
                        Token::Value(ValueToken::Expr(modifier, s))
                    }),
                    false => Err(E::UnexpectedCharacter {
                        expected: brk.0, found: c
                    }.into()),
                }
            } _ => { Err(E::UnexpectedEof { }.into()) }
        }
    }
}

impl<T: Iterator<Item=char>> Iterator for Tokenizer<T> {
    type Item=Res<Token>;

    /// Yield the next token
    fn next(&mut self) -> Option<Self::Item> {
        const GLYPH: char = '.'; // any non-whitespace char
        const ESC: char = '\\';

        while self.stream.peek().unwrap_or(&GLYPH).is_whitespace() { self.stream.next(); }
        match self.stream.peek() {
            Some('(') => { self.stream.next(); Some(Ok(Token::Value(ValueToken::Block))) },
            Some(')') => { self.stream.next(); Some(Ok(Token::Close(CloseToken::Block))) },
            Some('!') => { self.stream.next(); Some(Ok(Token::Unary(UnaryToken::Not))) },
            Some('&') => { self.stream.next(); Some(Ok(Token::Binary(BinaryToken::And))) },
            Some('|') => { self.stream.next(); Some(Ok(Token::Binary(BinaryToken::Or))) },
            Some('?') => { self.stream.next(); Some(self.take_expr('?', ('[', ']'), ESC)) },
            Some('$') => { self.stream.next(); Some(self.take_expr('$', ('[', ']'), ESC)) },
            Some('=') => { self.stream.next(); Some(self.take_expr('=', ('[', ']'), ESC)) },
            Some('[') => { Some(self.take_expr('=', ('[', ']'), ESC)) }, // shorthand
            Some(c) => Some(Err(E::InvalidCharacter { c: *c }.into())),
            None => None,
        }
    }
}

#[cfg(test)]
mod suite {

    use super::*;

    fn u(t: Option<Res<Token>>) -> Token { t.unwrap().unwrap() }

    #[test]
    fn check_empty_string() {
        let mut t = Tokenizer::new("".chars());
        assert!(t.next().is_none());
        let mut t = Tokenizer::new("      ".chars());
        assert!(t.next().is_none());
        let mut t = Tokenizer::new("\t\n\t".chars());
        assert!(t.next().is_none());
    }

    #[test]
    fn check_exp() {
        let mut t = Tokenizer::new("[!!!]".chars());
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "!!!".to_string())));
        assert!(t.next().is_none());
    }

    #[test]
    fn check_and() {
        let mut t = Tokenizer::new("=[!!!] & =[???]".chars());
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "!!!".to_string())));
        assert_eq!(u(t.next()), Token::Binary(BinaryToken::And));
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "???".to_string())));
        assert!(t.next().is_none());
    }

    #[test]
    fn check_or() {
        let mut t = Tokenizer::new("=[!!!] | =[???]".chars());
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "!!!".to_string())));
        assert_eq!(u(t.next()), Token::Binary(BinaryToken::Or));
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "???".to_string())));
        assert!(t.next().is_none());
    }

    #[test]
    fn check_not() {
        let mut t = Tokenizer::new("!=[???]".chars());
        assert_eq!(u(t.next()), Token::Unary(UnaryToken::Not));
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "???".to_string())));
        assert!(t.next().is_none());
    }

    #[test]
    fn check_block() {
        let mut t = Tokenizer::new("([???])".chars());
        assert_eq!(u(t.next()), Token::Value(ValueToken::Block));
        assert_eq!(u(t.next()), Token::Value(ValueToken::Expr('=', "???".to_string())));
        assert_eq!(u(t.next()), Token::Close(CloseToken::Block));
        assert!(t.next().is_none());
    }

    #[test]
    #[should_panic(expected = "InvalidCharacter")]
    fn check_invalid_character() {
        let mut t = Tokenizer::new("foo".chars());
        u(t.next());
    }

    #[test]
    #[should_panic(expected = "UnclosedDelimiter")]
    fn check_unclosed_delimiter() {
        let mut t = Tokenizer::new("=[foo".chars());
        u(t.next());
    }

    #[test]
    #[should_panic(expected = "UnexpectedEof")]
    fn check_unexpected_eof() {
        let mut t = Tokenizer::new("=".chars());
        u(t.next()); u(t.next());
    }
}

#[cfg(test)]
pub mod benches {

    use crate::expression::Tokenizer;
    use test::Bencher;

    fn generate_expression(n: usize, op: char) -> String {
        let mut s = String::from("=[...]");
        for _ in 0..n {
            s.push_str(&format!("{} =[...]", op));
        }
        s
    }

    #[bench]
    fn bench_1000_ands(b: &mut Bencher) {
        let s = test::black_box(generate_expression(1000, '&'));
        b.iter(|| {
            let mut tokens = Tokenizer::new(s.chars());
            let mut n = 0;
            while let Some(_) = tokens.next() { n += 1; }
            n
        });
    }

    #[bench]
    fn bench_1000_ors(b: &mut Bencher) {
        let s = test::black_box(generate_expression(1000, '|'));
        b.iter(|| {
            let mut tokens = Tokenizer::new(s.chars());
            let mut n = 0;
            while let Some(_) = tokens.next() { n += 1; }
            n
        });
    }
}
