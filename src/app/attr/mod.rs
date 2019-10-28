mod file;
mod tag;

pub mod prelude {
    /// The key under which we store our attribute data
    pub const TAG_KEY: &'static str = "user.tag.list";

    /// The tag seperator for serialization
    pub const TAG_SEPERATOR: &'static str = ",";

    /// The tag seperator for serialization
    pub const API_TAG: &'static str = "tdb::api::Entity";
}

pub mod import {
    pub use super::super::import::*;
    pub use super::prelude::*;
}

pub mod export {
    pub use super::file::*;
    pub use super::tag::*;
    pub use super::prelude::*;
}
pub use export::*;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "invalid tag: {}", tag)]
        InvalidTag { tag: String, },
        #[fail(display = "not unlinkable: {}", path)]
        NotUnlinkable { path: String, },
    }
}

pub mod api {
    use super::{import::*, export::*, error::{Error as E}};
    use regex::Regex;

    const DELIMITER: (&'static str, &'static str) = ("{{", "}}");
    lazy_static! {
        /// Regex for (in)validating tags
        static ref INVALID: Regex = {
            let (open, close) = (
                regex::escape(DELIMITER.0),
                regex::escape(DELIMITER.1),
            );
            Regex::new(&format!(r".*({}|{}|,).*", open, close))
                .expect("failed to compile regex")
        };
    }

    /// Tag sanitization
    pub fn sanitize<'a>(tag: &'a str) -> Res<&'a str> {
        if tag.len() == 0 || INVALID.is_match(tag) {
            Err(E::InvalidTag { tag: tag.into() }.into())
        } else {
            Ok(tag)
        }
    }

    pub fn unghost(tag: &str) -> Option<&str> {
        if !tag.starts_with("tdb::") {
            Some(tag)
        } else {
            None
        }
    }

    /// Generate an initial tag-set
    fn initial_tags() -> HashSet<Tag> {
        let mut set = HashSet::new();
        set.insert(Tag::new(API_TAG).expect("misconfigured api tag"));
        set
    }

    /// Read tag data from file
    pub fn read_with_sep(path: &Path, sep: &str) -> Res<HashSet<Tag>> {
        match xattr::get(path, TAG_KEY)? {
            Some(tags) => { decode(tags, sep) },
            None => { Ok(initial_tags()) }
        }
    }

    /// Read tag data from file
    pub fn read(path: &Path) -> Res<HashSet<Tag>> {
        read_with_sep(path, TAG_SEPERATOR)
    }

    /// Write tag data to file
    pub fn write_with_sep(path: &Path, tags: &HashSet<Tag>, sep: &str) -> Res<()> {
        if tags.is_empty() {
            purge(path)
        } else {
            Ok(xattr::set(path, TAG_KEY, &encode(tags, sep))?)
        }
    }

    /// Write tag data to file
    pub fn write(path: &Path, tags: &HashSet<Tag>) -> Res<()> {
        write_with_sep(path, tags, TAG_SEPERATOR)
    }

    /// Purge any tag data from the file
    pub fn purge(path: &Path) -> Res<()> {
        Ok(xattr::remove(path, TAG_KEY)?)
    }

    /// Format the given tags to a string
    pub fn migrate_seperator(path: &Path, src: &str, dst: &str) -> Res<()> {
        let tags = read_with_sep(path, src)?;
        write_with_sep(path, &tags, dst)?;
        Ok(())
    }

    /// Format the given tags to a string
    pub fn format(tags: &HashSet<Tag>, sep: &str) -> String {
        use std::cmp::Ordering;
        let mut list: Vec<&str> =
            tags.iter()
                .map(|tag| tag.as_str())
                .filter(|tag| unghost(tag).is_some())
                .filter(|tag| sanitize(tag).is_ok())
                .collect();
        list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        list.join(sep)
    }

    /// Decode and sanitize the given tags as UTF-8
    pub fn decode(tags: Vec<u8>, sep: &str) -> Res<HashSet<Tag>> {
        String::from_utf8(tags)?
            .split(sep)
            .try_fold(initial_tags(), |mut set, tag| {
                let t = Tag::new(tag.into())?;
                if !set.insert(t) { warn!("duplicate tag: '{}'", tag); }
                Ok(set)
            })
    }

    /// Encode the given tags as UTF-8
    pub fn encode(tags: &HashSet<Tag>, sep: &str) -> Vec<u8> {
        format(tags, sep).as_bytes().to_vec()
    }
}
