pub mod config;
pub mod diagnostic;
mod engine;
pub mod project;
pub mod rule;
pub mod rules;
pub mod source;
#[cfg(test)]
pub(crate) mod testing;

mod ast_util;

pub use config::{Config, Preset};
pub use diagnostic::{Diagnostic, Fix, Position, Severity, TextEdit};
pub use engine::{analyze_paths, analyze_sources, collect_rust_files, Report};
pub use rule::{Flag, Level, Rule, RuleMeta, Tier};
pub use source::{NodeSpan, SourceFile};
