use crate::{expression::Expansions};
use super::{import::*, config, Command};

/// A configured template
#[derive(Debug)]
pub struct Template {
    name: String,
    parameters: Vec<String>,
    commands: Vec<config::Command>,
}

impl Template {

    /// Create a new Directory instance
    pub fn configure(name: String, config: config::Template) -> Self {
        Self {
            name: name,
            parameters: config.parameters,
            commands: config.commands,
        }
    }

    pub fn parameter_expansions(&self, args: &Vec<String>) -> Expansions {
        let mut expansions = Expansions::new(("{{", "}}"));
        for (k, v) in self.parameters.iter().zip(args) {
            trace!("argument: '{}' -> '{}'", k, v);
            expansions.add(k.into(), v.into());
        }
        expansions
    }

    /// Instantiate this template generating a series of commands
    pub fn instantiate(&self, args: &Vec<String>, expand: &Expansions) -> Res<Vec<Command>> {
        let params = self.parameter_expansions(args).extend(expand.clone());
        self.commands.iter().try_fold(Vec::new(), |mut v, c| {
            v.push(Command::configure(c.clone(), &params)?);
            Ok(v)
        })
    }

    /// Return this templates name
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Return this templates commands
    pub fn commands(&self) -> &Vec<config::Command> {
        &self.commands
    }
}
