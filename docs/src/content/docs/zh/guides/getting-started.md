---
title: 开始使用
description: 从安装、配置到上传，快速部署并使用 RS-IBED。
---

本页帮助你快速把 RS-IBED 跑起来，并完成一次图片上传。

## 项目说明

RS-IBED 是一个使用 Rust 编写的图床服务，支持：

- 原图上传与访问
- 预设尺寸缩放
- 动态缩放与转码
- WebP / AVIF / JPEG / PNG 输出
- SQLite 或 PostgreSQL 存储元数据
- Bearer Token 与浏览器登录态鉴权
- OpenAPI 导出与前端 SDK 生成

## 安装方式

你可以通过以下两种方式使用：

### 方式一：下载已编译二进制

1. 前往 GitHub Releases 下载适合你系统的版本。
2. 准备 `config.toml` 配置文件。
3. 准备环境变量，例如：

```bash
export IMG_AUTH_TOKEN="your-upload-token"
export IMG_JWT_SECRET="your-jwt-secret"
export IMG_DATABASE_URL="sqlite://data.db"
```

4. 启动服务：

```bash
./rs-ibed
```

如果配置文件不在当前目录，可以显式指定：

```bash
./rs-ibed --config /path/to/config.toml
```

### 方式二：从源码运行

在仓库根目录执行：

```bash
cargo build
cargo run
```

## 最小可用配置

下面是一个适合本地试跑的最小配置示例：

```toml
[server]
host = "0.0.0.0"
port = 3000
log_level = "info"
env_prefix = "IMG"
url_pattern = "ymd"
cors_allow_origins = ["*"]
cors_max_age = 3600
enable_negotiated_cache = true
cache_max_age = 3600
trash_retention_days = 30

[database]
driver = "sqlite"
max_connections = 5
min_connections = 1

[storage]
base_dir = "./uploads"
cache_dir = "./cache"

[image]
enable = true
default_format = "webp"
quality = 75
max_workers = 2
allow_show_origin = true
cache_ttl = 25200
keep_metadata_fields = ["copyright", "settings", "time"]

[image.dynamic]
allow = false
allow_enlargement = false
max_width = 3840
max_height = 2160
```

更完整的参数说明见 [配置文件](/zh/guides/config/)。

## 必需环境变量

默认情况下，服务会读取 `IMG_` 前缀的环境变量：

| 变量名 | 说明 |
| :--- | :--- |
| `IMG_AUTH_TOKEN` | 上传、删除等受保护接口使用的 Bearer Token |
| `IMG_JWT_SECRET` | 用于浏览器登录态签发 JWT |
| `IMG_DATABASE_URL` | 数据库连接地址 |

如果你修改了 `server.env_prefix`，这里的前缀也要一起变化。

## 启动后可以访问什么

服务启动后，常见入口如下：

- `POST /api/upload`：上传图片
- `GET /v/...`：查看图片或处理后的图片
- `GET /d/...`：下载原图
- `GET /api/openapi.json`：获取 OpenAPI 文档
- `/login?token=...`：CLI 登录后用于浏览器建立登录态

## 上传图片

### 使用 HTTP API

上传接口为：

```text
POST /api/upload
```

鉴权方式支持：

- `Authorization: Bearer <AUTH_TOKEN>`
- 浏览器登录后的 Cookie

请求体为 `multipart/form-data`，其中：

- `file`：图片文件
- `keep_metadata_fields`：可选，逗号分隔的元数据保留字段，例如 `camera,time`

### 使用 curl 上传

```bash
curl -X POST "http://localhost:3000/api/upload" \
  -H "Authorization: Bearer $IMG_AUTH_TOKEN" \
  -F "file=@./photo.jpg"
```

如果希望覆盖服务端默认的 EXIF 保留规则：

```bash
curl -X POST "http://localhost:3000/api/upload" \
  -H "Authorization: Bearer $IMG_AUTH_TOKEN" \
  -F "file=@./photo.jpg" \
  -F "keep_metadata_fields=camera,time"
```

### 使用 upload CLI 上传

发行产物中会包含一个独立的 `upload` 二进制，适合批量上传或脚本调用。

运行方式：

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  ./photo.jpg
```

批量上传：

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  ./*.jpg
```

自定义保留的元数据字段：

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  --keep-metadata-fields camera,time \
  ./photo.jpg
```

也可以通过环境变量传入基础地址和 Token：

```bash
export UPLOAD_API_URL="http://localhost:3000"
export UPLOAD_AUTH_TOKEN="$IMG_AUTH_TOKEN"

./upload ./photo.jpg
```

成功时，CLI 只会输出上传后图片的完整链接，并且一行一个，适合配合 shell 管道使用：

```bash
./upload ./a.jpg ./b.jpg > links.txt
```

## 上传响应

上传成功后，服务会返回 JSON，其中包含图片访问地址：

```json
{
  "id": 1,
  "hash": "...",
  "url": "/v/2026/03/26/xxxxxxxx.webp",
  "file_name": "photo.jpg",
  "mime_type": "image/jpeg",
  "size": 123456,
  "width": 1920,
  "height": 1080
}
```

其中 `url` 字段就是图片访问路径。若你需要完整链接，需要再拼接服务基础地址，例如：

```text
http://localhost:3000/v/2026/03/26/xxxxxxxx.webp
```

## 开发相关工作流

如果你同时在开发前端或文档，常见流程如下：

1. 导出 OpenAPI：

```bash
cargo run -- export-openapi frontend/openapi.json
```

2. 生成前端 SDK（在 `frontend/` 目录）：

```bash
pnpm install
pnpm gen:api
```

3. 启动后端：

```bash
cargo run
```

4. 启动前端开发服务器：

```bash
cd frontend
pnpm dev
```

## 下一步

- 阅读 [配置文件](/zh/guides/config/) 深入了解配置项
- 阅读 [参与开发](/zh/guides/develop/) 了解本地开发流程
- 打开 `/api/openapi.json` 检查当前 API 定义
