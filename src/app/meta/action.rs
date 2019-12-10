use super::{
    import::*,
    config::CommandAction,
    error::{Error as E}
};
use crate::{
    app::{
        attr::File,
        data::{FileView, query::{Results, Forcings}},
    },
    model::file,
};

pub mod names {
    pub const REPORT: &'static str = "report";
    pub const UNLINK: &'static str = "unlink";
    pub const LINK: &'static str = "link";
    pub const ADD: &'static str = "add";
    pub const DEL: &'static str = "del";
    pub const MERGE: &'static str = "merge";
    pub const FORGET: &'static str = "forget";
    pub const EMIT: &'static str = "emit";
}

/// An action performed by the API
#[derive(Debug, Clone)]
pub enum ApiAction {
    Emit,
    Forget,
    Report(String),
}

/// An action performed on a File
#[derive(Debug, Clone)]
pub enum TagAction {
    Add(Vec<String>),
    Del(Vec<String>),
    Merge(String, String),
    Link(Vec<String>),
    Unlink(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct FileReport<'a> {
    file: file::Borrow<'a>,
    message: &'a str,
}

impl<'a> FileReport<'a> {
    pub fn format(&self) -> String {
        format!("{} -> {}", self.message, self.file.path)
    }
}

pub struct Report<'a> {
    action: &'a Action,
    files: Vec<file::Borrow<'a>>,
    updates: Vec<file::Borrow<'a>>,
    forgets: Vec<file::Borrow<'a>>,
    reports: Vec<FileReport<'a>>,
}

