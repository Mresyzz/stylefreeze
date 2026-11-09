use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleFreezeConfig {
    #[serde(default = "default_true")]
    pub preserve_comments: bool,

    #[serde(default = "default_true")]
    pub normalize_quotes: bool,

    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_exclude() -> Vec<String> {
    vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/dist/**".to_string(),
        "**/.venv/**".to_string(),
    ]
}

impl Default for StyleFreezeConfig {
    fn default() -> Self {
        Self {
            preserve_comments: true,
            normalize_quotes: true,
            exclude: default_exclude(),
        }
    }
}

impl StyleFreezeConfig {
    pub fn load_from_dir(dir: &Path) -> Self {
        let config_path = dir.join("stylefreeze.toml");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(cfg) = toml::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn write_default(path: &Path) -> anyhow::Result<()> {
        let content = r#"# StyleFreeze Configuration File
# Documentation: https://github.com/Mresyzz/stylefreeze

# Automatically preserve inline and block comments that an AI agent drops
preserve_comments = true

# Revert non-semantic quote changes (e.g. single vs double quotes)
normalize_quotes = true

# Paths to exclude from surgical filtering and audits
exclude = [
    "**/node_modules/**",
    "**/target/**",
    "**/dist/**",
    "**/.venv/**",
]
"#;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization_and_defaults() {
        let default_cfg = StyleFreezeConfig::default();
        assert!(default_cfg.preserve_comments);
        assert!(default_cfg.normalize_quotes);
        assert_eq!(default_cfg.exclude.len(), 4);

        let toml_str = toml::to_string(&default_cfg).unwrap();
        let parsed: StyleFreezeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.preserve_comments, default_cfg.preserve_comments);
    }
}
