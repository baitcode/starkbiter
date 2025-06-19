use config::{Config, ConfigError};
use serde::Deserialize;

use super::*;

#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub struct StarkbiterConfig {
    /// The path to the directory where the bindings will be generated.
    pub bindings_path: PathBuf,
    /// Whether to generate bindings for submodules.
    pub submodules: bool,
    /// Ignore interfaces flag
    pub ignore_interfaces: bool,
}

impl StarkbiterConfig {
    pub fn _new_mock_config() -> Self {
        StarkbiterConfig {
            bindings_path: PathBuf::from("src").join("bindings"),
            submodules: false,
            ignore_interfaces: false,
        }
    }

    pub fn _new_mock_config_with_submodules() -> Self {
        StarkbiterConfig {
            bindings_path: PathBuf::from("src"),
            submodules: true,
            ignore_interfaces: false,
        }
    }
}

impl StarkbiterConfig {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(config::File::with_name("arbiter.toml"))
            .build()?;
        Ok(s.into())
    }
}

impl Default for StarkbiterConfig {
    fn default() -> Self {
        Self {
            bindings_path: PathBuf::from("src"),
            submodules: false,
            ignore_interfaces: false,
        }
    }
}

impl From<config::Config> for StarkbiterConfig {
    fn from(config: config::Config) -> Self {
        // Here you need to convert the `config::Config` into `ArbiterConfig`
        StarkbiterConfig {
            bindings_path: PathBuf::from("src").join("bindings"),
            submodules: config.get_bool("submodules").unwrap_or(false),
            ignore_interfaces: config.get_bool("ignore_interfaces").unwrap_or(true),
        }
    }
}
