use std::collections::HashSet;

pub mod constants {
    pub const RESERVED_TAG: &'static str = "tag";
    pub const RESERVED_TDB: &'static str = "tdb";
    pub const RESERVED_PATH: &'static str = "path";
    pub const RESERVED_KIND: &'static str = "kind";
    pub const NAMESPACE_SEP: &'static str = "::";
}
use constants::*;

const DEFAULT_RESERVED: &'static str = RESERVED_TAG;

lazy_static! {
    /// Reserved Namespaces
    static ref RESERVED: HashSet<&'static str> = {
        let mut set = HashSet::new();
        set.insert(RESERVED_PATH);
        set.insert(RESERVED_KIND);
        set.insert(RESERVED_TAG);
        set.insert(RESERVED_TDB);
        set
    };

    static ref SHORTHAND_HEAD: String = {
        format!("{}{}", '%', NAMESPACE_SEP)
    };

    static ref SHORTHAND_TAIL: String = {
        format!("{}{}", NAMESPACE_SEP, '%')
    };
}

#[derive(Copy, Clone, Debug)]
pub enum Namespace<'n> {
    Root,
    Reserved(&'n str),
    User(&'n str),
}

impl<'n> Namespace<'n> {
    /// Return the underlying string slice
    pub fn as_str(&'n self) -> &'n str {
        match self {
            Self::Root => { "" },
            Self::Reserved(s) => { s },
            Self::User(s) => { s },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Namespec<'n> {
    borrow: &'n str,
    slices: Vec<Namespace<'n>>,
}

impl<'n> Namespec<'n> {

    pub fn new(name: &'n str) -> Self {
        Self {
            borrow: name,
            slices: Self::parse(name).unwrap(),
        }
    }

    pub fn to_string(&self) -> String {
        self.slices.iter()
            .map(Namespace::as_str)
            .collect::<Vec<&str>>()
            .join(NAMESPACE_SEP)
    }

    pub fn get_root(&'n self) -> &'n Namespace<'n> {
        &self.slices[0]
    }

    pub fn get_reserved(&'n self) -> &'n Namespace<'n> {
        &self.slices[1]
    }

    pub fn get_leaf(&'n self) -> &'n Namespace<'n> {
        &self.slices[self.slices.len() - 1]
    }

    pub fn to_tagspace(&self) -> Namespec<'n> {
        let mut specs = Vec::new();
        specs.push(Namespace::Root);
        specs.extend(self.slices[2..].iter().map(|e| *e));
        Self {
            borrow: self.borrow,
            slices: specs,
        }
    }

    pub fn canonicalize_with_default(&self, default: &'n str) -> Self {
        let mut specs = Vec::new();
        match self.slices[0] {
            Namespace::Root => { }
            Namespace::Reserved(_) => {
                specs.push(Namespace::Root);
            }
            Namespace::User(_) => {
                specs.push(Namespace::Root);
                specs.push(Namespace::Reserved(default));
            }
        }
        specs.extend(&self.slices);
        Self {
            borrow: self.borrow,
            slices: specs,
        }
    }

    pub fn parse(name: &'n str) -> Option<Vec<Namespace<'n>>> {

        const EMPTY: &'static str = "";

        let mut specs = Vec::new();
        let slices: Vec<&str> = name.split(NAMESPACE_SEP).collect();
        match slices.get(0) {
            Some(&EMPTY) => {
                specs.push(Namespace::Root);
            }
            Some(&s) => {
                if RESERVED.contains(s) {
                    specs.push(Namespace::Reserved(s));
                } else {
                    if s != EMPTY {
                        specs.push(Namespace::User(s));
                    }
                }
            }
            None => return None,
        }
        for i in 1..slices.len() {
            if slices[i] != EMPTY {
                specs.push(Namespace::User(slices[i]));
            }
        }
        Some(specs)
    }

    pub fn apply_shorthand_syntax<'a>(exp: &'a str) -> String {
        let len = exp.len();
        if len > 1 {
            let mut i = 0usize;
            let mut j = len;
            let head = {
                if exp.chars().nth(0) == Some(':')
                && exp.chars().nth(1) != Some(':') {
                    i = 1usize;
                    SHORTHAND_HEAD.as_str()
                } else { "" }
            };
            let tail = {
                if exp.chars().nth(len-1) == Some(':')
                && exp.chars().nth(len-2) != Some(':') {
                    j = len-1usize;
                    SHORTHAND_TAIL.as_str()
                } else { "" }
            };
            format!("{}{}{}", head, &exp[i..j], tail)
        } else {
            exp.into()
        }
    }

    pub fn canonicalize_user_expression<'a>(exp: &'a str) -> (Namespec<'a>, Namespec<'a>) {
        let namespec = Namespec::new(exp);
        let canonical = namespec.canonicalize_with_default(DEFAULT_RESERVED);
        let tagspace = canonical.to_tagspace();
        (canonical, tagspace)
    }
}

impl<'n> From<&'n Namespace<'n>> for &'n str {
    fn from(namespace: &'n Namespace) -> &'n str {
        namespace.as_str()
    }
}
