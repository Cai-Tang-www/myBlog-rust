# myBlog Rust

`Cai-Tang-www/myBlog` 的完整 Rust 重写：Rust 负责内容建模、Markdown 与静态生成，Rust/WASM 负责浏览器交互，Pagefind 负责中文全文索引。最终 `dist/` 可直接部署到 GitHub Pages，生产构建不依赖 Node.js。

## 已实现

- 首页、文章归档、7 篇文章、留言页、404；
- 原 Markdown/frontmatter 原样迁移及 SHA-256/字节级 fixture；
- Askama SSG、trailing slash、base path、正确 MIME 的预览服务器；
- RSS、sitemap、robots、Bing 验证、canonical、OG/Twitter、JSON-LD；
- 原视觉系统、桌面/移动响应式、typing、卡片与页面动效；
- Rust/WASM 粒子、鼠标排斥、目录、阅读进度、回顶、留言板；
- Mermaid SVG 渲染；giscus 可配置及未配置降级；
- Pagefind Extended v1.5.2 中文全文检索和 `?q=` 深链接；
- GitHub Actions CI 与 Pages 部署。

## 架构

- `crates/blog-core`：配置、内容、frontmatter、Markdown、标题目录、图片 URL、相关推荐、阅读时长；
- `crates/blog-generator`：Askama 页面、SEO/RSS/sitemap、静态资源、WASM glue、预览服务器与产物检查；
- `crates/blog-client`：渐进增强 Rust/WASM；正文和导航在 WASM 失败时仍可用；
- `xtask`：统一 `build/index/check/serve`；
- `content/posts`：唯一文章源；
- `assets` / `public`：CSS 与公开静态资源；
- `dist`：最终部署产物（不提交）。

详见 [`docs/architecture.md`](docs/architecture.md)、[`docs/acceptance.md`](docs/acceptance.md) 和 [`docs/rust-rewrite-plan.md`](docs/rust-rewrite-plan.md)。

## 工具链

仓库锁定 Rust `1.97.1`，包含 `rustfmt`、`clippy` 和 `wasm32-unknown-unknown`。Pagefind 使用官方 `pagefind_extended-v1.5.2`；CI 下载 tarball 与官方 `.sha256` 后校验，本地把可执行文件放在 `.tools/pagefind/`。

## 本地运行

```bash
cargo xtask build
cargo xtask check
cargo xtask serve --port 4173
```

访问 `http://127.0.0.1:4173/myBlog-rust/`。

完整质量门：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask build
cargo xtask check
```

## 配置

站点、域名、base path、图片 CDN、搜索引擎验证和 giscus 均在 `config/site.toml`。原仓库没有可用的 giscus repo/category IDs，因此默认关闭；补齐真实公开 ID 并设置 `enabled = true` 后，文章页自动加载 giscus。不要提交 token 或私密凭据。

## 部署

`.github/workflows/deploy-pages.yml` 在 `main` 上构建、检查并上传 `dist/`。仓库 Pages Source 已设为 **GitHub Actions**，`main` CI 与 Pages 部署已通过。生产地址：`https://cai-tang-www.github.io/myBlog-rust/`。
