use std::collections::HashMap;
use std::path::Path;

use crate::rule::{Level, RuleMeta, Tier};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Recommended,
    Strict,
    Strictest,
}

impl Preset {
    pub fn parse(value: &str) -> Option<Preset> {
        match value {
            "recommended" => Some(Preset::Recommended),
            "strict" => Some(Preset::Strict),
            "strictest" => Some(Preset::Strictest),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Preset::Recommended => "recommended",
            Preset::Strict => "strict",
            Preset::Strictest => "strictest",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub level: Option<Level>,
    pub options: toml::Value,
}

#[derive(Debug, Clone)]
pub struct Config {
    preset: Preset,
    rules: HashMap<String, RuleConfig>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            preset: Preset::Recommended,
            rules: HashMap::new(),
        }
    }
}

impl Config {
    pub fn preset(preset: Preset) -> Config {
        Config {
            preset,
            rules: HashMap::new(),
        }
    }

    pub fn preset_value(&self) -> Preset {
        self.preset
    }

    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = preset;
    }

    pub fn from_toml_str(text: &str) -> Result<Config, String> {
        let value: toml::Value = toml::from_str(text).map_err(|error| error.to_string())?;
        let preset = configured_preset(&value)?;
        let rules = configured_rules(&value)?;
        Ok(Config { preset, rules })
    }

    pub fn from_file(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Config::from_toml_str(&text)
    }

    pub fn set_level(&mut self, rule: &str, level: Level) {
        let entry = self
            .rules
            .entry(rule.to_string())
            .or_insert_with(|| RuleConfig {
                level: None,
                options: empty_table(),
            });
        entry.level = Some(level);
    }

    pub fn effective_level(&self, meta: &RuleMeta) -> Level {
        let configured = self.rules.get(meta.name);
        if let Some(rule) = configured {
            if let Some(level) = rule.level {
                return level;
            }
        }
        match self.preset {
            Preset::Recommended => {
                if meta.tier == Tier::Static {
                    Level::Error
                } else {
                    Level::Allow
                }
            }
            Preset::Strict => {
                if meta.tier == Tier::Static {
                    Level::Error
                } else {
                    Level::Warn
                }
            }
            Preset::Strictest => Level::Error,
        }
    }

    pub fn rule_options(&self, name: &str) -> toml::Value {
        let configured = self.rules.get(name);
        configured
            .map(|rule| rule.options.clone())
            .unwrap_or_else(empty_table)
    }
}

fn configured_preset(value: &toml::Value) -> Result<Preset, String> {
    let configured = value.get("preset").and_then(toml::Value::as_str);
    let Some(preset) = configured else {
        return Ok(Preset::Recommended);
    };
    Preset::parse(preset).ok_or_else(|| {
        format!("unknown preset `{preset}`; expected recommended, strict, or strictest")
    })
}

fn configured_rules(value: &toml::Value) -> Result<HashMap<String, RuleConfig>, String> {
    let mut rules = HashMap::new();
    let configured = value.get("rules").and_then(toml::Value::as_table);
    let Some(configured) = configured else {
        return Ok(rules);
    };
    for (name, entry) in configured {
        rules.insert(name.clone(), rule_config(entry)?);
    }
    Ok(rules)
}

fn rule_config(entry: &toml::Value) -> Result<RuleConfig, String> {
    let shorthand = entry.as_str();
    if let Some(level) = shorthand {
        let parsed = parse_level(level)?;
        return Ok(RuleConfig {
            level: Some(parsed),
            options: empty_table(),
        });
    }
    let level = entry
        .get("level")
        .and_then(toml::Value::as_str)
        .map(parse_level)
        .transpose()?;
    Ok(RuleConfig {
        level,
        options: entry.clone(),
    })
}

fn empty_table() -> toml::Value {
    toml::Value::Table(toml::Table::new())
}

fn parse_level(value: &str) -> Result<Level, String> {
    Level::parse(value)
        .ok_or_else(|| format!("unknown rule level `{value}`; expected off, warn, or error"))
}
