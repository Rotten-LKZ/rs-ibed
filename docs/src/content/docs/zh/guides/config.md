---
title: 配置文件
description: 配置文件配置方法
---

配置文件放到项目的根目录下 `config.toml` 文件，或者通过 `./rs-ibed --config <path/to/config_file.toml>` 命令指定配置文件位置。

## 配置文件示例

```toml
[server]
host = "0.0.0.0"
port = 3000
log_level = "info"
env_prefix = "IMG"         # 环境变量前缀，如 IMG_AUTH_TOKEN, IMG_DATABASE_URL
# URL 路径模式，所有访问以 /v/ 为前缀
# "ymd"  → /v/2025/03/19/{hash}.{ext}
# "ym"   → /v/2025/03/{hash}.{ext}
# "y"    → /v/2025/{hash}.{ext}
# "flat" → /v/{hash}.{ext}
# preset 和 download 路径自动派生：
#   /v/thumb/2025/03/19/{hash}.{ext}
#   /v/preview/2025/03/19/{hash}.{ext}
# 路径中 v 改成 d，将会返回原图，是否需要认证取决于 image.allow_show_origin 是否为 true
url_pattern = "ymd"
cors_allow_origins = ["*"]       # 允许的跨域来源，* 代表允许所有
cors_max_age = 3600              # 预检请求的缓存时间（秒）
enable_negotiated_cache = true   # 是否启用协商缓存，实现 304 Not Modified 同时启用 etag 和 last_modified
cache_max_age = 3600             # 协商缓存的最大有效期（秒） 若设置为 0 则相当于 no-cache

[database]
driver = "sqlite"          # "sqlite" | "postgres"
max_connections = 5
min_connections = 1

[storage]
type = "local"             # "local"
base_dir = "./uploads"
cache_dir = "./cache"

[image]
enable = true              # 是否启用图片处理功能 若不开启则只会根据keep_metadata_fields保留元数据并存储原图，访问时直接返回原图 如果开启上传不会处理 exif 元数据，访问时根据预设尺寸进行处理
default_format = "webp"    # "webp" | "avif" | "original"
quality = 75               # 转码质量，1-100
max_workers = 2
allow_show_origin = true   # 是否允许访问原图（包含所有 exif 信息的）
cache_ttl = 25200          # 所有预设尺寸以及动态生成的默认缓存时间（秒），如果预设尺寸里单独设置了 cache_ttl 则以预设尺寸的为准，若设置为 0 则永久缓存
# "camera" | "settings" | "time" | "copyright" | "location" | "others"
keep_metadata_fields = ["copyright", "settings", "time"]

[image.dynamic]
# 是否允许动态参数，如果设置为不允许 max_* 配置项自动忽略
# 若不允许，使用动态参数访问的话 如果匹配上预设尺寸则正常返回 否则将会直接 403 Forbidden
allow = false
allow_enlargement = false  # 是否允许放大图片（仅当 allow = true 时生效）
max_width = 3840
max_height = 2160

# ------------------------------------------
# 预设尺寸 (Presets)
# ------------------------------------------
# 缩放规则：
#   - 只写 width 或 height  → 等比缩放，fit 不生效
#   - 同时写 width + height → 按 fit 策略缩放（省略 fit 默认 "contain"）
#   - 都不写                → 仅转码压缩，不改尺寸
#
# fit 取值：
#   "cover"   - 裁剪填充：缩放至完全覆盖目标尺寸，超出部分居中裁切
#   "contain" - 适应包含：缩放至完全放入目标尺寸，保持比例（默认值）
#   "fill"    - 拉伸填充：强制拉伸到目标尺寸，可能变形
#   "inside"  - 同 contain，但不会放大小于目标的图片
#   "outside" - 同 cover，但不会缩小大于目标的图片

# 特殊预设 "default" 是所有未匹配预设的默认值 若没有设置 default 则使用 [image] 全局默认值
[image.presets.default]
# 仅转码压缩，不改尺寸
# quality/format 省略 → 继承 [image] 全局默认值

[image.presets.thumb]
width = 200
height = 200
fit = "cover"
quality = 60               # 缩略图可以更激进地压缩
eager = true               # 上传时立即生成，如果此项为 true 则忽略 cache_ttl，生成后永久缓存

[image.presets.preview]
width = 1200
quality = 80               # 预览稍高质量
# 只给 width，自动等比缩放
# eager 省略 → false，首次访问时生成

[image.presets.legacy]
format = "jpeg"            # 覆盖全局 format，兼容不支持 webp 的老设备
quality = 70
```

## 配置参数详解

