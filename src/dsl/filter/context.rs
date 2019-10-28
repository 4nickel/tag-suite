use crate::{expression::{Comparison}};
use super::import::*;

/// Caches compiled regexes and comparison expressions
pub struct Context {
    regexes: HashMap<String, Regex>,
    comparisons: HashMap<String, Comparison>,
}

impl Context {

    /// Create a new Context instance
    pub fn new() -> Self {
        Self { regexes: HashMap::new(), comparisons: HashMap::new() }
    }

    /// Get a cached regex
    pub fn regex<'a>(&'a mut self, exp: &str) -> Res<&'a Regex> {
        if !self.regexes.contains_key(exp) {
            self.regexes.insert(exp.into(), Regex::new(exp)?);
        }
        Ok(self.regexes.get(exp).unwrap())
    }

    /// Get a cached comparison
    pub fn comparison<'a>(&'a mut self, exp: &str) -> Res<&'a Comparison> {
        if !self.comparisons.contains_key(exp) {
            self.comparisons.insert(exp.into(), Comparison::new(exp)?);
        }
        Ok(self.comparisons.get(exp).unwrap())
    }
}
