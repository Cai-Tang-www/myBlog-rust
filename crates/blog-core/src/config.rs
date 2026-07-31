use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    pub site_name: String,
    pub site_title: String,
    pub site_url: String,
    #[serde(default)]
    pub base_path: String,
    pub language: String,
    pub locale: String,
    pub description: String,
    pub author: String,
    pub twitter: String,
    pub github_url: String,
    pub og_image: String,
    #[serde(default = "default_words_per_minute")]
    pub words_per_minute: usize,
    #[serde(default)]
    pub image: ImageConfig,
    #[serde(default)]
    pub verification: VerificationConfig,
    #[serde(default)]
    pub giscus: GiscusConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ImageConfig {
    #[serde(default)]
    pub provider: ImageProvider,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageProvider {
    Qiniu,
    Volc,
    #[default]
    None,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VerificationConfig {
    #[serde(default)]
    pub bing: String,
    #[serde(default)]
    pub baidu: String,
    #[serde(default)]
    pub so360: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GiscusConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub category_id: String,
    #[serde(default = "default_giscus_mapping")]
    pub mapping: String,
    #[serde(default = "default_giscus_theme")]
    pub theme: String,
}

const fn default_words_per_minute() -> usize {
    240
}

fn default_giscus_mapping() -> String {
    "pathname".to_owned()
}

fn default_giscus_theme() -> String {
    "preferred_color_scheme".to_owned()
}

impl SiteConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read site config {}", path.display()))?;
        let mut config: Self = toml::from_str(&raw)
            .with_context(|| format!("parse site config {}", path.display()))?;
        config.site_url = config.site_url.trim_end_matches('/').to_owned();
        config.base_path = normalize_base_path(&config.base_path);
        config.image.base_url = config.image.base_url.trim_end_matches('/').to_owned();
        Ok(config)
    }

    #[must_use]
    pub fn with_base_path(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            return path.to_owned();
        }
        let normalized = format!("/{}", path.trim_start_matches('/'));
        if self.base_path.is_empty()
            || normalized == self.base_path
            || normalized.starts_with(&format!("{}/", self.base_path))
        {
            normalized
        } else {
            format!("{}{}", self.base_path, normalized)
        }
    }

    #[must_use]
    pub fn absolute_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_owned()
        } else {
            format!("{}{}", self.site_url, self.with_base_path(path))
        }
    }
}

#[must_use]
pub fn normalize_base_path(value: &str) -> String {
    let trimmed = value.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(base_path: &str) -> SiteConfig {
        SiteConfig {
            site_name: "test".into(),
            site_title: "test".into(),
            site_url: "https://example.com".into(),
            base_path: normalize_base_path(base_path),
            language: "zh-CN".into(),
            locale: "zh_CN".into(),
            description: String::new(),
            author: String::new(),
            twitter: String::new(),
            github_url: String::new(),
            og_image: "/og.png".into(),
            words_per_minute: 240,
            image: ImageConfig::default(),
            verification: VerificationConfig::default(),
            giscus: GiscusConfig::default(),
        }
    }

    #[test]
    fn base_path_is_normalized_and_not_duplicated() {
        let site = config("/blog/");
        assert_eq!(site.with_base_path("/"), "/blog/");
        assert_eq!(site.with_base_path("/posts/a/"), "/blog/posts/a/");
        assert_eq!(site.with_base_path("/blog/posts/a/"), "/blog/posts/a/");
    }
}
