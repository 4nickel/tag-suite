pub mod command;
pub mod template;
pub mod action; // TODO: why pub?

pub mod config;

pub mod prelude {
    pub use super::config::*;
}

pub mod import {
    pub use super::super::import::*;
    pub use super::prelude::*;
    pub use crate::model::prelude::*;
}

pub mod export {
    pub use super::prelude::*;
    pub use super::command::*;
    pub use super::template::*;
    pub use super::action::*;
    pub use super::api::*;
}
pub use export::*;

pub mod error {
    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "unknown action: {}", action)]
        UnknownAction { action: String, },
        #[fail(display = "config error: {}", message)]
        ConfigurationError { message: String, },
        #[fail(display = "unknown template: {}", template)]
        UnknownTemplate { template: String, },
    }
}

pub mod api {
    use super::{import::*, template, command};
    use crate::expression::{Expansions};

    pub struct Configuration {
        pub templates: HashMap<String, template::Template>,
        pub conventions: Vec<command::Convention>,
        pub expansions: Expansions,
    }

    impl Configuration {

        pub fn configure(mut config: Config) -> Res<Self> {
            let exp = ("{{", "}}");

            let expansions = { if let Some(ref e) = config.dictionary {
                Expansions::from_map(exp, e.clone())
            } else {
                Expansions::new(exp)
            } };

            let mut templates = HashMap::new();
            let mut conventions = Vec::new();

            if let Some(mut t) = config.templates.take() {
                for (k, v) in t.drain() {
                    templates.insert(k.clone(), template::Template::configure(k, v));
                }
            }

            if let Some(mut c) = config.conventions.take() {
                for v in c.drain(..) { conventions.push(command::Convention::configure(v, &expansions, &templates)?); }
            }

            Ok(Self { expansions, conventions, templates })
        }

        pub fn add_template(&mut self, template: template::Template) {
            self.templates.insert(template.name().into(), template);
        }

        pub fn add_convention(&mut self, convention: command::Convention) {
            self.conventions.push(convention);
        }

        pub fn expansions(&self) -> &Expansions {
            &self.expansions
        }

        pub fn expand(&self, dir: String) -> Res<String> {
            self.expansions.expand(dir)
        }
    }
}
