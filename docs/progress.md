# 实施进度

> 最后更新：2026-07-31

## 可观测入口

- 总体 Epic：[#1 使用 Rust 完整重写 myBlog](https://github.com/Cai-Tang-www/myBlog-rust/issues/1)
- Phase 0：[#2 冻结旧站基线与建立验收资产](https://github.com/Cai-Tang-www/myBlog-rust/issues/2)
- Phase 1：[#3 Cargo workspace、CI 与构建工具链](https://github.com/Cai-Tang-www/myBlog-rust/issues/3)
- Phase 2：[#4 内容管线、Markdown 与静态路由](https://github.com/Cai-Tang-www/myBlog-rust/issues/4)
- Phase 3：[#5 完整迁移视觉设计与响应式布局](https://github.com/Cai-Tang-www/myBlog-rust/issues/5)
- Phase 4：[#6 Rust/WASM 浏览器交互与第三方集成](https://github.com/Cai-Tang-www/myBlog-rust/issues/6)
- Phase 5：[#7 SEO、Pagefind 与 GitHub Pages 部署](https://github.com/Cai-Tang-www/myBlog-rust/issues/7)
- Phase 6：[#8 自我 Review、回归验收与合并发布](https://github.com/Cai-Tang-www/myBlog-rust/issues/8)

## 当前状态

| 阶段 | 状态 | 证据 |
|---|---|---|
| Phase 0 | 进行中 | `tests/fixtures/legacy-baseline.json`、SEO head 快照、6 张视觉截图 |
| Phase 1 | 进行中 | Cargo workspace、CI/Pages workflow、Rust 1.97.1、本地质量门 |
| Phase 2 | 未开始 | — |
| Phase 3 | 未开始 | — |
| Phase 4 | 未开始 | — |
| Phase 5 | 未开始 | — |
| Phase 6 | 未开始 | — |

## 本地基线证据

- 原站：`Cai-Tang-www/myBlog`，`course-web@d635a6d`；
- 页面基线：1280×720 首页/归档/留言/文章；
- 移动基线：390×844 首页/文章；
- 内容清单：7 篇 Markdown 的 frontmatter、字节数和 SHA-256；
- SEO 快照：首页、归档、留言和文章 `<head>`；
- 已验证旧站构建生成 15 个静态页面和 7 篇公开文章。

## 交付规则

1. 所有实现进入 `codex/complete-rust-rewrite`；
2. Draft PR 持续更新实现说明、测试证据和 review 记录；
3. 每个阶段完成后更新并关闭对应 Issue；
4. 本地质量门和 GitHub CI 全部通过后才允许合并；
5. 合并后继续验证 main CI 与 GitHub Pages 部署。
