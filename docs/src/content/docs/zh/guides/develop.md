---
title: 参与开发
description: RS-IBED 后端、前端与文档站的本地开发说明。
---

本页介绍如何在本地参与 RS-IBED 的开发，包括后端、前端、OpenAPI 以及文档站。

## 仓库结构

该项目是一个 monorepo，主要目录如下：

```text
.
├── docs
├── frontend
├── migrations
├── src
├── Cargo.toml
├── config.toml
└── openapi.json
```

- `src/`：Rust 后端源码
- `frontend/`：SvelteKit 前端管理界面
- `docs/`：Astro Starlight 文档站
- `migrations/`：数据库迁移文件
- `openapi.json`：导出的 OpenAPI 描述

## 开发环境准备

建议先安装以下工具：

- Rust 与 Cargo
- `pnpm`
- SQLite 或 PostgreSQL（取决于你的数据库选择）

你还需要准备运行时环境变量：

```bash
export IMG_AUTH_TOKEN="dev-token"
export IMG_JWT_SECRET="dev-secret"
export IMG_DATABASE_URL="sqlite://data.db"
```

## 后端开发

### 常用命令

```bash
cargo build
cargo test
cargo run
```

如果需要指定配置文件：

```bash
cargo run -- --config ./config.toml
```

### 导出 OpenAPI

无需启动服务即可导出 OpenAPI：

```bash
cargo run -- export-openapi
cargo run -- export-openapi frontend/openapi.json
```

默认输出到仓库根目录的 `openapi.json`。

### 新增命令行工具

仓库的 `src/bin/` 下可以放独立二进制。例如：

- `src/bin/upload.rs`：用于通过 CLI 调用上传接口

发布或构建后会生成单独的 `upload` 可执行文件，可直接运行：

```bash
./upload --help
```

## 前端开发

前端位于 `frontend/`，使用 SvelteKit。

### 常用命令

在 `frontend/` 目录执行：

```bash
pnpm install
pnpm gen:api
pnpm check
pnpm build
pnpm dev
```

### 推荐联调顺序

1. 从后端导出 OpenAPI：

```bash
cargo run -- export-openapi frontend/openapi.json
```

2. 在前端生成 SDK：

```bash
cd frontend
pnpm gen:api
```

3. 启动后端：

```bash
cargo run
```

4. 启动前端：

```bash
cd frontend
pnpm dev
```

### 鉴权流程

当前项目包含一套 CLI 到浏览器的登录流程：

- `cargo run` 启动后，服务端会输出一个 CLI 登录 URL
- 该链接会指向 `/login?token=...`
- 浏览器打开后，前端会请求 `/api/auth/cli`
- 服务端随后写入 `ibed_token` Cookie，建立浏览器登录态

受保护接口也支持直接使用 Bearer Token，例如上传接口会校验：

```text
Authorization: Bearer <AUTH_TOKEN>
```

## 文档站开发

文档位于 `docs/`，使用 Astro Starlight。

### 常用命令

在 `docs/` 目录执行：

```bash
pnpm install
pnpm dev
pnpm build
pnpm preview
```

### 多语言文档位置

- `docs/src/content/docs/zh/`：中文文档
- `docs/src/content/docs/en/`：英文文档

如果你新增中文页面，通常也应该同步补上对应英文页面，保持导航结构一致。

### API 文档来源

文档站通过 `starlight-openapi` 读取仓库根目录的 `openapi.json`。

因此后端 API 发生变化后，推荐顺序是：

1. 重新导出 `openapi.json`
2. 如有需要同步到 `frontend/openapi.json`
3. 再启动或构建文档站与前端

## 调试建议

### 上传接口调试

你可以用以下方式调试上传：

- `curl`
- `cargo run --bin upload -- ...`
- 查看浏览器开发者工具里的网络请求

### 关注目录

开发上传、缩放或访问相关功能时，通常需要关注这些文件：

- `src/handlers/upload.rs`
- `src/handlers/view.rs`
- `src/router.rs`
- `src/config.rs`
- `src/auth.rs`

### 常见检查项

如果遇到接口调用失败，可以优先检查：

- `IMG_AUTH_TOKEN` 是否正确
- `IMG_DATABASE_URL` 是否可连接
- `config.toml` 中的 `url_pattern` 与图片访问路径是否一致
- `image.allow_show_origin` 是否影响原图访问
- OpenAPI 是否已经重新导出

## 提交前建议

提交改动前，建议至少执行：

```bash
cargo test
cargo build
```

如果修改了前端：

```bash
cd frontend
pnpm check
pnpm build
```

如果修改了文档：

```bash
cd docs
pnpm build
```

## 相关页面

- [开始使用](/zh/guides/getting-started/)
- [配置文件](/zh/guides/config/)
