use crate::{
    config::SiteConfig,
    image::{ImageOptions, build_image_url},
    markdown::{PostSection, contains_mermaid, count_words, reading_minutes, render_markdown},
};
use anyhow::{Context, Result, bail};
use chrono::{Datelike, NaiveDate};
use gray_matter::{Matter, engine::YAML};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fs, path::Path};
use walkdir::WalkDir;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frontmatter {
    title: String,
    #[serde(default)]
    title_lines: Vec<String>,
    summary: String,
    published_at: String,
    #[serde(default)]
    tags: Vec<String>,
    cover: Option<String>,
    #[serde(default)]
    featured: bool,
    featured_order: Option<i32>,
    #[serde(default)]
    draft: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub title_lines: Vec<String>,
    pub summary: String,
    pub published_at: NaiveDate,
    pub tags: Vec<String>,
    pub cover: Option<String>,
    pub featured: bool,
    pub featured_order: Option<i32>,
    pub reading_minutes: usize,
    pub content_html: String,
    pub sections: Vec<PostSection>,
    pub has_mermaid: bool,
}

impl Post {
    #[must_use]
    pub fn published_iso(&self) -> String {
        self.published_at.format("%Y-%m-%d").to_string()
    }

    #[must_use]
    pub fn published_rfc2822(&self) -> String {
        self.published_at
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .and_utc()
            .to_rfc2822()
    }

    #[must_use]
    pub fn published_zh(&self) -> String {
        format!(
            "{}年{}月{}日",
            self.published_at.year(),
            self.published_at.month(),
            self.published_at.day()
        )
    }
}

pub fn load_posts(content_dir: &Path, config: &SiteConfig) -> Result<Vec<Post>> {
    let matter = Matter::<YAML>::new();
    let mut posts = Vec::new();
    for entry in WalkDir::new(content_dir).min_depth(1).max_depth(1) {
        let entry = entry.with_context(|| format!("walk {}", content_dir.display()))?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("md" | "mdx")
            )
        {
            continue;
        }
        let raw =
            fs::read_to_string(path).with_context(|| format!("read post {}", path.display()))?;
        let parsed = matter
            .parse::<Frontmatter>(&raw)
            .with_context(|| format!("parse frontmatter {}", path.display()))?;
        let frontmatter = parsed
            .data
            .with_context(|| format!("post {} is missing YAML frontmatter", path.display()))?;
        if frontmatter.draft {
            continue;
        }
        let title = frontmatter.title.trim().to_owned();
        let summary = frontmatter.summary.trim().to_owned();
        if title.is_empty() || summary.is_empty() {
            bail!(
                "post {} must have non-empty title and summary",
                path.display()
            );
        }
        let published_at = NaiveDate::parse_from_str(&frontmatter.published_at, "%Y-%m-%d")
            .with_context(|| {
                format!(
                    "post {} has invalid publishedAt {:?}; expected YYYY-MM-DD",
                    path.display(),
                    frontmatter.published_at
                )
            })?;
        let slug = path
            .file_stem()
            .and_then(|value| value.to_str())
            .context("post filename must be valid UTF-8")?
            .to_owned();
        let rendered = render_markdown(config, &parsed.content);
        let word_count = count_words(&parsed.content);
        let cover = frontmatter.cover.as_deref().map(|value| {
            build_image_url(
                config,
                value,
                ImageOptions {
                    width: Some(960),
                    quality: Some(82),
                },
            )
        });
        posts.push(Post {
            slug,
            title,
            title_lines: frontmatter
                .title_lines
                .into_iter()
                .map(|line| line.trim().to_owned())
                .filter(|line| !line.is_empty())
                .collect(),
            summary,
            published_at,
            tags: frontmatter.tags,
            cover,
            featured: frontmatter.featured,
            featured_order: frontmatter.featured_order,
            reading_minutes: reading_minutes(word_count, config.words_per_minute),
            content_html: rendered.html,
            sections: rendered.sections,
            has_mermaid: contains_mermaid(&parsed.content),
        });
    }
    posts.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| a.slug.cmp(&b.slug))
    });
    Ok(posts)
}

#[must_use]
pub fn featured_posts(posts: &[Post], limit: usize) -> Vec<&Post> {
    let mut featured: Vec<_> = posts.iter().filter(|post| post.featured).collect();
    featured.sort_by(|a, b| {
        let order = a
            .featured_order
            .unwrap_or(i32::MAX)
            .cmp(&b.featured_order.unwrap_or(i32::MAX));
        if order == Ordering::Equal {
            b.published_at.cmp(&a.published_at)
        } else {
            order
        }
    });
    if featured.len() >= limit {
        featured.truncate(limit);
        featured
    } else {
        posts.iter().take(limit).collect()
    }
}

