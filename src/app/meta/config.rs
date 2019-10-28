use crate::{expression::Expansions, util::arg::Options};
use crate::app::meta::config;
use super::{import::*, error::{Error as E}};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub dictionary: Option<HashMap<String, String>>,
    pub conventions: Option<Vec<Convention>>,
    pub templates: Option<HashMap<String, Template>>,
}

impl Config {

    pub fn read(path: &str) -> Res<Self> {
        use std::io::prelude::*;
        use std::fs::File;
        trace!("reading configuration");
        let mut buffer = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut buffer)?;
        serde_yaml::from_str(&buffer)
            .map_err(|e| E::ConfigurationError {
                message: format!("{:?}", e)
            }.into())
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline<'a> {
    pub query: Option<&'a str>,
    pub filter: Option<&'a str>,
    pub pipe: Option<&'a str>,
}

impl<'a> Pipeline<'a> {

    pub fn new() -> Self {
        Self { query: None, filter: None, pipe: None }
    }

    pub fn with_query(mut self, query: &'a str) -> Self {
        self.query = Some(query); self
    }

    pub fn with_filter(mut self, filter: &'a str) -> Self {
        self.filter = Some(filter); self
    }

    pub fn with_pipe(mut self, pipe: &'a str) -> Self {
        self.pipe = Some(pipe); self
    }

    pub fn from_options(o: &'a Options) -> Self {
        Self { query: o.opt("QUERY"), filter: o.opt("filter"), pipe: o.opt("pipe") }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineBuf {
    pub query: Option<String>,
    pub filter: Option<String>,
    pub pipe: Option<String>,
}

impl PipelineBuf {

    pub fn from_config(config: config::Command) -> Self {
        Self { query: config.query, filter: config.filter, pipe: config.pipe }
    }

    pub fn from_pipeline(pipeline: &Pipeline) -> Self {
        Self {
            query: pipeline.query.map(|s| s.to_string()),
            filter: pipeline.filter.map(|s| s.to_string()),
            pipe: pipeline.pipe.map(|s| s.to_string()),
        }
    }

    pub fn as_pipeline<'a>(&'a self) -> Pipeline<'a> {
        Pipeline {
            query: self.query.as_ref().map(|s| s.as_str()),
            filter: self.filter.as_ref().map(|s| s.as_str()),
            pipe: self.pipe.as_ref().map(|s| s.as_str()),
        }
    }

    pub fn expand(mut self, expansions: &Expansions) -> Res<Self> {
        self.query = self.query.map(|s| expansions.expand(s)).transpose()?;
        self.filter = self.filter.map(|s| expansions.expand(s)).transpose()?;
        Ok(self)
    }
}


/// A representation of the 'map' command.
#[derive(Debug, Clone)]
pub enum CommandAction<'a> {
    Add(Vec<&'a str>),
    Del(Vec<&'a str>),
    Merge(&'a str, &'a str),
    Link(Vec<&'a str>),
    Unlink(Vec<&'a str>),
    Forget,
    Emit,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Instance {
    pub name: String,
    pub args: Vec<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Template {
    pub parameters: Vec<String>,
    pub commands: Vec<Command>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    pub query: Option<String>,
    pub filter: Option<String>,
    pub pipe: Option<String>,
    pub actions: Vec<Vec<String>>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Convention {
    pub comment: Option<String>,
    pub instances: Option<Vec<Instance>>,
    pub commands: Option<Vec<Command>>,
}
