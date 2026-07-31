# 验收记录

> 验收日期：2026-07-31；分支：`codex/complete-rust-rewrite`。

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
6. 外部 typing SVG：替换为本地可访问、移动端不裁切的 HTML/CSS 动效。