impl<'a> Report<'a> {
    pub fn new(action: &'a Action) -> Self {
        Self {
            action,
            files: Vec::new(),
            updates: Vec::new(),
            forgets: Vec::new(),
            reports: Vec::new(),
        }
    }
    pub fn summarize(&self) -> Summary {
        Summary {
            action: self.action.to_string(),
            files: self.files.iter().map(|f| f.path.to_string()).collect(),
            updates: self.updates.iter().map(|f| f.path.to_string()).collect(),
            forgets: self.forgets.iter().map(|f| f.path.to_string()).collect(),
            reports: self.reports.iter().map(|r| r.format()).collect(),
        }
    }
    pub fn updates(&self) -> Vec<&str> {
        // TODO: get rid of this collect
        // we have to pass an iterator here
        // and make the data layer accept
        // iterators instead of buffers
        self.updates.iter().map(|f| f.path).collect()
    }
    pub fn forgets(&self) -> Vec<i64> {
        // TODO: get rid of this collect
        // we have to pass an iterator here
        // and make the data layer accept
        // iterators instead of buffers
        self.forgets.iter().map(|f| f.id).collect()
    }
    pub fn add_file(&mut self, file: file::Borrow<'a>) { self.files.push(file) }
    pub fn add_update(&mut self, file: file::Borrow<'a>) { self.updates.push(file) }
    pub fn add_forget(&mut self, file: file::Borrow<'a>) { self.forgets.push(file) }
    pub fn add_report(&mut self, file: file::Borrow<'a>, message: &'a str) {
        self.reports.push(FileReport { file, message })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub action: String,
    pub files: Vec<String>,
    pub updates: Vec<String>,
    pub forgets: Vec<String>,
    pub reports: Vec<String>
}

impl Summary {
    pub fn format(&self) -> String {
        let mut s = self.action.clone();
        s.push('\n');
        s.push_str(&self.reports.join("\n"));
        s
    }
}

/// Action wrapper type
#[derive(Debug, Clone)]
pub enum Action {
    Api(ApiAction),
    Tag(TagAction),
}

impl Action {
    pub fn forcings(&self) -> Forcings {
        match self {
            Self::Tag(_) => Forcings::new(),
            Self::Api(_) => Forcings::new().mapped(),
        }
    }
    pub fn to_string(&self) -> String {
        match self {
            Self::Tag(TagAction::Add(s)) => { format!("{}({})", names::ADD, s.join(", ")) }
            Self::Tag(TagAction::Del(s)) => { format!("{}({})", names::DEL, s.join(", ")) }
            Self::Tag(TagAction::Link(s)) => { format!("{}({})", names::LINK, s.join(", ")) }
            Self::Tag(TagAction::Unlink(s)) => { format!("{}({})", names::UNLINK, s.join(", ")) }
            Self::Tag(TagAction::Merge(s, d)) => { format!("{}({}, {})", names::MERGE, s, d) }
            Self::Api(ApiAction::Emit) => { format!("{}", names::EMIT) }
            Self::Api(ApiAction::Forget) => { format!("{}", names::FORGET) }
            Self::Api(ApiAction::Report(m)) => { format!("{}({})", names::REPORT, m) }
        }
    }
    pub fn from_command_action(cmd: &CommandAction) -> Self {
        let alloc = |v: &Vec<&str>| -> Vec<String> {
            v.iter().map(|s| s.to_string()).collect()
        };
        match cmd {
            CommandAction::Unlink(directories) => { Action::Tag(TagAction::Unlink(alloc(directories))) }
            CommandAction::Link(directories) => { Action::Tag(TagAction::Link(alloc(directories))) }
            CommandAction::Add(tags) => { Action::Tag(TagAction::Add(alloc(tags))) }
            CommandAction::Del(tags) => { Action::Tag(TagAction::Del(alloc(tags))) }
            CommandAction::Merge(src, dst) => { Action::Tag(TagAction::Merge(src.to_string(), dst.to_string())) }
            CommandAction::Forget => { Action::Api(ApiAction::Forget) },
            CommandAction::Emit => { Action::Api(ApiAction::Emit) },
            CommandAction::Report(s) => { Action::Api(ApiAction::Report(s.to_string())) },
        }
    }
    /// Parse an action from a list of string tokens.
    /// Used for reading actions from the config.
    pub fn parse(mut tokens: Vec<String>) -> Res<Self> {
        /*
         * NOTE: Our helper functions work on reversed inputs.
         */
        tokens.reverse();
        let action = match one(&mut tokens)?.as_str() {
            names::UNLINK => { Action::Tag(TagAction::Unlink(one_or_more(&mut tokens)?)) }
            names::LINK => { Action::Tag(TagAction::Link(one_or_more(&mut tokens)?)) }
            names::ADD => { Action::Tag(TagAction::Add(one_or_more(&mut tokens)?)) }
            names::DEL => { Action::Tag(TagAction::Del(one_or_more(&mut tokens)?)) }
            names::MERGE => { Action::Tag(TagAction::Merge(one(&mut tokens)?, one(&mut tokens)?)) }
            names::EMIT => { Action::Api(ApiAction::Emit) }
            names::FORGET => { Action::Api(ApiAction::Forget) }
            names::REPORT => { Action::Api(ApiAction::Report(one(&mut tokens)?)) }
            unknown => { return Err(E::UnknownAction { action: unknown.into() }.into()) }
        };
        if tokens.len() == 0 {
            Ok(action)
        } else {
            Err(E::ConfigurationError {
                message: "trailing tokens in action".into()
            }.into())
        }
    }
    pub fn run<'a>(&'a self, results: &'a Results, commit: bool) -> Res<Report<'a>> {
        let mut report = Report::new(self);
        match self {
            Self::Tag(action) => {
                for file in results.file_iter()? {
                    let mut attributes = File::open(PathBuf::from(file.path))?;
                    action.run(&mut report, file, &mut attributes, commit)?;
                }
            }
            Self::Api(action) => {
                for file in results.file_view_iter()? {
                    let mut attributes = File::open(PathBuf::from(file.path()))?;
                    action.run(&mut report, &file, &mut attributes, commit)?;
                }
            }
        }
        Ok(report)
    }
}

impl TagAction {
    /// Run this action on a single file
    pub fn run<'a>(&self, report: &mut Report<'a>, file: file::Borrow<'a>, attributes: &mut File, commit: bool) -> Res<()> {
        trace!("{:?} -> {}", self, attributes.path_str());
        report.add_file(file);
        let update = match self {
            TagAction::Unlink(directories) => {
                for d in directories { if commit { attributes.unlink(d)?; }}
                false
            },
            TagAction::Link(directories) => {
                for d in directories { if commit { attributes.link(d)?; }}
                false
            },
            TagAction::Add(tags) => {
                for t in tags { attributes.add(t)?; }
                attributes.is_dirty()
            },
            TagAction::Del(tags) => {
                for t in tags { attributes.del(&t); }
                attributes.is_dirty()
            },
            TagAction::Merge(src, dst) => {
                attributes.merge(&src, &dst)?
            }
        };
        if update {
            if commit { trace!("saving"); attributes.save()?; }
            report.add_update(file);
        }
        Ok(())
    }
}

impl ApiAction {
    /// Run this action on a single file
    pub fn run<'a>(&'a self, report: &mut Report<'a>, file: &FileView<'a>, attributes: &mut File, commit: bool) -> Res<()> {
        trace!("{:?} -> {}", self, attributes.path_str());
        report.add_file(file.as_borrow());
        let update = match self {
            ApiAction::Emit => {
                attributes.set_tags(&file.tag_set()?);
                true
            },
            ApiAction::Forget => {
                report.add_forget(file.as_borrow());
                false
            },
            ApiAction::Report(message) => {
                report.add_report(file.as_borrow(), message);
                false
            },
        };
        if update {
            report.add_update(file.as_borrow());
            if commit { attributes.save()?; }
        }
        Ok(())
    }
}

impl From<Vec<String>> for Action {
    fn from(tokens: Vec<String>) -> Self {
        Self::parse(tokens).unwrap()
    }
}

/// Pop an element or complain
fn one(v: &mut Vec<String>) -> Res<String> {
    v.pop().ok_or(E::ConfigurationError {
        message: "unexpected end of input".into()
    }.into())
}

/// Take the remaining elements or complain if there are none
fn one_or_more(v: &mut Vec<String>) -> Res<Vec<String>> {
    match v.len() == 0 {
        true => Err(E::ConfigurationError {
            message: "expected one or more arguments".into()
        }.into()),
        false => Ok(v.drain(..).rev().collect()), // remember to reverse again
    }
}

#[cfg(test)]
pub mod suite {
    use super::*;
    #[test]
    fn check_parse_action_add()  {
        let tokens = vec![names::ADD.to_string(), "Test::Test".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Add(_)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_del()  {
        let tokens = vec![names::DEL.to_string(), "Test::Test".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Del(_)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_merge()  {
        let tokens = vec![names::MERGE.to_string(), "Test::A".to_string(), "Test::B".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Merge(_, _)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_link()  {
        let tokens = vec![names::LINK.to_string(), "a/directory".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Link(_)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_unlink()  {
        let tokens = vec![names::UNLINK.to_string(), "a/directory".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Unlink(_)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_emit()  {
        let tokens = vec![names::EMIT.to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Api(ApiAction::Emit) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_forget()  {
        let tokens = vec![names::FORGET.to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Api(ApiAction::Forget) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    fn check_parse_action_report()  {
        let tokens = vec![names::REPORT.to_string(), "Stuff".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Api(ApiAction::Report(_)) => { }, _ => panic!("wrong action"), }
    }
    #[test]
    #[should_panic]
    fn check_parse_action_unknown()  {
        let tokens = vec!["unknown".to_string()];
        let action = Action::parse(tokens).unwrap();
        match action { Action::Tag(TagAction::Add(_)) => { }, _ => panic!("wrong action"), }
    }
}
