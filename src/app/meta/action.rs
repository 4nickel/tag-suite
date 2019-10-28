use super::{import::*, config::CommandAction, error::{Error as E}};
use crate::{app::attr::File, app::data::{FileView, query::{prelude::*, Results}}};

/// An action performed by the API
#[derive(Debug, Clone)]
pub enum ApiAction {
    Emit,
    Forget,
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

/// Action wrapper type
#[derive(Debug, Clone)]
pub enum Action {
    Api(ApiAction),
    Tag(TagAction),
}

pub const UNLINK: &'static str = "unlink";
pub const LINK: &'static str = "link";
pub const ADD: &'static str = "add";
pub const DEL: &'static str = "del";
pub const MERGE: &'static str = "merge";
pub const FORGET: &'static str = "forget";
pub const EMIT: &'static str = "emit";

pub struct ActionState {
    pub files: Vec<File>,
    pub update: Vec<String>,
    pub forget: Vec<Fid>,
}

impl ActionState {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            update: Vec::new(),
            forget: Vec::new(),
        }
    }
}

impl Action {
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
        }
    }
    /// Parse an action from a list of string tokens.
    /// Used for reading actions from the config.
    pub fn parse(mut tokens: Vec<String>) -> Res<Self> {
        // we have to move the vec in and out of these
        // lambdas to keep borrowck happy.
        tokens.reverse(); // avoid taking from the front and use pop instead
        let pop = |v: &mut Vec<String>| -> Res<String> {
            v.pop().ok_or(E::ConfigurationError { message: "unexpected end of input".into() }.into())
        };
        let one_or_more = |v: Vec<String>| -> Res<Vec<String>> {
            match v.len() == 0 {
                true => Err(E::ConfigurationError { message: "expected one or more arguments".into() }.into()),
                false => Ok(v),
            }
        };
        match pop(&mut tokens)?.as_str() {
            UNLINK => { Ok(Action::Tag(TagAction::Unlink(one_or_more(tokens)?))) }
            LINK => { Ok(Action::Tag(TagAction::Link(one_or_more(tokens)?))) }
            ADD => { Ok(Action::Tag(TagAction::Add(one_or_more(tokens)?))) }
            DEL => { Ok(Action::Tag(TagAction::Del(one_or_more(tokens)?))) }
            MERGE => { Ok(Action::Tag(TagAction::Merge(pop(&mut tokens)?, pop(&mut tokens)?))) }
            EMIT => { Ok(Action::Api(ApiAction::Emit)) }
            FORGET => { Ok(Action::Api(ApiAction::Forget)) }
            unknown => { Err(E::UnknownAction { action: unknown.into() }.into()) }
        }
    }
    pub fn run(&self, results: &Results, commit: bool) -> Res<ActionState> {
        let mut state = ActionState::new();
        match self {
            Self::Tag(action) => {
                for f in results.file_iter()? {
                    let mut file = File::open(PathBuf::from(f.1))?;
                    action.run(&mut state, &mut file, commit)?;
                }
            }
            Self::Api(action) => {
                for f in results.file_view_iter()? {
                    let mut file = File::open(PathBuf::from(f.path()))?;
                    action.run(&mut state, &mut file, &f, commit)?;
                }
            }
        }
        Ok(state)
    }
}

impl TagAction {
    /// Run this action on a single file
    pub fn run(&self, state: &mut ActionState, file: &mut File, commit: bool) -> Res<()> {
        trace!("{:?} -> {}", self, file.path_str());
        let update = match self {
            TagAction::Unlink(directories) => {
                for d in directories { file.unlink(d)?; }
                false
            },
            TagAction::Link(directories) => {
                for d in directories { file.link(d)?; }
                false
            },
            TagAction::Add(tags) => {
                for t in tags { file.add(t)?; }
                file.is_dirty()
            },
            TagAction::Del(tags) => {
                for t in tags { file.del(&t); }
                file.is_dirty()
            },
            TagAction::Merge(src, dst) => {
                file.merge(&src, &dst)?
            }
        };
        if update {
            state.update.push(file.path_str().into());
            if commit { trace!("saving"); file.save()?; }
        }
        Ok(())
    }
}

impl ApiAction {
    /// Run this action on a single file
    pub fn run(&self, state: &mut ActionState, file: &mut File, view: &FileView, commit: bool) -> Res<()> {
        trace!("{:?} -> {}", self, file.path_str());
        let update = match self {
            ApiAction::Emit => {
                file.set_tags(&view.tag_set());
                true
            },
            ApiAction::Forget => {
                state.forget.push(view.id());
                false
            },
        };
        if update {
            state.update.push(file.path_str().into());
            if commit { file.save()?; }
        }
        Ok(())
    }
}

impl From<Vec<String>> for Action {
    fn from(tokens: Vec<String>) -> Self {
        Self::parse(tokens).unwrap()
    }
}
