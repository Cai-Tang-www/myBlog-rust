use anyhow::{Context, Result};
use askama::Template;
use blog_core::{GiscusConfig, Post, SiteConfig, featured_posts, load_posts, related_posts};
use chrono::{Datelike, Utc};
use clap::{Parser, Subcommand};
use html_escape::encode_text;
use regex::Regex;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Parser)]
#[command(name = "blog", about = "Build and preview the Rust myBlog rewrite")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Build,
    Check,
    Serve {
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
}

#[derive(Template)]
#[template(path = "page.html")]
struct PageTemplate<'a> {
    language: &'a str,
    locale: &'a str,
    title: &'a str,
    site_title: &'a str,
    description: &'a str,
    author: &'a str,
    keywords: &'a str,
    canonical: &'a str,
    og_type: &'a str,
    og_image: &'a str,
    site_name: &'a str,
    base_path: &'a str,
    page_kind: &'a str,
    home_url: &'a str,
    blog_url: &'a str,
    contact_url: &'a str,
    github_url: &'a str,
    rss_url: &'a str,
    favicon_url: &'a str,
    client_js: &'a str,
    client_wasm: &'a str,
    stylesheets: &'a [String],
    json_ld: &'a [String],
    body: &'a str,
    year: i32,
    bing: &'a str,
    baidu: &'a str,
    so360: &'a str,
    giscus_enabled: bool,
    giscus_repo: &'a str,
    giscus_repo_id: &'a str,
    giscus_category: &'a str,
    giscus_category_id: &'a str,
    giscus_mapping: &'a str,
    giscus_theme: &'a str,
}

struct PageSpec<'a> {
    title: &'a str,
    description: &'a str,
    keywords: String,
    path: &'a str,
    kind: &'a str,
    og_type: &'a str,
    og_image: String,
    page_style: &'a str,
    json_ld: Vec<String>,
    body: String,
}

fn workspace_root() -> Result<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("resolve workspace root")
}

fn esc(value: &str) -> String {
    encode_text(value).into_owned()
}

fn url(config: &SiteConfig, path: &str) -> String {
    config.with_base_path(path)
}

