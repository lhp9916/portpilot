use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::{ForwardRule, Language, Theme};

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_config_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub rules: Vec<ForwardRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_VERSION,
            language: Language::System,
            theme: Theme::System,
            close_to_tray: true,
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesFile {
    #[serde(default = "default_config_version")]
    pub schema_version: u32,
    pub rules: Vec<ForwardRule>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine the user config directory")]
    MissingConfigDirectory,
    #[error("could not read config: {0}")]
    Read(#[from] io::Error),
    #[error("could not parse config: {0}")]
    Parse(#[from] serde_json::Error),
}

pub struct LoadOutcome {
    pub config: AppConfig,
    pub warnings: Vec<String>,
    pub path: PathBuf,
}

pub fn app_dir() -> Result<PathBuf, ConfigError> {
    let dirs = BaseDirs::new().ok_or(ConfigError::MissingConfigDirectory)?;
    Ok(dirs.config_dir().join("PortPilot"))
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(app_dir()?.join("config.json"))
}

pub fn load() -> Result<LoadOutcome, ConfigError> {
    let path = config_path()?;
    let mut warnings = Vec::new();
    let config = match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<AppConfig>(&contents) {
            Ok(config) => config,
            Err(error) => {
                let backup = backup_corrupt_file(&path)?;
                warnings.push(format!(
                    "configuration parse failed ({error}); backup: {}",
                    backup.display()
                ));
                AppConfig::default()
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => AppConfig::default(),
        Err(error) => return Err(ConfigError::Read(error)),
    };

    Ok(LoadOutcome {
        config,
        warnings,
        path,
    })
}

pub fn save(config: &AppConfig) -> Result<PathBuf, ConfigError> {
    let path = config_path()?;
    save_to(config, &path)?;
    Ok(path)
}

pub fn save_to(config: &AppConfig, path: &Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(config)?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

pub fn export_rules(rules: &[ForwardRule], path: &Path) -> Result<(), ConfigError> {
    let file = RulesFile {
        schema_version: CONFIG_VERSION,
        rules: rules.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&file)?;
    fs::write(path, bytes)?;
    Ok(())
}

pub fn import_rules(path: &Path) -> Result<Vec<ForwardRule>, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let file: RulesFile = serde_json::from_str(&contents)?;
    Ok(file.rules)
}

fn backup_corrupt_file(path: &Path) -> io::Result<PathBuf> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup = path.with_file_name(format!("config.corrupt-{stamp}.json"));
    fs::rename(path, &backup)?;
    Ok(backup)
}

fn default_config_version() -> u32 {
    CONFIG_VERSION
}

fn default_close_to_tray() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Protocol;
    use uuid::Uuid;

    fn sample_rule() -> ForwardRule {
        ForwardRule {
            id: Uuid::new_v4(),
            name: "Web".to_owned(),
            protocol: Protocol::Tcp,
            bind_ip: "127.0.0.1".parse().unwrap(),
            bind_port: 8080,
            target_host: "localhost".to_owned(),
            target_port: 3000,
            enabled: true,
        }
    }

    #[test]
    fn round_trips_config_and_rules() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let mut config = AppConfig::default();
        config.rules.push(sample_rule());
        save_to(&config, &config_path).unwrap();
        let parsed: AppConfig =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(parsed.rules.len(), 1);
        assert_eq!(parsed.rules[0].bind_port, 8080);
    }

    #[test]
    fn overwrites_existing_config_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let mut config = AppConfig::default();
        save_to(&config, &config_path).unwrap();
        config.close_to_tray = false;
        save_to(&config, &config_path).unwrap();
        let parsed: AppConfig =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(!parsed.close_to_tray);
    }

    #[test]
    fn exports_and_imports_rules() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rules.json");
        let rule = sample_rule();
        export_rules(std::slice::from_ref(&rule), &path).unwrap();
        let imported = import_rules(&path).unwrap();
        assert_eq!(imported, vec![rule]);
    }
}
