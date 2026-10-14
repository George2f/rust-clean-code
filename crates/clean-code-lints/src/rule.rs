use std::path::PathBuf;

use crate::diagnostic::Fix;
use crate::project::ProjectIndex;
use crate::source::{NodeSpan, SourceFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Static,
    Heuristic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Allow,
    Warn,
    Error,
}

impl Level {
    pub fn parse(value: &str) -> Option<Level> {
        match value {
            "off" | "allow" => Some(Level::Allow),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Level::Allow => "off",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    On,
    Off,
}

impl Flag {
    pub fn is_on(self) -> bool {
        self == Flag::On
    }
}

pub struct RuleMeta {
    pub name: &'static str,
    pub clean_code_id: &'static str,
    pub clean_code_rule: &'static str,
    pub description: &'static str,
    pub tier: Tier,
    pub fixable: bool,
    pub has_suggestions: bool,
}

pub struct RuleContext<'a> {
    pub file: &'a SourceFile,
    pub ast: &'a syn::File,
    pub options: &'a toml::Value,
    pub project: &'a ProjectIndex,
}

pub fn value_int(value: &toml::Value, key: &str, default: i64) -> i64 {
    value
        .get(key)
        .and_then(toml::Value::as_integer)
        .unwrap_or(default)
}

pub fn value_names(value: &toml::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

impl RuleContext<'_> {
    pub fn options_int(&self, key: &str, default: i64) -> i64 {
        value_int(self.options, key, default)
    }

    pub fn options_flag(&self, key: &str, default: Flag) -> Flag {
        match self.options.get(key).and_then(toml::Value::as_bool) {
            Some(true) => Flag::On,
            Some(false) => Flag::Off,
            None => default,
        }
    }

    pub fn options_names(&self, key: &str) -> Vec<String> {
        value_names(self.options, key)
    }

    pub fn options_names_or(&self, key: &str, defaults: &[&str]) -> Vec<String> {
        let configured = self.options.get(key);
        match configured {
            Some(_) => self.options_names(key),
            None => defaults.iter().map(|value| (*value).to_string()).collect(),
        }
    }

    pub fn options_ints(&self, key: &str) -> Vec<i64> {
        self.options
            .get(key)
            .and_then(toml::Value::as_array)
            .map(|items| items.iter().filter_map(toml::Value::as_integer).collect())
            .unwrap_or_default()
    }

    pub fn finding(&self, span: NodeSpan, message: impl Into<String>) -> Finding {
        Finding {
            message: message.into(),
            help: None,
            file: self.file.path.clone(),
            span,
            fix: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub message: String,
    pub help: Option<String>,
    pub file: PathBuf,
    pub span: NodeSpan,
    pub fix: Option<Fix>,
}

impl Finding {
    pub fn new(file: PathBuf, span: NodeSpan, message: impl Into<String>) -> Finding {
        Finding {
            message: message.into(),
            help: None,
            file,
            span,
            fix: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Finding {
        self.help = Some(help.into());
        self
    }

    pub fn with_fix(mut self, fix: Fix) -> Finding {
        self.fix = Some(fix);
        self
    }
}

pub trait Rule: Send + Sync {
    fn meta(&self) -> &'static RuleMeta;

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding>;

    fn check_project(&self, _project: &ProjectIndex, _options: &toml::Value) -> Vec<Finding> {
        Vec::new()
    }
}
