use std::collections::HashMap;
use crate::{
    app::{
        meta::{Template, config::CommandAction},
        data::{DatabaseLayer, query::{Pipeline, Forcings}}
    },
    expression::Expansions, expression::{Expression},
};
use super::{import::*, Action, action, config};

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

pub struct FieldReport {
    comment: Option<String>,
    summaries: Vec<Summary>,
}

impl FieldReport {
    pub fn new(convention: &Convention) -> Self {
        Self {
            comment: convention.comment.clone(),
            summaries: Vec::new(),
        }
    }
    pub fn add_summary(&mut self, summary: Summary) {
        self.summaries.push(summary)
    }
    pub fn format(&self) -> String {
        let mut buffer = String::new();
        for summary in &self.summaries {
            buffer.push_str(&summary.format());
        }
        buffer
    }
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
    pub fn enforce(&self, dapi: &DatabaseLayer, commit: bool) -> Res<FieldReport> {
        if let Some(comment) = &self.comment {
            info!(".. {}", comment);
        }
        let mut report = FieldReport::new(&self);
        for command in &self.commands {
            report.add_summary(command.run(dapi, commit)?);
        }
        Ok(report)
    }
}

struct Report<'a> {
    command: &'a Command,
    reports: Vec<action::Report<'a>>,
}

impl<'a> Report<'a> {
    pub fn new(command: &'a Command) -> Self {
        Self { command, reports: Vec::new() }
    }
    pub fn add_report(&mut self, report: action::Report<'a>) {
        self.reports.push(report)
    }
    pub fn summarize(&self) -> Summary {
        Summary {
            actions: self.reports.iter().map(|r| r.summarize()).collect(),
        }
    }
}

pub struct Summary {
    pub actions: Vec<action::Summary>,
}

impl Summary {
    pub fn format(&self) -> String {
        let mut buffer = String::new();
        for summary in &self.actions {
            buffer.push_str(&summary.format());
        }
        buffer
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

    pub fn forcings(&self) -> Forcings {
        let mut forcings = Forcings::new();
        for action in &self.actions {
            forcings = forcings.combine(action.forcings());
        }
        forcings
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
    pub fn run(&self, dapi: &DatabaseLayer, commit: bool) -> Res<Summary> {
        let results = dapi.query(&self.pipeline, self.forcings())?;
        let mut report = Report::new(&self);
        for action in &self.actions {
            let action_report = action.run(&results, commit)?;
            if commit {
                dapi.forget(&action_report.forgets())?;
                dapi.update(&action_report.updates())?;
            }
            report.add_report(action_report);
        }
        Ok(report.summarize())
    }
}
