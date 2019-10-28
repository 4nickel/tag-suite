use super::{import::*, Token, ValueToken, BinaryToken, UnaryToken, error::{Error as E}};

/// An operator node in the Ast
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    Not(Box<Ast>),
    And(Box<Ast>, Box<Ast>),
    Or(Box<Ast>, Box<Ast>),
}

/// A single node in the Ast
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ast {
    Expr(char, String),
    Operation(Operator),
}

/// Parses our custom expressions
pub struct Parser<T: Iterator> {
    stream: T,
}

impl<T: Iterator<Item=Res<Token>>> Parser<T> {

    /// Construct a new Parser for a stream of Tokens
    pub fn new(stream: T) -> Self {
        Self { stream: stream }
    }

    /// Parse a binary operation from the BinaryToken
    fn binary(&mut self, operator: BinaryToken, lhs: Ast) -> Res<Ast> {
        let rhs = self.parse()?.ok_or(E::UnexpectedEof { })?;
        match operator {
            BinaryToken::And => Ok(Ast::Operation(Operator::And(box lhs, box rhs))),
            BinaryToken::Or => Ok(Ast::Operation(Operator::Or(box lhs, box rhs))),
        }
    }

    /// Parse a unary operation from the UnaryToken
    fn unary(&mut self, operator: UnaryToken) -> Res<Ast> {
        let val = self.parse()?.ok_or(E::UnexpectedEof { })?;
        match operator {
            UnaryToken::Not => Ok(Ast::Operation(Operator::Not(box val))),
        }
    }

    /// Expect this node to be present
    fn required(opt: Option<Ast>) -> Res<Ast> {
        opt.ok_or(E::MissingValue { }.into())
    }

    /// Parse the stream of Tokens into an Ast
    pub fn parse(&mut self) -> Res<Option<Ast>> {
        let mut node = None;
        while let Some(token) = self.stream.next() {
            node = match token? {
                Token::Unary(t)  => { Some(self.unary(t)?) },
                Token::Binary(t) => { Some(self.binary(t, Self::required(node)?)?) },
                Token::Value(t)  => {
                    match t {
                        ValueToken::Block => { Some(self.parse()?.ok_or(E::UnexpectedEof { })?) },
                        ValueToken::Expr(c, s) => { return Ok(Some(Ast::Expr(c, s))) }
                    }
                }
                Token::Close(_)  => { return Ok(node); }
            }
        }
        Ok(node)
    }
}

#[cfg(test)]
mod suite {

    use crate::expression::Tokenizer;
    use super::*;

    #[test]
    fn check_and() {
        let e = Parser::new(Tokenizer::new("=[...] & =[...]".chars())).parse().unwrap().unwrap();
        if let Ast::Operation(node) = e { if let Operator::And(_, _) = node { return } }
        //panic!()
    }

    #[test]
    fn check_or() {
        let e = Parser::new(Tokenizer::new("=[...] | =[...]".chars())).parse().unwrap().unwrap();
        if let Ast::Operation(node) = e { if let Operator::Or(_, _) = node { return } }
        //panic!()
    }

    #[test]
    fn check_not() {
        let e = Parser::new(Tokenizer::new("!=[...]".chars())).parse().unwrap().unwrap();
        if let Ast::Operation(node) = e { if let Operator::Not(_) = node { return } }
        panic!()
    }

    #[test]
    fn check_block() {
        let e = Parser::new(Tokenizer::new("(=[...])".chars())).parse().unwrap().unwrap();
        if let Ast::Expr(_, _) = e { return }
        panic!()
    }

    #[test]
    #[should_panic(expected = "UnexpectedEof")]
    fn check_empty_block() {
        Parser::new(Tokenizer::new("()".chars())).parse().unwrap();
    }

    #[test]
    #[should_panic(expected = "MissingValue")]
    fn check_leading_binary_operator() {
        Parser::new(Tokenizer::new("& [...]".chars())).parse().unwrap();
    }

    // #[test]
    // #[should_panic(expected = "UnexpectedEof")]
    // fn check_trailing_binary_operator() {
    //     let ast = Parser::new(Tokenizer::new("[...] &".chars())).parse().unwrap();
    //     println!("{:?}", ast);
    // }
}

#[cfg(test)]
mod benches {

    use crate::expression::{Tokenizer};
    use super::*;
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
        let expression = test::black_box(generate_expression(1000, '&'));
        b.iter(|| {
            Parser::new(Tokenizer::new(expression.chars())).parse()
        });
    }

    #[bench]
    fn bench_1000_ors(b: &mut Bencher) {
        let expression = test::black_box(generate_expression(1000, '|'));
        b.iter(|| {
            Parser::new(Tokenizer::new(expression.chars())).parse()
        });
    }
}
