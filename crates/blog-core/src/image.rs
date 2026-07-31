use crate::config::{ImageProvider, SiteConfig};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

#[derive(Debug, Clone, Copy, Default)]
pub struct ImageOptions {
    pub width: Option<u32>,
    pub quality: Option<u8>,
}

#[must_use]
pub fn build_image_url(config: &SiteConfig, input: &str, options: ImageOptions) -> String {
    if input.is_empty() {
        return String::new();
    }

    let is_http = input.starts_with("http://") || input.starts_with("https://");
    let source = if is_http {
        input.to_owned()
    } else if config.image.base_url.is_empty() {
        config.with_base_path(input)
    } else {
        format!(
            "{}/{}",
            config.image.base_url,
            input.trim_start_matches('/')
        )
    };

    if !is_http && config.image.base_url.is_empty() {
        return source;
    }

    let width = options.width.unwrap_or(1280).clamp(200, 4096);
    let quality = options.quality.unwrap_or(82).clamp(40, 95);
    match config.image.provider {
        ImageProvider::Qiniu => {
            let command = format!("imageView2/2/w/{width}/q/{quality}/format/webp");
            if source.contains('?') {
                format!("{source}|{command}")
            } else {
                format!("{source}?{command}")
            }
        }
        ImageProvider::Volc => {
            let command = format!("image/resize,w_{width}/quality,q_{quality}/format,webp");
            let encoded = utf8_percent_encode(&command, NON_ALPHANUMERIC);
            if source.contains('?') {
                format!("{source}&x-tos-process={encoded}")
            } else {
                format!("{source}?x-tos-process={encoded}")
            }
        }
        ImageProvider::None => source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GiscusConfig, ImageConfig, VerificationConfig};

    fn config(provider: ImageProvider, base_url: &str) -> SiteConfig {
        SiteConfig {
            site_name: "test".into(),
            site_title: "test".into(),
            site_url: "https://example.com".into(),
            base_path: "/repo".into(),
            language: "zh-CN".into(),
            locale: "zh_CN".into(),
            description: String::new(),
            author: String::new(),
            twitter: String::new(),
            github_url: String::new(),
            og_image: "/og.png".into(),
            words_per_minute: 240,
            image: ImageConfig {
                provider,
                base_url: base_url.into(),
            },
            verification: VerificationConfig::default(),
            giscus: GiscusConfig::default(),
        }
    }

    #[test]
    fn local_images_receive_base_path() {
        assert_eq!(
            build_image_url(
                &config(ImageProvider::None, ""),
                "/a.png",
                ImageOptions::default()
            ),
            "/repo/a.png"
        );
    }

    #[test]
    fn qiniu_processing_is_appended() {
        assert!(
            build_image_url(
                &config(ImageProvider::Qiniu, "https://cdn.example.com"),
                "/a.png",
                ImageOptions {
                    width: Some(960),
                    quality: Some(82)
                }
            )
            .ends_with("?imageView2/2/w/960/q/82/format/webp")
        );
    }
}
