use anyhow::Context;
use directories::ProjectDirs;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct RakalyConfig {
    pub user: String,
    pub api_key: String,

    #[serde(default = "default_base_url")]
    pub base_url: String,
}

pub fn default_base_url() -> String {
    String::from("https://pdx.tools")
}

pub fn read_config<P: AsRef<Path>>(location: P) -> anyhow::Result<RakalyConfig> {
    let path = location.as_ref();
    let config_data =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config = parse_config(&config_data)
        .with_context(|| format!("Malformatted config file: {}", path.display()))?;
    Ok(config)
}

pub fn parse_config(data: &[u8]) -> anyhow::Result<RakalyConfig> {
    toml::de::from_slice(data).context("unable to deserialize toml config")
}

pub fn default_config_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "Rakaly", "Rakaly") {
        let default_path = proj_dirs.config_dir().join("config.toml");
        if default_path.exists() {
            return Some(default_path);
        }
    }

    None
}
