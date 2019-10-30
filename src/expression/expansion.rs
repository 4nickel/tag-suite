use super::{import::*, error::{Error as E}};
use regex::{Regex, Match};

const RECURSION_LIMIT: usize = 128;
const EMPTY_WRAPPER: (&'static str, &'static str) = ("", "");

#[derive(Debug, Clone)]
pub struct Expansions {
    regex: Regex,
    recursion_limit: usize,
    delimiter: (&'static str, &'static str),
    wrapper: (&'static str, &'static str),
    map: HashMap<String, String>,
}

/// Check an identifier for validity.
fn validate_identifier<'a>(id: &'a str) -> bool {
    id.len() > 0 && util::string::check_contents_with(id, |c| {
        c.is_ascii_alphanumeric()
            || (c == '-')
            || (c == '_')
    })
}

/// Generate a regex capable of finding delimited
/// expansions.
fn generate_regex(delimiter: (&str, &str)) -> Regex {
    let (open, close) = (
        regex::escape(delimiter.0),
        regex::escape(delimiter.1),
    );
    Regex::new(
        &format!("{}[^{}]*{}",
            open,
            close.chars().nth(1).expect("misconfigured delimiter"),
            close
        ))
        .expect("failed to compile regex")
}

impl Expansions {

    /// Create a new set of Expansions.
    pub fn new(delimiter: (&'static str, &'static str)) -> Self {
        Self::from_map(delimiter, HashMap::new())
    }

    /// Sets the optional wrapper strings.
    pub fn with_wrapper(mut self, wrapper: (&'static str, &'static str)) -> Self {
        self.wrapper = wrapper; self
    }

    /// Override recursion limit.
    pub fn with_recursion_limit(mut self, recursion_limit: usize) -> Self {
        self.recursion_limit = recursion_limit; self
    }

    /// Create Expansions from a map of strings.
    pub fn from_map(delimiter: (&'static str, &'static str), map: HashMap<String, String>) -> Self {
        Self {
            regex: generate_regex(delimiter),
            map: map,
            wrapper: EMPTY_WRAPPER,
            delimiter,
            recursion_limit: RECURSION_LIMIT,
        }
    }

    /// Add (K, V) pair to the Expansions.
    pub fn add(&mut self, key: String, val: String) {
        self.map.insert(key, val);
    }

    /// Canonicalize the expansion key for map lookup.
    /// We want to ignore the delimiter and strip whitespace.
    fn canonicalize_identifier<'a>(&self, exp: &'a str) -> Res<&'a str> {
        let (llen, rlen) = (self.delimiter.0.len(), self.delimiter.1.len());
        assert!(exp.len() >= llen + rlen, "bug: expansion capture is broken");

        let inner = &exp[llen..exp.len() - rlen];
        let ident = util::string::strip_with(inner, |c| c.is_whitespace());
        match validate_identifier(ident) {
            true => Ok(ident),
            false => Err(E::InvalidIdentifier { id: ident.into() }.into()),
        }
    }

    /// Get the replacement for a given match.
    fn resolve_match<'a>(&'a self, matched: &Match) -> Res<(&'a str, usize, usize)> {
        let key = self.canonicalize_identifier(matched.as_str())?;
        let val = self.map.get(key).ok_or(E::UnknownExpansion { key: key.into() })?;
        Ok((val.as_str(), matched.start(), matched.end()))
    }

    /// Expand a string by continually replacing any expandable substrings.
    pub fn expand(&self, mut expression: String) -> Res<String> {
        let mut recursion_guard = self.recursion_limit;
        while let Some((s, i, j)) = {
            self.regex.find(&expression).map(|m| self.resolve_match(&m)).transpose()?
        } {
            recursion_guard -= 1;
            if recursion_guard == 0 {
                return Err(E::RecursionLimitReached { key: s.into(), limit: self.recursion_limit }.into())
            }
            expression.replace_range(i..j, &format!("{}{}{}", self.wrapper.0, s, self.wrapper.1))
        }
        Ok(expression)
    }

    /// Extend this set of expansions with more expansions.
    pub fn extend(mut self, expansions: Expansions) -> Self {
        self.map.extend(expansions.map);
        self
    }
}

#[cfg(test)]
mod suite {

    use super::*;
    use std::fmt::Display;

    const DELIMITER: (&'static str, &'static str) = ("{{", "}}");

    fn no<T>(result: Res<T>)
    where
        T: Display + PartialEq + Eq
    {
        match result {
            Ok(result) => panic!("expected error, found '{}'", result),
            _ => {}
        }
    }

    #[test]
    fn check_normal_expansion() {
        let mut e = Expansions::new(DELIMITER);
        e.add("a".to_string(), "1".to_string());
        e.add("b".to_string(), "2".to_string());
        assert_eq!(e.expand("{{a}}".to_string()).unwrap(), "1".to_string());
        assert_eq!(e.expand("{{b}}".to_string()).unwrap(), "2".to_string());
    }

    #[test]
    fn check_recursive_expansion() {
        let mut e = Expansions::new(DELIMITER);
        e.add("a".to_string(), "a{{b}}".to_string());
        e.add("b".to_string(), "b{{c}}".to_string());
        e.add("c".to_string(), "c".to_string());
        assert_eq!(e.expand("{{a}}".to_string()).unwrap(), "abc".to_string());
    }

    #[test]
    fn check_wrapped_expansion() {
        let mut e = Expansions::new(DELIMITER).with_wrapper(("(", ")"));
        e.add("a".to_string(), "a{{b}}".to_string());
        e.add("b".to_string(), "b{{c}}".to_string());
        e.add("c".to_string(), "c".to_string());
        assert_eq!(e.expand("{{a}}".to_string()).unwrap(), "(a(b(c)))".to_string());
    }

    #[test]
    #[should_panic(expected = "RecursionLimitReached")]
    fn check_recursion_limit() {
        let mut e = Expansions::new(DELIMITER);
        e.add("a".to_string(), "{{b}}".to_string());
        e.add("b".to_string(), "{{a}}".to_string());
        no(e.expand("{{a}}".to_string()));
        no(e.expand("{{b}}".to_string()));
        e.expand("{{a}}".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidIdentifier")]
    fn check_invalid_identifier() {
        let e = Expansions::new(DELIMITER);
        no(e.expand("{{ white space }}".to_string()));
        no(e.expand("{{ question?mark }}".to_string()));
        no(e.expand("{{ amper&sand }}".to_string()));
        e.expand("{{}}".to_string()).unwrap();
    }

    #[test]
    #[should_panic(expected = "UnknownExpansion")]
    fn check_expansion_failure() {
        let mut e = Expansions::new(DELIMITER);
        e.add("a".to_string(), "1".to_string());
        e.add("b".to_string(), "2".to_string());
        e.expand("{{c}}".to_string()).unwrap();
    }

    #[test]
    fn check_whitespace_in_identifier() {
        let mut e = Expansions::new(DELIMITER);
        e.add("a".to_string(), "1".to_string());
        assert_eq!(e.expand("{{a }}".to_string()).unwrap(), "1".to_string());
        assert_eq!(e.expand("{{ a}}".to_string()).unwrap(), "1".to_string());
        assert_eq!(e.expand("{{ a }}".to_string()).unwrap(), "1".to_string());
    }
}
