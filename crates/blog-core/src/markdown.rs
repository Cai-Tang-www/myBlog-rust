use crate::{
    config::SiteConfig,
    image::{ImageOptions, build_image_url},
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, html};
use regex::{Captures, Regex};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PostSection {
    pub id: String,
    pub title: String,
    pub level: u8,
}

pub struct RenderedMarkdown {
    pub html: String,
    pub sections: Vec<PostSection>,
}

#[must_use]
pub fn render_markdown(config: &SiteConfig, markdown: &str) -> RenderedMarkdown {
    let sections = extract_heading_sections(markdown);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(markdown, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    let output = inject_heading_ids(&output, &sections);
    let output = hydrate_markdown_images(config, &output);
    RenderedMarkdown {
        html: output,
        sections,
    }
}

#[must_use]
pub fn extract_heading_sections(markdown: &str) -> Vec<PostSection> {
    let mut sections = Vec::new();
    let mut in_fence = false;
    let mut index = 0;
    let heading = Regex::new(r"^\s*(#{1,3})\s+(.+?)\s*#*\s*$").expect("valid heading regex");
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some(capture) = heading.captures(line) else {
            continue;
        };
        let title = normalize_heading_text(&capture[2]);
        if title.is_empty() {
            continue;
        }
        index += 1;
        sections.push(PostSection {
            id: format!("section-{index}"),
            title,
            level: capture[1].len() as u8,
        });
    }
    sections
}

fn normalize_heading_text(source: &str) -> String {
    let code = Regex::new(r"`([^`]+)`").expect("valid code regex");
    let links = Regex::new(r"\[([^\]]+)\]\([^)]+\)").expect("valid link regex");
    let markers = Regex::new(r"[*_~]").expect("valid marker regex");
    let text = code.replace_all(source, "$1");
    let text = links.replace_all(&text, "$1");
    markers.replace_all(&text, "").trim().to_owned()
}

fn inject_heading_ids(html: &str, sections: &[PostSection]) -> String {
    let heading = Regex::new(r"(?i)<h([1-3])([^>]*)>").expect("valid html heading regex");
    let mut section_index = 0;
    heading
        .replace_all(html, |capture: &Captures<'_>| {
            if capture[2].contains("id=") {
                return capture[0].to_owned();
            }
            let level = capture[1].parse::<u8>().unwrap_or_default();
            while section_index < sections.len() && sections[section_index].level != level {
                section_index += 1;
            }
            let Some(section) = sections.get(section_index) else {
                return capture[0].to_owned();
            };
            section_index += 1;
            format!("<h{} id=\"{}\"{}>", &capture[1], section.id, &capture[2])
        })
        .into_owned()
}

fn hydrate_markdown_images(config: &SiteConfig, html: &str) -> String {
    let image = Regex::new(r#"(?i)<img([^>]*?)src="([^"]+)"([^>]*)>"#).expect("valid image regex");
    image
        .replace_all(html, |capture: &Captures<'_>| {
            let source = build_image_url(config, &capture[2], ImageOptions::default());
            let attributes = format!("{}{}", &capture[1], &capture[3]);
            let loading = if attributes.contains("loading=") {
                String::new()
            } else {
                " loading=\"lazy\"".to_owned()
            };
            let decoding = if attributes.contains("decoding=") {
                String::new()
            } else {
                " decoding=\"async\"".to_owned()
            };
            format!(
                "<img{} src=\"{}\"{}{}>",
                &capture[1],
                source,
                &capture[3],
                format_args!("{loading}{decoding}")
            )
        })
        .into_owned()
}

#[must_use]
pub fn count_words(markdown: &str) -> usize {
    let fenced = Regex::new(r"(?s)```.*?```").expect("valid fence regex");
    let inline = Regex::new(r"`[^`]*`").expect("valid inline regex");
    let images = Regex::new(r"!\[[^\]]*\]\([^)]+\)").expect("valid image regex");
    let links = Regex::new(r"\[[^\]]*\]\([^)]+\)").expect("valid link regex");
    let markers = Regex::new(r"[>#*_~\-]").expect("valid marker regex");
    let spaces = Regex::new(r"\s+").expect("valid whitespace regex");
    let text = fenced.replace_all(markdown, " ");
    let text = inline.replace_all(&text, " ");
    let text = images.replace_all(&text, " ");
    let text = links.replace_all(&text, " ");
    let text = markers.replace_all(&text, " ");
    let text = spaces.replace_all(&text, " ");
    let normalized = text.trim();
    if normalized.is_empty() {
        0
    } else {
        normalized.split(' ').count()
    }
}

#[must_use]
pub fn reading_minutes(word_count: usize, words_per_minute: usize) -> usize {
    if word_count == 0 || words_per_minute == 0 {
        return 1;
    }
    // Match JavaScript Math.round(words / wordsPerMinute), with a minimum of one.
    ((word_count * 2 + words_per_minute) / (words_per_minute * 2)).max(1)
}

#[must_use]
pub fn contains_mermaid(markdown: &str) -> bool {
    let mut parser = Parser::new_ext(markdown, Options::all());
    parser.any(|event| {
        matches!(
            event,
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                if info.trim().eq_ignore_ascii_case("mermaid")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_skip_fenced_code() {
        let sections = extract_heading_sections("# A\n```md\n## ignored\n```\n## `B` and **C**");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].id, "section-2");
        assert_eq!(sections[1].title, "B and C");
    }

    #[test]
    fn legacy_reading_time_rounds_like_javascript() {
        assert_eq!(reading_minutes(1, 240), 1);
        assert_eq!(reading_minutes(359, 240), 1);
        assert_eq!(reading_minutes(360, 240), 2);
    }
}