### [server] 服务器配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `host` | String | `"0.0.0.0"` | 监听地址 |
| `port` | u16 | `3000` | 监听端口 |
| `log_level` | Enum | `"info"` | 日志级别：`trace`, `debug`, `info`, `warn`, `error` |
| `env_prefix` | String | `"IMG"` | 环境变量前缀，用于加载敏感配置（如数据库 URL、Token 等） |
| `url_pattern` | Enum | `"ymd"` | URL 生成模式：`ymd` (年月日), `ym` (年月), `y` (年), `flat` (无层级) |
| `cors_allow_origins` | Array | `["*"]` | 允许的跨域来源列表 |
| `cors_max_age` | u64 | `3600` | CORS 预检请求缓存时间（秒） |
| `enable_negotiated_cache` | bool | `true` | 是否启用协商缓存（Etag / Last-Modified） |
| `cache_max_age` | u64 | `3600` | 浏览器强缓存时间（秒）。设置为 0 表示 `no-cache` |

### [database] 数据库配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `driver` | Enum | `"sqlite"` | 数据库驱动：`sqlite` 或 `postgres` |
| `max_connections` | u32 | `5` | 数据库连接池最大连接数 |
| `min_connections` | u32 | `1` | 数据库连接池最小连接数 |

### [storage] 存储配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `base_dir` | String | `"./uploads"` | 原始图片存储根目录 |
| `cache_dir` | String | `"./cache"` | 处理后的图片（缩略图等）缓存目录 |

### [image] 图片全局配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `enable` | bool | `true` | 是否启用图片处理功能。请注意：如果为 `false`，则上传的时候会根据 `keep_metadata_fields` 部分保留相关 EXIF，并且访问 `/v/[default]/[YYYY/MM/DD]/<hash>.<ext>` 时会原样返回保存了的数据；如果为 `true`，则上传的时候会保留所有信息，上传的文件将会原样保存到服务器，上传时不会处理 EXIF 相关信息 |
| `default_format` | Enum | `"webp"` | 默认输出格式：`webp`, `avif`, `jpeg`, `png`, `original` (保持原图格式) |
| `quality` | u8 | `75` | 默认转码质量 (1-100) |
| `max_workers` | u32 | `2` | 图片处理的最大并发工作线程数 |
| `allow_show_origin` | bool | `true` | 是否允许通过 `/d/` 路径访问原图（包含完整 EXIF 信息），如果为 `false` 则需要登录态才允许访问 |
| `cache_ttl` | u64 | `25200` | 缓存有效期（秒）。设置为 0 则永久缓存 |
| `keep_metadata_fields` | Array | `["copyright", "settings", "time"]` | 保留的元数据字段：`camera`, `settings`, `time`, `copyright`, `location`, `others` |

### [image.dynamic] 动态处理配置
允许通过 URL 参数（如 `/v/w_100,h_80/[YYYY/MM/DD]/<hash>.<ext>`）动态生成图片。

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `allow` | bool | `false` | 是否允许动态参数处理。若关闭，未匹配预设的动态请求将返回 403 |
| `allow_enlargement` | bool | `false` | 是否允许放大图片 |
| `max_width` | u32 | `3840` | 动态处理允许的最大宽度 |
| `max_height` | u32 | `2160` | 动态处理允许的最大高度 |

### [image.presets.NAME] 预设尺寸配置
可以通过 `[image.presets.thumb]` 等方式定义多个预设尺寸。其中 `default` 是特殊字段，其配置的是没有提供预设时访问到的照片配置（比如访问 `/v/[YYYY/MM/DD]/<hash>.<ext>`），如果访问不存在的 preset 仍然会返回 `404`。

对于 `width` 和 `height` 相关设定情况如下：
- 如果 `width` 和 `height` 都未定义，则会按照原尺寸进行转化；
- 如果只定义 `width` 或者 `height`，那么将会等比例缩放；
- 如果同时定义 `width` 和 `height`，会根据 `fit` 参数设定好缩放模式。

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `width` | u32 | - | 目标宽度 |
| `height` | u32 | - | 目标高度 |
| `fit` | Enum | `"contain"` | 缩放模式：`cover`, `contain`, `fill`, `inside`, `outside`（仅在同时设置宽高时生效） |
| `format` | Enum | 继承全局 | 该预设的特定格式 |
| `quality` | u8 | 继承全局 | 该预设的特定质量 |
| `eager` | bool | `false` | 是否在上传时立即生成该尺寸（若为 true 则永久缓存） |
| `cache_ttl` | u64 | 继承全局 | 该预设的缓存有效期（秒） |

### 环境变量 (Secrets)
敏感信息必须通过环境变量传递，前缀由 `server.env_prefix` 定义（默认为 `IMG`）。

| 变量名 | 说明 |
| :--- | :--- |
| `IMG_AUTH_TOKEN` | 管理操作（如上传、删除）所需的认证令牌 |
| `IMG_JWT_SECRET` | 用于生成和验证鉴权 JWT 的密钥 |
| `IMG_DATABASE_URL` | 数据库连接字符串（例如 SQLite：`sqlite://data.db`；Postgres：`postgres://user:pass@host/db`） |