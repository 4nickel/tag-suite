use clap::{ArgMatches};

pub struct Options<'o, 'a> {
    args: &'o ArgMatches<'a>,
}

impl<'o, 'a> Options<'o, 'a> {

    pub fn new(args: &'o ArgMatches<'a>) -> Self {
        Self { args }
    }

    pub fn _get<'f>(args: &'f ArgMatches, key: &str) -> &'f str {
        args.value_of(key)
            .expect("bug: missing argument value")
    }

    pub fn _vec<'f>(args: &'f ArgMatches, key: &str) -> Vec<&'f str> {
        args.values_of(key)
            .expect("bug: missing argument value")
            .collect()
    }

    pub fn _opt_vec<'f>(args: &'f ArgMatches, key: &str) -> Option<Vec<&'f str>> {
        match args.values_of(key) {
            Some(v) => Some(v.collect()),
            None => None,
        }
    }

    pub fn _opt<'f>(args: &'f ArgMatches, key: &str) -> Option<&'f str> {
        args.value_of(key)
    }

    pub fn _flag<'f>(args: &'f ArgMatches, key: &str) -> bool {
        args.is_present(key)
    }

    pub fn flag(&'a self, key: &str) -> bool {
        Options::_flag(self.args, key)
    }

    pub fn get(&'a self, key: &str) -> &'a str {
        Options::_get(self.args, key)
    }

    pub fn opt_vec(&'a self, key: &str) -> Option<Vec<&'a str>> {
        Options::_opt_vec(self.args, key)
    }

    pub fn vec(&'a self, key: &str) -> Vec<&'a str> {
        Options::_vec(self.args, key)
    }

    pub fn opt(&'a self, key: &str) -> Option<&'a str> {
        Options::_opt(self.args, key)
    }
}