fn write_route(dist: &Path, route: &str, contents: &str) -> Result<()> {
    let file = if route == "/" {
        dist.join("index.html")
    } else if route.ends_with(".html") || route.ends_with(".xml") || route.ends_with(".txt") {
        dist.join(route.trim_start_matches('/'))
    } else {
        dist.join(route.trim_matches('/')).join("index.html")
    };
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&file, contents).with_context(|| format!("write {}", file.display()))
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(source) {
        let entry = entry.with_context(|| format!("walk {}", source.display()))?;
        let relative = entry.path().strip_prefix(source)?;
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &destination).with_context(|| {
                format!(
                    "copy {} to {}",
                    entry.path().display(),
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}

fn post_card(post: &Post, index: usize, config: &SiteConfig) -> String {
    let mut html = format!(
        "<article class=\"post-card post-block\" style=\"animation-delay:{}ms\">",
        120 + index * 80
    );
    if let Some(cover) = &post.cover {
        html.push_str(&format!("<div class=\"post-card-cover-wrap\"><img class=\"post-card-cover\" src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\"></div>", esc(cover), esc(&post.title)));
    }
    html.push_str(&format!(
        "<p class=\"post-card-meta\"><span>{}</span><span>·</span><span>{} 分钟阅读</span></p><h3 class=\"post-card-title\"><a href=\"{}\">{}</a></h3><p class=\"post-card-summary\">{}</p><div class=\"post-card-tags\">",
        post.published_zh(), post.reading_minutes, url(config, &format!("/blog/{}/", post.slug)), esc(&post.title), esc(&post.summary)
    ));
    for tag in &post.tags {
        html.push_str(&format!("<span class=\"chip\">{}</span>", esc(tag)));
    }
    html.push_str("</div></article>");
    html
}

fn home_body(posts: &[Post], config: &SiteConfig) -> String {
    let featured = featured_posts(posts, 3);
    let mut groups: BTreeMap<&str, Vec<&Post>> = BTreeMap::new();
    for key in ["agent", "engineering", "architecture"] {
        groups.insert(key, Vec::new());
    }
    for post in posts {
        let key = match post.slug.as_str() {
            "hooks-lifecycle-control-layer"
            | "subagent-from-dag-to-inline"
            | "skills-agent-brain" => "agent",
            "human-in-the-loop-neocode" => "engineering",
            _ => "architecture",
        };
        groups.entry(key).or_default().push(post);
    }
    let mut html = format!(
        r#"<div class="page"><section class="container hero"><div class="decorLayer" aria-hidden="true"><span class="orb orbA"></span><span class="orb orbB"></span><span class="orb orbC"></span></div><div class="heroLead"><p class="kicker">CA1_TANG / ENGINEERING NOTES</p><h1 class="heroTypingTitle"><span class="heroTypingMain">从想法到交付</span><span class="heroTypingSub">先回答要解决什么问题</span></h1><div class="heroActions"><a href="{}" class="button-primary">进入文章库</a></div></div><aside class="heroPanel"><div class="profileHead"><img src="{}" alt="CA1_TANG avatar" class="profileAvatar" loading="lazy" decoding="async"><div class="profileTitleWrap"><h2>CA1_TANG</h2><p class="profileSubtitle">Go Backend × Agent Engineering</p></div></div><p class="profileIntro">主要做后端工程与系统实现，最近聚焦 Agent Runtime、SubAgent 编排、工具链与可交付工程实践。</p><p class="profileTagline">野鸡学校&nbsp; | &nbsp;东莞留子&nbsp; | &nbsp;IMSB&nbsp; | &nbsp;绩点倒数&nbsp; | &nbsp;中中混血</p><div class="stackChips">"#,
        url(config, "/blog/"),
        url(config, "/images/avatar-ca1.png")
    );
    for stack in ["Go", "Gin", "MySQL", "Redis", "Docker", "Agent Runtime"] {
        html.push_str(&format!("<span class=\"chip\">{stack}</span>"));
    }
    html.push_str(&format!(r##"</div><div class="profileActions"><a class="profileLink" href="{}" target="_blank" rel="noreferrer">GitHub 主页 →</a><a href="#" class="resumeLink" data-resume-link>看看简历 →</a></div></aside></section><section class="container postSection"><div class="sectionHeader"><h2>精选文章</h2><a href="{}" class="textLink">查看全部 →</a></div><div class="postGrid">"##, config.github_url, url(config,"/blog/")));
    for (index, post) in featured.into_iter().enumerate() {
        html.push_str(&post_card(post, index, config));
    }
    html.push_str(r#"</div></section><section class="container aboutSection"><div class="aboutDecor" aria-hidden="true"><span class="aboutIcon">📝</span></div><h2 class="aboutTitle">关于本站</h2><p class="aboutText">这是一个由 Rust 静态生成器和 Rust/WASM 构建的个人技术博客，内容涵盖 Agent 开发、后端工程实践与系统架构思考。本模块使用 CSS <code>float: right</code> 实现装饰图标与文字环绕排版——这是一种经典的 CSS 布局方式，与现代 Flex/Grid 布局形成互补。项目源码完全开放，欢迎通过留言板交流想法。</p></section><section id="plan" class="container phaseSection"><div class="sectionHeader"><h2>文章板块</h2><p>按主题组织内容，所有条目都可点击进入详情。</p></div><div class="phaseGrid">"#);
    let category_data = [
        (
            "agent",
            "板块 1",
            "Agent开发",
            "聚焦 SubAgent、Skills、Hooks 等核心能力实现与演进。",
        ),
        (
            "engineering",
            "板块 2",
            "工程经验",
            "记录人机协作、交付节奏、复盘方法和团队实践。",
        ),
        (
            "architecture",
            "板块 3",
            "架构思考",
            "围绕系统边界、能力分层与长期演进做结构化思考。",
        ),
    ];
    for (index, (id, phase, title, note)) in category_data.iter().enumerate() {
        let items = &groups[id];
        html.push_str(&format!("<article class=\"phaseCard\" style=\"animation-delay:{}ms\"><p class=\"phaseMeta\"><span>{phase}</span><span>{} 篇</span></p><h3>{title}</h3><p class=\"phaseIntro\">{note}</p><ul class=\"phaseLinks\">", index*90+160,items.len()));
        for post in items {
            html.push_str(&format!(
                "<li><a href=\"{}\" class=\"phaseLink\">{}</a></li>",
                url(config, &format!("/blog/{}/", post.slug)),
                esc(&post.title)
            ));
        }
        html.push_str("</ul></article>");
    }
    html.push_str("</div></section></div>");
    html
}

fn blog_body(posts: &[Post], config: &SiteConfig) -> String {
    let mut html = String::from(
        r#"<div class="container page"><header class="header"><p class="kicker">ARTICLE INDEX</p><h1>文章归档</h1><p>支持全文检索和标签筛选。每次构建后会自动更新静态搜索索引。</p></header><section class="searchArea" id="phase-plan"><div id="search" class="search-fallback"><label class="sr-only" for="search-input">搜索文章</label><input id="search-input" type="search" placeholder="搜索文章标题、标签或正文内容" autocomplete="off"><p class="search-status" role="status">可搜索标题、标签和摘要</p><div class="search-fallback-results"></div></div></section><section class="grid">"#,
    );
    for (index, post) in posts.iter().enumerate() {
        html.push_str(&post_card(post, index, config));
    }
    html.push_str("</section></div>");
    html
}

fn comments_html(config: &GiscusConfig) -> String {
    if !config.enabled
        || config.repo.is_empty()
        || config.repo_id.is_empty()
        || config.category.is_empty()
        || config.category_id.is_empty()
    {
        return String::from(
            r#"<section class="comments-fallback"><h2>评论区预留</h2><p>当前还未配置 giscus。在 <code>config/site.toml</code> 中补齐仓库、分类与 ID 后，评论会自动生效。</p></section>"#,
        );
    }
    String::from(
        r#"<section class="wrapper comments-wrapper" aria-label="评论区"><h2 class="comments-title">评论</h2><p class="comments-hint comments-status">评论组件加载中…</p><div class="giscus-host"></div></section>"#,
    )
}

fn post_body(post: &Post, related: &[&Post], config: &SiteConfig) -> String {
    let mut title = String::new();
    if post.title_lines.is_empty() {
        title.push_str(&esc(&post.title));
    } else {
        for line in &post.title_lines {
            title.push_str(&format!(
                "<span class=\"titleLineManual\">{}</span>",
                esc(line)
            ));
        }
    }
    let mut tags = String::new();
    for tag in &post.tags {
        tags.push_str(&format!(
            "<span class=\"chip\" data-pagefind-filter=\"tag:{}\">{}</span>",
            esc(tag),
            esc(tag)
        ));
    }
    let cover = post.cover.as_ref().map_or_else(String::new, |cover| {
        format!(
            "<img class=\"heroImage\" src=\"{}\" alt=\"{}\" loading=\"eager\" decoding=\"async\">",
            esc(cover),
            esc(&post.title)
        )
    });
    let mut toc = String::new();
    let visible: Vec<_> = post.sections.iter().filter(|s| s.level <= 2).collect();
    if !visible.is_empty() {
        toc.push_str("<aside class=\"article-toc\"><div class=\"toc-rail\" aria-hidden=\"true\"><span class=\"toc-progress\"></span></div><ol class=\"toc-list\">");
        for (i, section) in visible.iter().enumerate() {
            toc.push_str(&format!("<li class=\"toc-item toc-level{}{}\"><button type=\"button\" data-section-id=\"{}\">{}</button></li>",section.level,if i==0{" toc-active"}else{""},section.id,esc(&section.title)));
        }
        toc.push_str("</ol></aside>");
    }
    let mut html = format!(
        r#"<div class="container page"><div class="readingLayout"><article class="article" data-pagefind-body><header class="header"><p class="meta"><span>{}</span><span>·</span><span>{} 分钟阅读</span></p><h1 data-pagefind-meta="title">{}</h1><p class="summary">{}</p><div class="tags">{}</div>{}</header><section class="content markdown-content" data-has-mermaid="{}">{}</section><div class="backArea"><a href="{}" class="button-secondary">返回文章列表</a></div>{}</article>{}</div>"#,
        post.published_zh(),
        post.reading_minutes,
        title,
        esc(&post.summary),
        tags,
        cover,
        post.has_mermaid,
        post.content_html,
        url(config, "/blog/"),
        comments_html(&config.giscus),
        toc
    );
    if !related.is_empty() {
        html.push_str("<aside class=\"related\"><h2>相关文章</h2><div class=\"relatedGrid\">");
        for (index, item) in related.iter().enumerate() {
            html.push_str(&post_card(item, index, config));
        }
        html.push_str("</div></aside>");
    }
    html.push_str("</div>");
    html
}

fn contact_body() -> String {
    String::from(
        r#"<div class="page"><section class="container hero"><h1 class="title">留言板</h1><p class="subtitle">有什么想说的，写在这里吧。所有留言会保存在浏览器本地。</p></section><section class="container formSection"><div class="floatDeco" aria-hidden="true"><span class="floatIcon">💬</span><span class="floatLabel">Say Hi</span></div><div class="successToast" role="status" hidden>✅ 留言提交成功！（已保存在本地浏览器中）</div><form class="form" id="contact-form" novalidate><div class="field"><label for="contact-name" class="label">昵称 <span class="required">*</span></label><input id="contact-name" name="name" type="text" class="input" placeholder="你的名字或昵称" autocomplete="name"><span class="errorText" data-error-for="name" role="alert" hidden></span></div><div class="field"><label for="contact-email" class="label">邮箱 <span class="required">*</span></label><input id="contact-email" name="email" type="email" class="input" placeholder="your@email.com" autocomplete="email"><span class="errorText" data-error-for="email" role="alert" hidden></span></div><div class="field"><label for="contact-content" class="label">留言内容 <span class="required">*</span></label><textarea id="contact-content" name="content" class="textarea" placeholder="写下你想说的话…" maxlength="500"></textarea><span class="charCount"><span data-char-count>0</span> / 500</span><span class="errorText" data-error-for="content" role="alert" hidden></span></div><button type="submit" class="submitBtn">提交留言</button></form></section><section class="container history" hidden><h2 class="historyTitle">历史留言 <span class="historyCount">（<span data-message-count>0</span> 条）</span></h2><div class="messageList"></div></section></div>"#,
    )
}

fn not_found_body(config: &SiteConfig) -> String {
    format!(
        r#"<div class="container page"><section class="hero"><p class="kicker">404 / NOT FOUND</p><h1>页面走丢了</h1><p>你访问的页面不存在，可能已移动或链接有误。</p><a class="button-primary" href="{}">返回首页</a></section></div>"#,
        url(config, "/")
    )
}

fn render_page(config: &SiteConfig, spec: &PageSpec<'_>) -> Result<String> {
    let mut stylesheet_names = vec![
        "global.css",
        "header.css",
        "footer.css",
        "post-card.css",
        spec.page_style,
    ];
    if spec.kind == "post" {
        stylesheet_names.extend(["article-progress.css", "comments.css"]);
    }
    let stylesheets = stylesheet_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .map(|name| url(config, &format!("/styles/{name}")))
        .collect::<Vec<_>>();
    let canonical = config.absolute_url(spec.path);
    let og_image = if spec.og_image.starts_with("http") {
        spec.og_image.clone()
    } else {
        config.absolute_url(&spec.og_image)
    };
    let home_url = url(config, "/");
    let blog_url = url(config, "/blog/");
    let contact_url = url(config, "/contact/");
    let rss_url = config.absolute_url("/rss.xml");
    let favicon_url = url(config, "/favicon.svg");
    let client_js = url(config, "/client/blog_client.js");
    let client_wasm = url(config, "/client/blog_client_bg.wasm");
    PageTemplate {
        language: &config.language,
        locale: &config.locale,
        title: spec.title,
        site_title: &config.site_title,
        description: spec.description,
        author: &config.author,
        keywords: &spec.keywords,
        canonical: &canonical,
        og_type: spec.og_type,
        og_image: &og_image,
        site_name: &config.site_name,
        base_path: &config.base_path,
        page_kind: spec.kind,
        home_url: &home_url,
        blog_url: &blog_url,
        contact_url: &contact_url,
        github_url: &config.github_url,
        rss_url: &rss_url,
        favicon_url: &favicon_url,
        client_js: &client_js,
        client_wasm: &client_wasm,
        stylesheets: &stylesheets,
        json_ld: &spec.json_ld,
        body: &spec.body,
        year: Utc::now().year(),
        bing: &config.verification.bing,
        baidu: &config.verification.baidu,
        so360: &config.verification.so360,
        giscus_enabled: config.giscus.enabled,
        giscus_repo: &config.giscus.repo,
        giscus_repo_id: &config.giscus.repo_id,
        giscus_category: &config.giscus.category,
        giscus_category_id: &config.giscus.category_id,
        giscus_mapping: &config.giscus.mapping,
        giscus_theme: &config.giscus.theme,
    }
    .render()
    .context("render Askama page")
}

fn build_site() -> Result<PathBuf> {
    let root = workspace_root()?;
    let config = SiteConfig::load(&root.join("config/site.toml"))?;
    let posts = load_posts(&root.join("content/posts"), &config)?;
    let dist = root.join("dist");
    if dist.exists() {
        fs::remove_dir_all(&dist).context("clear dist")?;
    }
    fs::create_dir_all(&dist)?;
    copy_tree(&root.join("public"), &dist)?;
    copy_tree(&root.join("assets"), &dist)?;
    let common_keywords = "技术博客,后端工程,Agent,Go,Rust,系统设计,内容系统,产品设计";
    let website_ld=json!({"@context":"https://schema.org","@type":"WebSite","name":config.site_name,"url":config.absolute_url("/"),"description":config.description,"inLanguage":config.locale,"potentialAction":{"@type":"SearchAction","target":{"@type":"EntryPoint","urlTemplate":format!("{}?q={{search_term_string}}",config.absolute_url("/blog/"))},"query-input":"required name=search_term_string"}}).to_string();
    let home_ld=json!({"@context":"https://schema.org","@type":"Blog","name":config.site_title,"description":config.description,"url":config.absolute_url("/"),"author":{"@type":"Person","name":config.author},"blogPost":featured_posts(&posts,3).iter().map(|p|json!({"@type":"BlogPosting","headline":p.title,"description":p.summary,"url":config.absolute_url(&format!("/blog/{}/",p.slug)),"datePublished":p.published_iso()})).collect::<Vec<_>>()}).to_string();
    let home = PageSpec {
        title: &config.site_title,
        description: &config.description,
        keywords: common_keywords.into(),
        path: "/",
        kind: "home",
        og_type: "website",
        og_image: config.og_image.clone(),
        page_style: "home.css",
        json_ld: vec![website_ld.clone(), home_ld],
        body: home_body(&posts, &config),
    };
    write_route(&dist, "/", &render_page(&config, &home)?)?;
    let collection=json!({"@context":"https://schema.org","@type":"CollectionPage","name":"文章归档","description":"浏览所有技术文章、构建记录和内容系统实践。","url":config.absolute_url("/blog/"),"mainEntity":{"@type":"ItemList","itemListElement":posts.iter().enumerate().map(|(i,p)|json!({"@type":"ListItem","position":i+1,"item":{"@type":"BlogPosting","headline":p.title,"description":p.summary,"url":config.absolute_url(&format!("/blog/{}/",p.slug)),"datePublished":p.published_iso()}})).collect::<Vec<_>>()}}).to_string();
    let blog = PageSpec {
        title: "文章归档 | Ca1_Tang",
        description: "浏览所有技术文章、构建记录和内容系统实践。",
        keywords: "技术文章,Agent,后端,Go,Rust,构建笔记,内容系统".into(),
        path: "/blog/",
        kind: "blog",
        og_type: "website",
        og_image: config.og_image.clone(),
        page_style: "blog.css",
        json_ld: vec![website_ld.clone(), collection],
        body: blog_body(&posts, &config),
    };
    write_route(&dist, "/blog/", &render_page(&config, &blog)?)?;
    for post in &posts {
        let related = related_posts(&posts, post, 3);
        let post_path = format!("/blog/{}/", post.slug);
        let article_ld=json!({"@context":"https://schema.org","@type":"Article","headline":post.title,"description":post.summary,"url":config.absolute_url(&post_path),"datePublished":post.published_iso(),"dateModified":post.published_iso(),"author":{"@type":"Person","name":config.author},"keywords":post.tags.join(", "),"image":post.cover.as_deref().unwrap_or(&config.og_image),"breadcrumb":{"@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":config.site_name,"item":config.absolute_url("/")},{"@type":"ListItem","position":2,"name":"文章归档","item":config.absolute_url("/blog/")},{"@type":"ListItem","position":3,"name":post.title}]}}).to_string();
        let page_title = format!("{} | {}", post.title, config.site_name);
        let page = PageSpec {
            title: &page_title,
            description: &post.summary,
            keywords: post.tags.join(","),
            path: &post_path,
            kind: "post",
            og_type: "article",
            og_image: post
                .cover
                .clone()
                .unwrap_or_else(|| config.og_image.clone()),
            page_style: "post.css",
            json_ld: vec![website_ld.clone(), article_ld],
            body: post_body(post, &related, &config),
        };
        write_route(&dist, &post_path, &render_page(&config, &page)?)?;
    }
    let contact = PageSpec {
        title: "留言板 | Ca1_Tang",
        description: "在浏览器本地保存的个人博客留言板。",
        keywords: "留言板,交流,技术博客".into(),
        path: "/contact/",
        kind: "contact",
        og_type: "website",
        og_image: config.og_image.clone(),
        page_style: "contact.css",
        json_ld: vec![website_ld.clone()],
        body: contact_body(),
    };
    write_route(&dist, "/contact/", &render_page(&config, &contact)?)?;
    let not_found = PageSpec {
        title: "页面不存在 | Ca1_Tang",
        description: "请求的页面不存在。",
        keywords: String::new(),
        path: "/404.html",
        kind: "404",
        og_type: "website",
        og_image: config.og_image.clone(),
        page_style: "blog.css",
        json_ld: vec![website_ld],
        body: not_found_body(&config),
    };
    write_route(&dist, "/404.html", &render_page(&config, &not_found)?)?;
    write_route(&dist, "/404/", &render_page(&config, &not_found)?)?;
    write_metadata(&dist, &config, &posts)?;
    build_wasm_client(&root, &dist)?;
    Ok(dist)
}

fn write_metadata(dist: &Path, config: &SiteConfig, posts: &[Post]) -> Result<()> {
    let mut rss = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>{}</title><link>{}</link><description>{}</description><language>{}</language>",
        esc(&config.site_title),
        config.absolute_url("/"),
        esc(&config.description),
        config.language
    );
    for p in posts {
        rss.push_str(&format!("<item><title>{}</title><link>{}</link><guid>{}</guid><pubDate>{}</pubDate><description>{}</description></item>",esc(&p.title),config.absolute_url(&format!("/blog/{}/",p.slug)),config.absolute_url(&format!("/blog/{}/",p.slug)),p.published_rfc2822(),esc(&p.summary)));
    }
    rss.push_str("</channel></rss>");
    write_route(dist, "/rss.xml", &rss)?;
    let mut sitemap = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">",
    );
    for path in ["/", "/blog/", "/contact/"] {
        sitemap.push_str(&format!(
            "<url><loc>{}</loc></url>",
            config.absolute_url(path)
        ));
    }
    for p in posts {
        sitemap.push_str(&format!(
            "<url><loc>{}</loc><lastmod>{}</lastmod></url>",
            config.absolute_url(&format!("/blog/{}/", p.slug)),
            p.published_iso()
        ));
    }
    sitemap.push_str("</urlset>");
    write_route(dist, "/sitemap.xml", &sitemap)?;
    write_route(
        dist,
        "/robots.txt",
        &format!(
            "User-agent: *\nAllow: /\nSitemap: {}\n",
            config.absolute_url("/sitemap.xml")
        ),
    )?;
    Ok(())
}

fn build_wasm_client(root: &Path, dist: &Path) -> Result<()> {
    let wasm = root.join("target/wasm32-unknown-unknown/release/blog_client.wasm");
    anyhow::ensure!(
        wasm.is_file(),
        "WASM binary missing; run cargo xtask build so the client is compiled first"
    );
    let output = dist.join("client");
    fs::create_dir_all(&output)?;
    let mut bindgen = wasm_bindgen_cli_support::Bindgen::new();
    bindgen
        .input_path(&wasm)
        .web(true)?
        .typescript(false)
        .generate(&output)
        .context("generate wasm-bindgen web bindings")?;
    Ok(())
}

fn local_target(dist: &Path, config: &SiteConfig, value: &str) -> Option<PathBuf> {
    let clean = value.split(['?', '#']).next().unwrap_or(value);
    if clean.is_empty()
        || clean.starts_with('#')
        || clean.starts_with("http://")
        || clean.starts_with("https://")
        || clean.starts_with("mailto:")
        || clean.starts_with("data:")
        || clean.starts_with("javascript:")
    {
        return None;
    }
    let relative = clean
        .strip_prefix(&config.base_path)
        .unwrap_or(clean)
        .trim_start_matches('/');
    let path = dist.join(relative);
    if relative.is_empty() || clean.ends_with('/') || path.extension().is_none() {
        Some(path.join("index.html"))
    } else {
        Some(path)
    }
}

fn check_site() -> Result<PathBuf> {
    let root = workspace_root()?;
    let dist = root.join("dist");
    anyhow::ensure!(
        dist.is_dir(),
        "dist is missing; run cargo xtask build first"
    );
    let required = [
        "index.html",
        "blog/index.html",
        "contact/index.html",
        "404.html",
        "rss.xml",
        "sitemap.xml",
        "robots.txt",
        "BingSiteAuth.xml",
        "styles/global.css",
        "client/blog_client.js",
        "client/blog_client_bg.wasm",
        "pagefind/pagefind-ui.js",
        "pagefind/pagefind-ui.css",
        "images/og-default.svg",
    ];
    for item in required {
        anyhow::ensure!(dist.join(item).is_file(), "missing generated {item}");
    }
    let config = SiteConfig::load(&root.join("config/site.toml"))?;
    let posts = load_posts(&root.join("content/posts"), &config)?;
    for post in &posts {
        anyhow::ensure!(
            dist.join("blog")
                .join(&post.slug)
                .join("index.html")
                .is_file(),
            "missing route for {}",
            post.slug
        );
    }
    let attribute = Regex::new(r#"(?:href|src)=\"([^\"]+)\""#)?;
    let mut html_count = 0;
    for entry in WalkDir::new(&dist)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("html"))
    {
        html_count += 1;
        let html = fs::read_to_string(entry.path())?;
        anyhow::ensure!(
            html.contains("<!doctype html>"),
            "invalid HTML {}",
            entry.path().display()
        );
        anyhow::ensure!(
            html.contains("<link rel=\"canonical\"")
                && html.contains("property=\"og:image\"")
                && html.contains("application/ld+json"),
            "SEO metadata missing in {}",
            entry.path().display()
        );
        anyhow::ensure!(
            !html.contains("readme-typing-svg") && !html.contains("Next.js"),
            "legacy runtime or external typing asset leaked into {}",
            entry.path().display()
        );
        anyhow::ensure!(
            html.contains("await init({ module_or_path:")
                && html.contains("document.documentElement.dataset.rustBlogReady"),
            "Rust/WASM bootstrap missing in {}",
            entry.path().display()
        );
        for capture in attribute.captures_iter(&html) {
            let value = &capture[1];
            if let Some(target) = local_target(&dist, &config, value) {
                anyhow::ensure!(
                    target.is_file(),
                    "broken local reference {value:?} in {} -> {}",
                    entry.path().display(),
                    target.display()
                );
            }
        }
    }
    anyhow::ensure!(
        html_count == posts.len() + 5,
        "unexpected HTML page count: {html_count}"
    );
    let rss = fs::read_to_string(dist.join("rss.xml"))?;
    anyhow::ensure!(
        rss.contains("+0000</pubDate>"),
        "RSS dates are not RFC 2822"
    );
    println!("checked {} routes and metadata", posts.len() + 4);
    Ok(dist)
}

fn serve(dist: &Path, port: u16) -> Result<()> {
    let address = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&address)
        .map_err(|e| anyhow::anyhow!("start preview server: {e}"))?;
    println!("serving {} at http://{address}", dist.display());
    for request in server.incoming_requests() {
        let request_path = request.url().split('?').next().unwrap_or("/");
        let base = SiteConfig::load(&workspace_root()?.join("config/site.toml"))?.base_path;
        let path = request_path
            .strip_prefix(&base)
            .unwrap_or(request_path)
            .trim_start_matches('/');
        let relative = if path.is_empty() { "index.html" } else { path };
        let candidate = dist.join(relative);
        let requested_file = if candidate.is_dir() {
            candidate.join("index.html")
        } else if candidate.is_file() {
            candidate
        } else if candidate.extension().is_none() {
            candidate.join("index.html")
        } else {
            candidate
        };
        let found = requested_file.is_file();
        let file = if found {
            requested_file
        } else {
            dist.join("404.html")
        };
        let status = if found { 200 } else { 404 };
        let mut bytes = Vec::new();
        fs::File::open(&file)?.read_to_end(&mut bytes)?;
        let mime = mime_guess::from_path(&file).first_or_octet_stream();
        let header = tiny_http::Header::from_bytes("Content-Type", mime.as_ref())
            .map_err(|_| anyhow::anyhow!("invalid content type"))?;
        request.respond(
            tiny_http::Response::from_data(bytes)
                .with_status_code(status)
                .with_header(header),
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build => {
            let dist = build_site()?;
            println!("built {}", dist.display());
        }
        Command::Check => {
            check_site()?;
        }
        Command::Serve { port } => {
            let dist = workspace_root()?.join("dist");
            anyhow::ensure!(
                dist.is_dir(),
                "dist is missing; run cargo xtask build first"
            );
            serve(&dist, port)?;
        }
    }
    Ok(())
}
