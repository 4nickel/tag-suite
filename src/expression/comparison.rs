use super::{import::*, error::{Error as E}};
use regex::Regex;

const VARIABLE: &'static str = r"[a-zA-Z0-9\.-_]+";
const OPERATOR: &'static str = r"<=|>=|==|<|>";
const NUMBER: &'static str = r"[0-9+]";

lazy_static! {
    static ref COMPARISON: Regex = {
        Regex::new(&format!(r"({}) *({}) *({})", VARIABLE, OPERATOR, NUMBER))
            .expect("failed to compile regex")
    };
}

/// A comparison
#[derive(Copy, Clone)]
enum Comparator { Eq, Ne, Lt, Le, Ge, Gt }

/// A compiled Comparison expression
pub struct Comparison {
    cmp: Comparator,
    lhs: String,
    rhs: usize,
}

impl Comparison {

    /// Parsing these simple comparison expressions is easily done via regex
    pub fn capture<'c>(expression: &'c str) -> Res<(&'c str, &'c str, &'c str)> {
        let cap = match COMPARISON.captures(expression) {
            Some(cap) => cap,
            None => return Err(E::FailedCapture { expression: expression.into() }.into())
        };
        let get = |n| {
            cap.get(n).map(|s| s.as_str())
                .ok_or(E::FailedCapture { expression: expression.into() })
        };
        let lhs = get(1)?;
        let cmp = get(2)?;
        let rhs = get(3)?;
        Ok((lhs, cmp, rhs))
    }

    /// Create a new compiled Comparison instance from an expression
    pub fn new(expression: &str) -> Res<Self> {
        let (lhs, cmp, rhs) = Self::capture(expression)?;
        Ok(Self {
            lhs: lhs.into(),
            cmp: Comparables::operator(cmp)?,
            rhs: Comparables::literal(rhs)?,
        })
    }
}

/// A collection of variables for comparison
#[derive(Debug)]
pub struct Comparables {
    variables: HashMap<&'static str, usize>,
}

impl Comparables {

    /// Create a new collection of Comparables from the given Map
    pub fn from_map(variables: HashMap<&'static str, usize>) -> Self {
        Self { variables }
    }

    /// Interpret a variable
    fn variable(&self, variable: &str) -> Res<usize> {
        match self.variables.get(variable) {
            Some(number) => Ok(*number),
            None => Err(E::UnknownVariable { variable: variable.into() }.into())
        }
    }

    /// Interpret a literal
    fn literal(literal: &str) -> Res<usize> {
        Ok(literal.parse::<usize>()?)
    }

    /// Interpret the operator string
    fn operator(operator: &str) -> Res<Comparator> {
        match operator {
            "==" => Ok(Comparator::Eq),
            "!=" => Ok(Comparator::Ne),
            ">=" => Ok(Comparator::Ge),
            "<=" => Ok(Comparator::Le),
            ">" => Ok(Comparator::Gt),
            "<" => Ok(Comparator::Lt),
            _ => Err(E::UnknownOperator { operator: operator.into() }.into())
        }
    }

    /// Perform the comparison
    fn operation(lhs: usize, cmp: Comparator, rhs: usize) -> bool {
        match cmp {
            Comparator::Eq => lhs == rhs,
            Comparator::Ne => lhs != rhs,
            Comparator::Ge => lhs >= rhs,
            Comparator::Le => lhs <= rhs,
            Comparator::Gt => lhs > rhs,
            Comparator::Lt => lhs < rhs,
        }
    }

    /// Evaluate the comparison
    pub fn evaluate(&self, comparison: &Comparison) -> Res<bool> {
        Ok(Self::operation(self.variable(&comparison.lhs)?, comparison.cmp, comparison.rhs))
    }
}

/// Create Comparables from a Map of variables
impl From<HashMap<&'static str, usize>> for Comparables {
    fn from(variables: HashMap<&'static str, usize>) -> Self {
        Self::from_map(variables)
    }
}

/// A type with Comparable Parameters
pub trait Parameters {
    fn parameters(&self) -> HashMap<&'static str, usize>;
}
