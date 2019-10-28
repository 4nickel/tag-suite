use super::import::*;
use crate::app::{attr};

/// Stores the results of filesystem traversal
/// Shoutout to the glorious burntsushi :)
struct Walk {
    pub root: PathBuf,
    pub files: Vec<PathBuf>,
}

impl Walk {

    /// Create and run a new Walk
    pub fn new(root: String) -> Self {
        use walkdir::WalkDir;
        profile!("walk", {
            let files =
                WalkDir::new(&root).into_iter()
                    .filter_map(|e| e.ok())
                    .filter_map(|e| e.path().canonicalize().ok())
                    .collect();
            Self {
                root: PathBuf::from(&root),
                files: files,
            }
        })
    }
}

/// Stores the accumulated results of a filesystem scan
pub struct Scan {
    attributes: Vec<attr::File>,
    directories: Vec<String>,
    files: Vec<String>,
}

impl Scan {

    /// Create a new Scan.
    pub fn new() -> Self {
        Self { attributes: Vec::new(), directories: Vec::new(), files: Vec::new() }
    }

    /// Shrink the internal buffers to fit.
    pub fn shrink_to_fit(&mut self) {
        self.attributes.shrink_to_fit();
        self.directories.shrink_to_fit();
        self.files.shrink_to_fit();
    }

    /// Take the files buffer from this Scan.
    pub fn take_files(&mut self) -> Vec<String> {
        std::mem::replace(&mut self.files, Vec::new())
    }

    /// Take the attributes buffer from this Scan.
    pub fn take_attributes(&mut self) -> Vec<attr::File> {
        std::mem::replace(&mut self.attributes, Vec::new())
    }

    /// Take the directories buffer from this Scan.
    pub fn take_directories(&mut self) -> Vec<String> {
        std::mem::replace(&mut self.directories, Vec::new())
    }

    /// Scan a single file.
    fn scan_file(&mut self, file: String) {
        if let Ok(file) = attr::File::open(PathBuf::from(file)) {
            self.files.push(file.path_str().into());
            self.attributes.push(file);
        }
    }

    /// Scan a directory and it's descendants.
    fn scan_directory(&mut self, directory: String) {
        let walk = Walk::new(directory);
        self.attributes.extend(
            walk.files.iter()
                .filter_map(|e| {
                    if let Some(s) = e.to_str() {
                        if let Ok(f) = attr::File::open(PathBuf::from(s)) { return Some(f) }
                    }
                    None
                })
        );
        if let Some(s) = walk.root.to_str() {
            self.directories.push(s.to_owned())
        }
    }

    /// Scan the given file or directory.
    fn scan_path(this: &mut Scan, path: &str) {
        if let Ok(canonical) = PathBuf::from(path).canonicalize() {
            if let Some(s) = canonical.to_str() {
                if      canonical.is_file() { this.scan_file(s.to_string()) }
                else if canonical.is_dir()  { this.scan_directory(s.to_string()) }
            }
        }
    }

    /// Scan the given paths and returing a new Scan instance.
    pub fn scan(paths: &Vec<&str>) -> Self {
        profile!("scan", { Self::new();
            let mut this = Scan::new();
            for path in paths { Self::scan_path(&mut this, path); }
            info!("scanned {} Directories", this.directories.len());
            info!("scanned {} Files", this.files.len());
            info!("scanned {} Attributes", this.attributes.len());
            this.shrink_to_fit();
            this
        })
    }
}
