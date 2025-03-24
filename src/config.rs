use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub service_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service_url: "http://localhost:9673".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        
        if !config_path.exists() {
            let config = Config::default();
            std::fs::create_dir_all(config_path.parent().unwrap())?;
            std::fs::write(
                &config_path,
                serde_json::to_string_pretty(&config)?,
            )?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(config_path)
            .context("Failed to read config file")?;
        let config: Config = serde_json::from_str(&content)
            .context("Failed to parse config file")?;
        Ok(config)
    }

    #[cfg(test)]
    pub fn with_url(service_url: String) -> Self {
        Self { service_url }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".config").join("whisper-client").join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.service_url, "http://localhost:9673");
    }

    #[test]
    fn test_config_with_custom_url() {
        let url = "http://example.com:8000".to_string();
        let config = Config::with_url(url.clone());
        assert_eq!(config.service_url, url);
    }

    #[test]
    fn test_config_load_create_default() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("HOME", temp_dir.path());

        let config = Config::load().unwrap();
        assert_eq!(config.service_url, "http://localhost:9673");

        // Verify file was created
        let config_path = temp_dir.path().join(".config").join("whisper-client").join("config.json");
        assert!(config_path.exists());
    }
}
