use std::collections::HashMap;
use crate::{expression::Expansions, expression::{Expression}};
use crate::app::{meta::config::CommandAction, data::{DatabaseLayer, query::{Pipeline, Forcings}}};
use crate::app::meta::Template;
use super::{import::*, Action, config};

/// A configured Command
#[derive(Debug, Clone)]
pub struct Command {
    pipeline: Pipeline,
    actions: Vec<Action>,
}

/// A configured Convention
#[derive(Debug, Clone)]
pub struct Convention {
    comment: Option<String>,
    commands: Vec<Command>,
}

impl Convention {

    pub fn new() -> Self {
        Self { comment: None, commands: Vec::new() }
    }

    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment); self
    }

    pub fn from_commands(commands: Vec<Command>) -> Self {
        Self { comment: None, commands }
    }

    /// Create a new configured Convention instance.
    pub fn configure(config: config::Convention, expand: &Expansions, templates: &HashMap<String, Template>) -> Res<Self> {
        use super::error::{Error as E};

        let mut commands = if let Some(instances) = config.instances {
            instances.iter().try_fold(Vec::new(), |mut v, i| -> Res<Vec<_>> {
                let template = templates.get(&i.name).ok_or(E::UnknownTemplate {
                    template: i.name.clone()
                })?;
                v.extend(template.instantiate(&i.args, expand)?);
                Ok(v)
            })
        } else {
            Ok(Vec::new())
        }?;

        if let Some(config_commands) = config.commands {
            commands.extend(
                config_commands.into_iter()
                    .filter_map(|c| Command::configure(c, &expand).ok())
            );
        }

        Ok(Self { comment: config.comment, commands: commands })
    }

    /// Create a new Convention from a single command-action.
    pub fn from_command_action(pipeline: Pipeline, action: &CommandAction) -> Self {
        Self {
            comment: None,
            commands: vec![Command::from_command_action(pipeline, action)],
        }
    }

    /// Enforce the Convention on a database interface.
    pub fn enforce(&self, dapi: &DatabaseLayer, commit: bool) -> Res<()> {
        if let Some(comment) = &self.comment {
            info!(".. {}", comment);
        }
        for command in &self.commands {
            command.run(dapi, commit)?;
        }
        Ok(())
    }
}

impl Command {

    /// Create a new configured Command instance.
    pub fn configure(config: config::Command, expand: &Expansions) -> Res<Self> {
        // TODO: get rid of this clone
        let pipeline = config::PipelineBuf::from_config(config.clone()).expand(expand)?;
        let actions =
            config.actions
                .into_iter()
                .map(|e| e.into_iter().map(|s| expand.expand(s).unwrap()).collect::<Vec<String>>().into())
                .collect();
        Ok(Self {
            pipeline: Pipeline::from_pipeline(pipeline)?,
            actions: actions,
        })
    }

    /// Create a new configured Command instance from a single command-action
    pub fn from_command_action(pipeline: Pipeline, action: &CommandAction) -> Self {
        Self { pipeline, actions: vec![Action::from_command_action(action)] }
    }

    /// Return this Command's query Expression.
    pub fn query<'a>(&'a self) -> &'a Option<Expression> {
        self.pipeline.get_query()
    }

    /// Return this Command's filter Expression.
    pub fn filter<'a>(&'a self) -> &'a Option<Expression> {
        self.pipeline.get_filter()
    }

    /// Return this Command's pipe Expression.
    pub fn pipe<'a>(&'a self) -> &'a Option<String> {
        self.pipeline.get_pipe()
    }

    /// Run the Command against the a data interface.
    pub fn run(&self, dapi: &DatabaseLayer, commit: bool) -> Res<()> {
        let results = dapi.query(&self.pipeline, Forcings::new())?;
        for action in &self.actions {
            let state = action.run(&results, commit)?;
            if commit {
                dapi.forget(&state.forget)?;
                // TODO: get rid of this collect
                dapi.update(&state.update.iter().map(|s| s.as_str()).collect())?;
            }
        }
        Ok(())
    }
}