#[must_use]
pub fn related_posts<'a>(posts: &'a [Post], post: &Post, limit: usize) -> Vec<&'a Post> {
    let tags: Vec<_> = post.tags.iter().map(|tag| tag.to_lowercase()).collect();
    let mut related: Vec<_> = posts
        .iter()
        .filter(|candidate| candidate.slug != post.slug)
        .filter_map(|candidate| {
            let overlap = candidate
                .tags
                .iter()
                .filter(|tag| tags.contains(&tag.to_lowercase()))
                .count();
            (overlap > 0).then_some((candidate, overlap))
        })
        .collect();
    related.sort_by(|(a, a_overlap), (b, b_overlap)| {
        b_overlap
            .cmp(a_overlap)
            .then_with(|| b.published_at.cmp(&a.published_at))
            .then_with(|| a.slug.cmp(&b.slug))
    });
    related
        .into_iter()
        .take(limit)
        .map(|(post, _)| post)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GiscusConfig, ImageConfig, VerificationConfig};

    fn config() -> SiteConfig {
        SiteConfig {
            site_name: "test".into(),
            site_title: "test".into(),
            site_url: "https://example.com".into(),
            base_path: String::new(),
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
    fn repository_posts_are_loaded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let posts = load_posts(&root.join("content/posts"), &config()).expect("load posts");
        assert_eq!(posts.len(), 7);
        assert!(
            posts
                .windows(2)
                .all(|pair| pair[0].published_at >= pair[1].published_at)
        );
        assert_eq!(featured_posts(&posts, 3)[0].slug, "neocode-agent");
    }

    #[test]
    fn legacy_fixture_matches_all_public_posts() {
        use sha2::{Digest, Sha256};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let baseline: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("tests/fixtures/legacy-baseline.json"))
                .expect("read baseline"),
        )
        .expect("parse baseline");
        let posts = load_posts(&root.join("content/posts"), &config()).expect("load posts");
        let expected = baseline["posts"].as_array().expect("posts array");
        assert_eq!(posts.len(), expected.len());

        for fixture in expected {
            let slug = fixture["slug"].as_str().expect("slug");
            let post = posts.iter().find(|post| post.slug == slug).expect("post");
            let source = fs::read(root.join(fixture["file"].as_str().expect("file")))
                .expect("read post source");
            let hash = format!("{:x}", Sha256::digest(&source));
            assert_eq!(hash, fixture["sha256"].as_str().expect("sha256"));
            assert_eq!(
                source.len() as u64,
                fixture["bytes"].as_u64().expect("bytes")
            );
            assert_eq!(
                post.title,
                fixture["frontmatter"]["title"].as_str().expect("title")
            );
            assert_eq!(
                post.summary,
                fixture["frontmatter"]["summary"].as_str().expect("summary")
            );
            assert_eq!(
                post.published_iso(),
                &fixture["frontmatter"]["publishedAt"]
                    .as_str()
                    .expect("publishedAt")[..10]
            );
            let tags = fixture["frontmatter"]["tags"]
                .as_array()
                .expect("tags")
                .iter()
                .map(|tag| tag.as_str().expect("tag"))
                .collect::<Vec<_>>();
            assert_eq!(
                post.tags.iter().map(String::as_str).collect::<Vec<_>>(),
                tags
            );
        }
    }

    #[test]
    fn reading_times_match_legacy_javascript_for_every_post() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let posts = load_posts(&root.join("content/posts"), &config()).expect("load posts");
        let expected = [
            ("document-guides", 1),
            ("hooks-lifecycle-control-layer", 3),
            ("human-in-the-loop-neocode", 2),
            ("national-college-entrance-examination", 1),
            ("neocode-agent", 3),
            ("skills-agent-brain", 2),
            ("subagent-from-dag-to-inline", 2),
        ];
        for (slug, minutes) in expected {
            assert_eq!(
                posts
                    .iter()
                    .find(|post| post.slug == slug)
                    .expect("post")
                    .reading_minutes,
                minutes,
                "{slug}"
            );
        }
    }

    #[test]
    fn related_posts_have_stable_overlap_date_and_slug_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let posts = load_posts(&root.join("content/posts"), &config()).expect("load posts");
        let source = posts
            .iter()
            .find(|post| post.slug == "subagent-from-dag-to-inline")
            .expect("source post");
        let related = related_posts(&posts, source, 3)
            .into_iter()
            .map(|post| post.slug.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            related,
            [
                "document-guides",
                "neocode-agent",
                "human-in-the-loop-neocode"
            ]
        );
    }

    #[test]
    fn drafts_are_skipped_and_frontmatter_errors_include_the_file() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("blog-core-{suffix}"));
        fs::create_dir_all(&dir).expect("create temp fixture");
        fs::write(
            dir.join("draft.md"),
            "---\ntitle: Draft\nsummary: Hidden\npublishedAt: 2026-01-01\ndraft: true\n---\n# Hidden",
        )
        .expect("write draft");
        assert!(
            load_posts(&dir, &config())
                .expect("load draft dir")
                .is_empty()
        );
        fs::write(
            dir.join("broken.md"),
            "---\ntitle: Broken\nsummary: Invalid date\npublishedAt: not-a-date\n---\n# Broken",
        )
        .expect("write broken fixture");
        let error = load_posts(&dir, &config()).expect_err("invalid frontmatter must fail");
        let message = format!("{error:#}");
        assert!(message.contains("broken.md"));
        assert!(message.contains("publishedAt"));
        fs::remove_dir_all(dir).expect("remove temp fixture");
    }
}
