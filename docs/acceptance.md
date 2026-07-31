# 验收记录

> 验收日期：2026-07-31；实现分支：`codex/complete-rust-rewrite`；生产提交：`67d8c4a`。

## 自动质量门

- `cargo fmt --all --check`；
- `cargo clippy --workspace --all-targets -- -D warnings`；
- `cargo test --workspace`：10 项通过；
- `cargo xtask build`：12 个 HTML，Pagefind Extended v1.5.2 索引 7 页、2357 词、1 个 filter；
- `cargo xtask check`：11 条必需路由与元数据通过。

Windows GNU linker 的 `corrupt .drectve` 为本机工具链 warning；命令退出码为 0，Linux GitHub Actions 不出现该问题。

## 浏览器功能验收

- WASM bootstrap：`data-rust-blog-ready=true`，Canvas 由默认 300×150 正确调整为 1280×720；
- 首页：本地 HTML/CSS typing，无远端 typing SVG，无横向溢出；阅读百分比与回顶可操作；
- 搜索：Pagefind 中文正文命中；`?q=人不能退出` 自动恢复并返回 3 篇结果；静态卡片在查询时隐藏；
- 留言：空字段、错误邮箱、字数、成功提交、toast、刷新后 1 条历史记录均验证；内容按文本节点渲染；
- 文章：15 项目录、点击写入 `#section-1`、active/progress、5 个 Mermaid SVG、相关文章、giscus 未配置提示；
- 404：未知无扩展名路径返回 HTTP 404，不再返回 500；
- MIME：CSS `text/css`、JS `text/javascript`、WASM `application/wasm`、Pagefind JS `text/javascript`；
- 390×844 iframe：首页 Canvas 390×844、typing 不裁切、无横向溢出；文章无横向溢出、5 个 Mermaid SVG、目录 `display:none`，不遮挡正文。

## 视觉证据

`tests/visual/baseline/` 保存旧站 6 张基线；`tests/visual/rust/` 保存 Rust 桌面首页、归档、留言、文章截图。移动端以真实 390×844 iframe 的 DOM/布局指标验收（in-app 浏览器截图不合成跨进程 iframe 像素）。

## 自审发现并修复

1. 未知路由曾返回 500：改为存在性检查并回退 404 页面；
2. WASM glue 曾只加载未初始化：显式 `await init({ module_or_path })`，并加入产物检查；
3. `?q=` 在 Pagefind 接管 DOM 后丢失：使用 `triggerSearch` 恢复；
4. TOC Rust 选择器与命名空间类不一致：统一为 `.toc-item/.toc-active/.toc-progress`；
5. 文章页漏挂目录和评论 CSS：生成器按 post 页面追加两张组件样式；
6. 外部 typing SVG：替换为本地可访问、移动端不裁切的 HTML/CSS 动效；
7. PR CI 首轮发现 Windows CRLF fixture 与 Linux LF 不一致：改为按原仓库 Git canonical LF 建立 SHA-256/字节基线，并在测试中规范化 checkout-specific CRLF。

## GitHub 与生产验收

- PR [#9](https://github.com/Cai-Tang-www/myBlog-rust/pull/9) 在最新 Ubuntu `quality` CI 全绿后转 Ready，经最终 diff/敏感信息/产物边界复核无阻断，squash 合并为 `67d8c4a`；
- `main` CI [30657561310](https://github.com/Cai-Tang-www/myBlog-rust/actions/runs/30657561310) 成功：format、clippy、test、完整静态构建、产物检查与 artifact 上传均通过；
- Pages workflow [30657561315](https://github.com/Cai-Tang-www/myBlog-rust/actions/runs/30657561315) 的 build 与 deploy 均成功，Pages Source 为 GitHub Actions，HTTPS 开启；
- 生产首页可访问，WASM `data-rust-blog-ready=true`，Canvas 1280×720，canonical、OG、JSON-LD 与 `/myBlog-rust` 子路径正确，无横向溢出；
- 生产归档 `?q=人不能退出` 恢复查询并显示 3 个 Pagefind 结果；
- 生产文章显示 15 项目录、1 个 active 项、5 个 Mermaid SVG、3 篇相关文章和 giscus 未配置提示；目录点击更新 `#section-1` 并正确滚动；
- 生产留言页显示 3 项必填校验；有效留言提交后出现 1 条历史记录，刷新后仍存在；
- 生产未知路由显示自定义 404 页面；关键 CSS、Rust JS/WASM 与头像资源均从生产子路径成功加载；浏览器无站点 error/warning。
