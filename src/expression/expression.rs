use super::{import::*, Tokenizer, Parser, Ast, error::{Error as E}};

/// A compiled expression
#[derive(Debug, Clone)]
pub struct Expression {
    raw: String,
    ast: Ast,
}

impl Expression {

    /// Create a new compiled Expression
    pub fn new(exp: String) -> Res<Self> {
        let expression = format!("({})", exp);
        let tok = Tokenizer::new(expression.chars().into_iter());
        let ast = Parser::new(tok.into_iter()).parse()?.ok_or(E::EmptyExpression { })?;
        //trace!("ast: {:?}", ast);
        Ok(Self { raw: expression, ast: ast, })
    }

    /// Return this expressions raw string
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Return this abstract syntax tree
    pub fn as_ast(&self) -> &Ast {
        &self.ast
    }

    /// Convenience functions for compiling pairs of query and filter expressions
    pub fn compile(q: Option<String>, f: Option<String>) -> Res<(Option<Self>, Option<Self>)> {
        let compile = |o: Option<String>| o.map(|s| Self::new(s)).transpose() ;
        Ok(( compile(q)?, compile(f)? ))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    #[should_panic(expected = "UnexpectedEof")]
    fn check_disallow_empty_expression() {
        Expression::new(String::new()).unwrap();
    }
}
