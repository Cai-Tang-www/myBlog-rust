# myBlog Rust

当前 TypeScript/Next.js 博客的 Rust 完整重写。

## 目标

- Rust 构建期静态站点生成；
- Rust/WASM 浏览器交互；
- 完整保留现有页面、视觉、响应式布局、动效、搜索、评论、留言和 SEO；
- 生产构建不依赖 Node.js；
- GitHub Pages 静态部署。

完整功能设计、迁移阶段和验收矩阵见 [`docs/rust-rewrite-plan.md`](docs/rust-rewrite-plan.md)。

## 当前状态

> 正在实施。GitHub Issue 和 Pull Request 是进度与决策的权威轨迹。

## 本地命令

```bash
cargo xtask build
cargo xtask check
cargo xtask serve --port 4173
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --package blog-client --target wasm32-unknown-unknown --release
```

## 架构

- `crates/blog-core`：内容模型、Markdown、SEO、相关推荐；
- `crates/blog-generator`：静态路由、Askama 模板、预览服务器；
- `crates/blog-client`：WASM 交互；
- `xtask`：构建、检查、搜索和发布编排；
- `content/posts`：Markdown 内容；
- `dist`：最终静态产物。
