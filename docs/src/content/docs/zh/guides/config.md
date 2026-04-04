---
title: 配置文件
description: 配置文件配置方法，包括分布式存储端点配置。
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
trash_retention_days = 30        # 自动永久删除回收站中超过此天数的图片，0 表示不自动清理
# strict_health_check = false    # 启用后，当图片元数据存在但在所有存储节点都找不到文件时，
                                 # 返回 503（存储不可用）而不是 404。有助于区分存储节点故障与真正缺失的文件。

[database]
driver = "sqlite"          # "sqlite" | "postgres"
max_connections = 5
min_connections = 1

# ------------------------------------------
# 存储端点（分布式存储）
# ------------------------------------------
# RS-IBED 使用一致性哈希环将文件分布在多个存储端点上。
# 这实现了水平扩展和高可用性。
#
# 主要特性：
#   - 一致性哈希：增删节点只影响相邻的哈希区间
#   - 容量加权：容量越大的端点获得越多的虚拟节点
#   - 优先级分层：priority 数值越小优先级越高，满了才会降级到下一层
#   - 多后端支持：支持本地文件系统和 S3 兼容存储
#   - 自动故障转移：读取时按哈希环顺序探测所有节点直到找到文件

[storage]
# max_payload_bytes = 52428800   # 可选：限制最大上传大小（字节），超过返回 413。

# 定义一个或多个存储端点。每个端点可以是 Local 或 S3 类型。
[[storage.endpoints]]
name           = "local-primary"
description    = "主本地文件系统"
type           = "Local"           # "Local" 或 "S3"
path           = "./data/storage"  # Local 类型必需
capacity_bytes = 10737418240       # 10 GB - 用于哈希环加权
priority       = 1                 # 数值越小优先级越高

[[storage.endpoints]]
name           = "minio-node1"
description    = "MinIO S3 节点 1"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true            # MinIO 和自建 S3 必需
direct_mode    = "presigned"        # "proxy" | "presigned" | "public"
direct_url_ttl = 3600              # 预签名 URL 有效期（秒）
capacity_bytes = 21474836480       # 20 GB
priority       = 1

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
# 若不允许，动态访问将会直接 403 Forbidden 除非匹配上预设
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
| `trash_retention_days` | u64 | `0` | 自动永久删除回收站中超过此天数的图片，设置为 0 表示不自动清理 |
| `strict_health_check` | bool | `false` | 启用后，当元数据存在但在所有存储节点都找不到文件时，返回 503 而不是 404 |

### [database] 数据库配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `driver` | Enum | `"sqlite"` | 数据库驱动：`sqlite` 或 `postgres` |
| `max_connections` | u32 | `5` | 数据库连接池最大连接数 |
| `min_connections` | u32 | `1` | 数据库连接池最小连接数 |

### [storage] 存储配置

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `max_payload_bytes` | u64 | - | 最大上传大小（字节），超过返回 HTTP 413 |

### [[storage.endpoints]] 存储端点配置

每个 `[[storage.endpoints]]` 块定义一个存储位置。路由器使用基于 `capacity_bytes` 加权的一致性哈希环来分布文件。

| 参数 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `name` | String | **必需** | 唯一标识符（用于环境变量凭证和管理 API）。所有端点间必须唯一。 |
| `description` | String | `""` | 人类可读的标签（可通过管理 API 编辑） |
| `type` | Enum | `"S3"` | 后端类型：`"Local"`（文件系统）或 `"S3"`（S3 兼容）。 |
| `path` | String | - | **Local 类型必需**：存储文件的目录路径。 |
| `bucket` | String | - | S3 桶名。也可通过环境变量设置：`IMG_ENDPOINT_{UPPER_NAME}__BUCKET` |
| `region` | String | `"us-east-1"` | S3 区域。也可通过环境变量设置。 |
| `force_path_style` | bool | `false` | 使用路径样式 S3 寻址（`http://host:port/bucket/key`）。MinIO 必需。 |
| `direct_mode` | Enum | `"proxy"` | 客户端如何访问图片：`"proxy"`（服务器代理）、`"presigned"`（预签名 URL）、`"public"`（直接公开 URL）。 |
| `direct_url_ttl` | u64 | - | 预签名 URL 有效期（秒）。仅在 `direct_mode = "presigned"` 时使用。 |
| `direct_url_public_endpoint` | String | - | 生成直接 URL 时覆盖 S3 端点 URL。反向代理设置中很有用。 |
| `capacity_bytes` | i64 | **必需** | 最大存储容量（字节）。用于环加权：容量越大 = 虚拟节点越多。 |
| `priority` | i32 | `1` | 路由优先级层级（数值越小优先级越高）。路由器先尝试所有 priority=1 的端点；如果都满了，降级到 priority=2。 |

