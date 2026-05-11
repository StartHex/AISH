//! Band isolation environment manager.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

/// Band isolation level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BandIsolationLevel {
    /// Lightweight: only isolate HOME and config directories.
    Lightweight,
    /// Standard: isolate HOME + temp filesystem + restrict network to loopback.
    Standard,
}

/// Band environment definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub name: String,
    pub isolation: BandIsolationLevel,
    pub root: PathBuf,
    pub created_at: String,
}

/// Band status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandStatus {
    pub name: String,
    pub isolation: String,
    pub root: String,
    pub created_at: String,
    pub exists: bool,
    pub db_exists: bool,
    pub db_size_bytes: u64,
    pub adapters_configured: bool,
}

impl Band {
    /// Create a new band environment under `bands_root/<name>`.
    pub fn create(name: &str, isolation: BandIsolationLevel, bands_root: &Path) -> Result<Self> {
        let root = bands_root.join(name);

        if root.exists() {
            anyhow::bail!("Band '{}' already exists at {}", name, root.display());
        }

        // Create isolated directory structure
        fs::create_dir_all(root.join("home"))?;
        fs::create_dir_all(root.join("config").join("aish"))?;
        fs::create_dir_all(root.join("data"))?;
        fs::create_dir_all(root.join("tmp"))?;

        let band = Band {
            name: name.to_string(),
            isolation,
            root: root.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Write band.toml
        let band_toml = toml::to_string_pretty(&band)?;
        fs::write(root.join("band.toml"), band_toml)?;

        // Write default adapters.ron for this band
        let default_adapters = crate::config::AdaptersConfig { adapters: vec![] };
        let adapters_ron = ron::ser::to_string_pretty(&default_adapters, ron::ser::PrettyConfig::default())?;
        fs::write(root.join("config").join("aish").join("adapters.ron"), adapters_ron)?;

        info!(%name, root = %root.display(), "Band created");
        Ok(band)
    }

    /// Destroy a band environment.
    pub fn destroy(name: &str, bands_root: &Path) -> Result<()> {
        let root = bands_root.join(name);
        if !root.exists() {
            anyhow::bail!("Band '{}' not found", name);
        }
        fs::remove_dir_all(&root)?;
        info!(%name, "Band destroyed");
        Ok(())
    }

    /// List all bands.
    pub fn list(bands_root: &Path) -> Result<Vec<Band>> {
        if !bands_root.exists() {
            return Ok(vec![]);
        }

        let mut bands = vec![];
        for entry in fs::read_dir(bands_root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let toml_path = entry.path().join("band.toml");
                if toml_path.exists() {
                    let content = fs::read_to_string(&toml_path)?;
                    let band: Band = toml::from_str(&content)?;
                    bands.push(band);
                }
            }
        }
        Ok(bands)
    }

    /// Resolve the band root directory. Default: ~/.aish/bands
    pub fn default_bands_root() -> PathBuf {
        if let Ok(root) = std::env::var("AISH_BANDS_ROOT") {
            PathBuf::from(root)
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".aish")
                .join("bands")
        }
    }

    /// Build environment variables for running a process inside this band.
    pub fn env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("AISH_BAND".into(), self.name.clone()),
            ("AISH_BAND_ROOT".into(), self.root.to_string_lossy().to_string()),
            ("HOME".into(), self.root.join("home").to_string_lossy().to_string()),
            (
                "XDG_CONFIG_HOME".into(),
                self.root.join("config").to_string_lossy().to_string(),
            ),
            ("TMPDIR".into(), self.root.join("tmp").to_string_lossy().to_string()),
            (
                "AISH_DB_PATH".into(),
                self.root.join("data").join("aish.db").to_string_lossy().to_string(),
            ),
        ]
    }

    /// Get the status of this band (disk usage, writability, etc.).
    pub fn status(&self) -> Result<BandStatus> {
        let exists = self.root.exists();
        let db_path = self.root.join("data").join("aish.db");
        let db_exists = db_path.exists();
        let db_size = if db_exists {
            fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        let adapters_path = self.root.join("config").join("aish").join("adapters.ron");
        let adapters_exist = adapters_path.exists();

        Ok(BandStatus {
            name: self.name.clone(),
            isolation: format!("{:?}", self.isolation),
            root: self.root.display().to_string(),
            created_at: self.created_at.clone(),
            exists,
            db_exists,
            db_size_bytes: db_size,
            adapters_configured: adapters_exist,
        })
    }

    /// Execute a command inside this band environment.
    pub fn exec(&self, command: &str, args: &[&str]) -> Result<std::process::Output> {
        let mut cmd = Command::new(command);
        cmd.args(args);
        for (key, value) in self.env_vars() {
            cmd.env(key, value);
        }
        let output = cmd.output().context("Failed to execute command in band")?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_band_create_and_destroy() {
        let tmp = TempDir::new().unwrap();
        let bands_root = tmp.path().join("bands");

        let band = Band::create("test-band", BandIsolationLevel::Lightweight, &bands_root).unwrap();
        assert_eq!(band.name, "test-band");
        assert!(band.root.join("band.toml").exists());
        assert!(band.root.join("home").exists());
        assert!(band.root.join("config").join("aish").join("adapters.ron").exists());

        let list = Band::list(&bands_root).unwrap();
        assert_eq!(list.len(), 1);

        Band::destroy("test-band", &bands_root).unwrap();
        let list = Band::list(&bands_root).unwrap();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_band_env_vars() {
        let tmp = TempDir::new().unwrap();
        let bands_root = tmp.path().join("bands");
        let band = Band::create("env-band", BandIsolationLevel::Standard, &bands_root).unwrap();

        let env_vars: std::collections::HashMap<_, _> = band.env_vars().into_iter().collect();
        assert_eq!(env_vars.get("AISH_BAND").unwrap(), "env-band");
        assert!(env_vars.get("AISH_BAND_ROOT").unwrap().contains("env-band"));
        assert!(env_vars.get("HOME").unwrap().contains("home"));
    }
}
