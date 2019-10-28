use super::{import::*, export::*, state::{Results}};
use crate::{app::attr};

/// Collectors provide a simple interface for
/// getting data out of queries in a reusable
/// way. They also provide a mechanism for
/// communicating the requirements to successfully
/// extract the data.
pub trait Collector<C> {
    fn forcings(&self) -> Forcings;
    fn collect_results(&self, results: Results) -> C;
    fn collect(&self, results: Results) -> C {
        profile!("collect", { self.collect_results(results) })
    }
}

macro_rules! collect {
    ( $b:expr, $n:expr, char, $($c:expr),+ ) => {
        {
            $($b.push($c);)*
            if { $n } { $b.push('\n'); }
            $b
        }
    };
    ( $b:expr, $n:expr, String, $($c:expr),+ ) => {
        {
            $($b.push_str($c);)*
            if { $n } { $b.push('\n'); }
            $b
        }
    };
}

pub trait Stringify {
    fn stringify(&self) -> String;
}

impl Stringify for usize {
    fn stringify(&self) -> String {
        format!("{}\n", self)
    }
}

pub struct FileIds;
impl Collector<Res<Vec<Fid>>> for FileIds {
    fn forcings(&self) -> Forcings {
        Forcings::new()
    }
    fn collect_results(&self, results: Results) -> Res<Vec<Fid>> {
        Ok(results.file_iter()?.map(|e| e.0).collect())
    }
}

pub struct TagIds;
impl Collector<Res<Vec<Tid>>> for TagIds {
    fn forcings(&self) -> Forcings {
        Forcings::new()
    }
    fn collect_results(&self, results: Results) -> Res<Vec<Tid>> {
        Ok(results.tag_iter()?.map(|e| e.0).collect())
    }
}

pub struct FileCount;
impl Collector<Res<usize>> for FileCount {
    fn forcings(&self) -> Forcings {
        Forcings::new()
    }
    fn collect_results(&self, results: Results) -> Res<usize> {
        Ok(results.row_count())
    }
}

pub struct TagCount;
impl Collector<Res<usize>> for TagCount {
    fn forcings(&self) -> Forcings {
        Forcings::new()
    }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<usize> {
        Ok(results.row_count())
    }
}

pub struct FilePaths;
impl Collector<Res<String>> for FilePaths {
    fn forcings(&self) -> Forcings {
        Forcings::new()
    }
    fn collect_results(&self, results: Results) -> Res<String> {
        Ok(results.file_iter()?
            .fold(String::new(), |mut s, (_, file)| {
                collect!(&mut s, true, String, file);
                s
            }))
    }
}

pub struct TagNames;
impl Collector<Res<String>> for TagNames {
    fn forcings(&self, ) -> Forcings {
        Forcings::new()
    }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<String> {
        Ok(results.tag_iter()?
            .fold(String::new(), |mut s, (_, tag)| {
                collect!(&mut s, true, String, tag);
                s
            }))
    }
}

use std::path::PathBuf;
pub struct TagFiles;
impl Collector<Res<Vec<attr::File>>> for TagFiles {
    fn forcings(&self, ) -> Forcings { Forcings::new() }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<Vec<attr::File>> {
        Ok(results
            .file_iter()?
            .filter_map(|(_, file)| {
                match attr::File::open(PathBuf::from(file)) {
                    Ok(file) => Some(file),
                    _ => None
                }
            })
            .collect())
    }
}

pub struct SerializePlain;
impl Collector<Res<String>> for SerializePlain {
    fn forcings(&self) -> Forcings { Forcings::new().mapped() }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<String> {
        let output =
            results.file_view_iter()?
                .fold(String::new(), |mut buf, file| {
                    let f = SerialFile::new(&file);
                    collect!(&mut buf, true, String, &f.to_plain());
                    buf
                });
        Ok(output)
    }
}

#[derive(Serialize)]
struct SerialFile<'q> {
    path: &'q str,
    tags: Vec<&'q str>,
}

impl<'q> SerialFile<'q> {
    pub fn new(view: &FileView<'q>) -> Self {
        Self {
            path: view.path(),
            tags: view.iter()
                .map(|t| t.name())
                .filter(|t| !t.starts_with("tdb::"))
                .collect()
        }
    }
    pub fn to_plain(&self) -> String {
        format!("{}\n {}", self.path, self.tags.join("\n * "))
    }
}

pub struct SerializeYaml;
impl Collector<Res<String>> for SerializeYaml {
    fn forcings(&self) -> Forcings { Forcings::new().mapped() }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<String> {
        let serial: Vec<SerialFile> =
            results.file_view_iter()?
                .map(|f| SerialFile::new(&f))
                .collect();
        Ok(serde_yaml::to_string(&serial)?)
    }
}

pub struct SerializeJson;
impl Collector<Res<String>> for SerializeJson {
    fn forcings(&self) -> Forcings { Forcings::new().mapped() }
    fn collect_results<'q>(&self, results: Results<'q>) -> Res<String> {
        let serial: Vec<SerialFile> =
            results.file_view_iter()?
                .map(|f| SerialFile::new(&f))
                .collect();
        Ok(serde_json::to_string(&serial)?)
    }
}