#### S3 端点环境变量

对于 S3 端点，凭证从环境变量加载，使用以下模式：
- `IMG_ENDPOINT_{UPPER_NAME}__ENDPOINT_URL`（可选；AWS S3 可省略）
- `IMG_ENDPOINT_{UPPER_NAME}__ACCESS_KEY`（必需）
- `IMG_ENDPOINT_{UPPER_NAME}__SECRET_KEY`（必需）
- `IMG_ENDPOINT_{UPPER_NAME}__BUCKET`（可选；可使用配置字段）
- `IMG_ENDPOINT_{UPPER_NAME}__REGION`（可选；默认 us-east-1）
- `IMG_ENDPOINT_{UPPER_NAME}__FORCE_PATH_STYLE`（可选；默认 false）

名称格式：转为大写，连字符替换为下划线。示例：`name = "minio-node1"` → `IMG_ENDPOINT_MINIO_NODE1__ACCESS_KEY`。

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
| `allow` | bool | `false` | 是否允许动态参数处理。若关闭，未匹配的动态请求将返回 403 |
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

S3 存储端点的环境变量，参见上文 S3 端点环境变量部分。

## 存储配置示例

### 单本地端点（最小化配置）

```toml
[[storage.endpoints]]
name           = "local-primary"
description    = "主本地文件系统"
type           = "Local"
path           = "./data/storage"
capacity_bytes = 10737418240    # 10 GB
priority       = 1
```

### 双本地端点（分层优先级）

```toml
# 快速 NVMe 存储 - 最高优先级
[[storage.endpoints]]
name           = "nvme-fast"
description    = "NVMe 主存储"
type           = "Local"
path           = "/mnt/nvme/storage"
capacity_bytes = 107374182400   # 100 GB
priority       = 0              # 最高优先级

# HDD 备用 - NVMe 满了时使用
[[storage.endpoints]]
name           = "hdd-fallback"
description    = "HDD 备用"
type           = "Local"
path           = "/mnt/hdd/storage"
capacity_bytes = 1099511627776  # 1 TB
priority       = 1              # 较低优先级
```

### 本地主存储 + S3 故障转移

```toml
# 本地快速存储
[[storage.endpoints]]
name           = "local-origin"
description    = "本地源存储"
type           = "Local"
path           = "./data/storage"
capacity_bytes = 107374182400   # 100 GB
priority       = 0

# S3 云存储 - 本地满了时使用
[[storage.endpoints]]
name           = "cloud-s3"
description    = "S3 兼容云存储"
type           = "S3"
bucket         = "my-images"
region         = "us-east-1"
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 1099511627776  # 1 TB
priority       = 1
```

### 多节点 MinIO 集群

```toml
[[storage.endpoints]]
name           = "minio-node1"
description    = "MinIO 节点 1"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 21474836480    # 20 GB
priority       = 1

[[storage.endpoints]]
name           = "minio-node2"
description    = "MinIO 节点 2"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 53687091200    # 50 GB
priority       = 1

[[storage.endpoints]]
name           = "minio-node3"
description    = "MinIO 节点 3"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "public"
capacity_bytes = 107374182400   # 100 GB
priority       = 1
```

对应的环境变量：
```bash
export IMG_ENDPOINT_MINIO_NODE1__ENDPOINT_URL="http://minio1:9000"
export IMG_ENDPOINT_MINIO_NODE1__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE1__SECRET_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE1__BUCKET="ibed-images"

export IMG_ENDPOINT_MINIO_NODE2__ENDPOINT_URL="http://minio2:9000"
export IMG_ENDPOINT_MINIO_NODE2__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE2__SECRET_KEY="minioadmin"

export IMG_ENDPOINT_MINIO_NODE3__ENDPOINT_URL="http://minio3:9000"
export IMG_ENDPOINT_MINIO_NODE3__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE3__SECRET_KEY="minioadmin"
```
