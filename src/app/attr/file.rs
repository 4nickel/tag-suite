use super::{import::*, api, Tag, error::{Error as E}};
use crate::model::{file, tag};

/// A file and it's associated tag data. Provides
/// methods for opening, querying, modifying and saving
/// tags using extended filesystem attributes.
#[derive(Clone)]
pub struct File {
    path: PathBuf,
    tags: HashSet<Tag>,
    dirty: bool,
}

impl File {

    /// Opens a new file and reads the filesystems
    /// tag data
    pub fn open(path: PathBuf) -> Res<Self> {
        let tags = api::read(&path)?;
        assert!(tags.len() > 0, "a file should never have an empty tag-set: {}", path.to_string_lossy());
        Ok(File { path: path, tags: tags, dirty: false })
    }

    /// Return this files path
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str()
            .expect("FIXME: path validation")
    }

    /// Return this files set of tags
    pub fn tags(&self) -> &HashSet<Tag> {
        &self.tags
    }

    /// Return a custom iterator for this files tags
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=&'a Tag> {
        self.tags.iter()
    }

    /// Indicates if the contents of this file have
    /// been changed by us
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn ident<'a>(&'a self) -> file::Ident<'a> {
        file::Ident { path: self.path_str(), kind: util::file::get_file_type(self.path_str()).to_i64() }
    }

    /// Generate tuples mapping (file -> tag)
    pub fn mapping<'a>(&'a self) -> Vec<(file::Ident<'a>, tag::Ident<'a>)> {
        self.iter().map(|t| { (self.ident(), t.ident()) }).collect()
    }

    /// Merge one tag into another
    /// Returns a bool indicating changed contents
    pub fn merge(&mut self, src: &str, dst: &str) -> Res<bool> {
        if self.del(src) {
            self.add(dst)?;
        }
        Ok(self.dirty)
    }

    /// Purge all tags from this fle
    pub fn purge(&mut self) {
        self.tags.clear();
    }

    /// Make this files tags match the given set
    pub fn set_tags(&mut self, tags: &HashSet<Tag>) -> bool {
        self.dirty = self.tags.len() > 0;
        self.tags.clear();
        self.tags.extend(tags.iter().map(|e| e.clone()));
        self.dirty
    }

    /// Return the file-name component, so it exists
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|s| s.to_str())
    }

    /// Return the parent path component, so it exists
    pub fn base_name(&self) -> Option<&str> {
        self.path.parent().and_then(|s| s.to_str())
    }

    /// Return the <name> part of the canonical
    /// link-target for this file.
    /// To avoid name collisions, we create an
    /// md5-digest of the parent directories full
    /// path and append this file's name with a
    /// dash: <md5-digest>-<file-name>.
    fn link_name(&self) -> PathBuf {
        PathBuf::from(format!("{:?}-{}",
            md5::compute(self.base_name().unwrap_or("root::")),
                         self.file_name().unwrap_or("node::"))
        )
    }

    /// Return the full target path for linking this
    /// file
    fn link_path(&self, dst: &str) -> PathBuf {
        let mut path = PathBuf::from(dst);
        path.push(&self.link_name());
        path
    }

    /// Unlink this file from the target directory
    /// FIXME: symlink race here, but it's hard to fix :(
    pub fn unlink(&self, dst: &str) -> Res<bool> {
        let link = self.link_path(dst);
        if let Ok(m) = link.symlink_metadata() {
            if m.file_type().is_symlink() {
                info!("removing: {}", link.to_string_lossy());
                return Ok(std::fs::remove_file(link).is_ok())
            }
        }
        Err(E::NotUnlinkable { path: link.to_str().unwrap().into() }.into())
    }

    /// Link this file to the target directory
    pub fn link(&self, dst: &str) -> Res<bool> {
        use std::os::unix;
        info!("linking: {:?}", self.path);
        let link = self.link_path(dst);
        unix::fs::symlink(&self.path, link)?;
        Ok(true)
    }

    /// Check if the file contains a tag
    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Add a tag to the file
    pub fn add(&mut self, tag: &str) -> Res<bool> {
        // NOTE: potentially save an allocation here
        if self.tags.insert(Tag::new(tag.into())?) {
            self.dirty = true;
        }
        Ok(self.dirty)
    }

    /// Delete a tag from the file
    pub fn del(&mut self, tag: &str) -> bool {
        if self.tags.remove(tag) {
            self.dirty = true;
        }
        self.dirty
    }

    /// Write tags back to the filesystem only if the data has changed
    /// Returns a bool indicating if data has been written
    pub fn save(&mut self) -> Res<bool> {
        if self.dirty {
            self.force_save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Write tags back to the filesystem unconditionally
    pub fn force_save(&mut self) -> Res<()> {
        api::write(&self.path, &self.tags)?;
        self.dirty = false;
        Ok(())
    }

    /// Format the files path and tags in a simple way for human consumption
    pub fn format(&self) -> String {
        match api::format(&self.tags, TAG_SEPERATOR) {
            Some(s) => format!("{} {}", &self.path.to_string_lossy(), s),
            None => format!("{}", &self.path.to_string_lossy())
        }
    }
}

#[cfg(test)]
mod suite {
    use super::*;

    #[test]
    fn check_add() {
        let mut f = File::open("test/files/a".into()).unwrap();
        assert_eq!(f.has("foo"), false);
        assert_eq!(f.has("bar"), false);
        assert_eq!(f.add("foo").unwrap(), true);
        assert_eq!(f.add("bar").unwrap(), true);
        assert_eq!(f.has("foo"), true);
        assert_eq!(f.has("bar"), true);
    }

    #[test]
    fn check_del() {
        let mut f = File::open("test/files/a".into()).unwrap();
        assert_eq!(f.add("foo").unwrap(), true);
        assert_eq!(f.has("foo"), true);
        assert_eq!(f.del("foo"), true);
        assert_eq!(f.has("foo"), false);
    }

    #[test]
    fn check_save() {
        {
            let mut f = File::open("test/files/a".into()).unwrap();
            assert_eq!(f.add("foo").unwrap(), true);
            assert_eq!(f.save().unwrap(), true);
        }
        {
            let mut f = File::open("test/files/a".into()).unwrap();
            assert_eq!(f.del("foo"), true);
            assert_eq!(f.save().unwrap(), true);
        }
    }

    #[test]
    fn check_api_tag() {
        use super::super::import::*;
        let f = File::open("test/files/b".into()).unwrap();
        let api_tag = f.iter().next().unwrap();
        assert_eq!(api_tag.as_str(), API_TAG);
    }

    #[test]
    #[should_panic]
    fn check_empty_file_name() {
        File::open("".into()).unwrap();
    }
}
