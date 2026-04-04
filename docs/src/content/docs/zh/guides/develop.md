---
title: 参与开发
description: RS-IBED 后端、前端与文档站的本地开发说明。包括存储架构和分布式存储内部原理。
---

本页介绍如何在本地参与 RS-IBED 的开发，包括后端、前端、OpenAPI 以及文档站。同时还涵盖了分布式存储架构的技术细节，供希望理解或扩展存储层的开发者参考。

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

如果使用 S3 存储端点，还需要：

```bash
export IMG_ENDPOINT_MINIO1__ENDPOINT_URL="http://localhost:9000"
export IMG_ENDPOINT_MINIO1__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO1__SECRET_KEY="minioadmin"
export IMG_ENDPOINT_MINIO1__BUCKET="ibed-images"
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

## 存储架构（技术详解）

RS-IBED 使用**一致性哈希环**配合**虚拟节点**来实现分布式存储。本节为希望理解或修改存储层的开发者解释其工作原理。

### 核心组件

```text
src/
├── storage/
│   └── mod.rs           # StorageManager, StorageRouter, EndpointEntry
├── storage_backend/
│   ├── mod.rs           # StorageBackend trait
│   ├── local.rs         # LocalBackend 实现
│   └── s3.rs            # S3Backend 实现
└── models/
    └── storage_endpoint.rs  # StorageEndpointModel, DTOs
```

### 一致性哈希环

路由器构建的哈希环具有以下特性：

1. **每个端点**被映射到环上的多个**虚拟节点**
2. **虚拟节点数量**与 `capacity_bytes` 成正比（每 GB 100 个虚拟节点）
3. **文件放置**通过对内容哈希进行哈希计算，确定在环上的位置
4. **顺时针探测**找到下一个可用的端点用于写入
5. **多层优先级**允许容量溢出时降级到低优先级端点

```rust
// 来自 src/storage/mod.rs
// 虚拟节点数量计算
let node_count = (entry.capacity_bytes / 1_073_741_824).max(1) * 100;
```

### 写入流程

```
1. 哈希内容 → 获得 32 位位置
2. 从该位置开始，在当前优先级层顺时针遍历
3. 检查容量：used_size + file_size ≤ capacity_bytes
4. 如果满了，尝试下一个虚拟节点（同优先级）
5. 如果该优先级全满，降级到下一优先级层
6. 写入选中的端点
7. 更新内存和数据库中的 used_size
```

### 读取流程

```
1. 哈希内容 → 获得 32 位位置
2. 按顺时针顺序收集所有活跃端点（去重）
3. 对每个端点：
   - HEAD 检查缓存变体是否存在
   - 如果存在且 direct_mode ≠ Proxy → 返回重定向/预签名 URL
   - 否则 → 代理返回文件内容
4. 如果缓存未命中 → 从任意端点读取原图
5. 实时处理并可选地缓存结果
```

### 节点增删行为

| 场景 | 影响 | 数据位置 |
|------|------|----------|
| **添加节点** | 新的虚拟节点接管相邻的哈希区间 | 已有文件保留在原节点；新文件可能映射到新节点 |
| **移除节点** | 虚拟节点被移除；相邻节点接管该区间 | 已有文件保留在被移除的节点上；读取时会探测所有节点直到找到 |
| **禁用节点** | 状态改为 "disabled"；写入路由排除该端点的虚拟节点 | 文件保留；不再用于新写入 |

**关键特性**：只有哈希值落在相邻区间的文件会受影响。大多数文件继续映射到它们的原始节点。

### 文件存储布局

文件使用内容寻址的布局存储：

```
origin/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}
cache/{variant}/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}
```

示例：
```
origin/ab/cd/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890.jpg
cache/thumb/ab/cd/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890.webp
```

### 关键数据结构

```rust
// 路由器结构
pub struct StorageRouter {
    /// priority → ring (virtual_node_u32 → endpoint_name)
    rings: BTreeMap<i32, BTreeMap<u32, String>>,
    endpoints: HashMap<String, Arc<EndpointEntry>>,
}

// 端点条目
pub struct EndpointEntry {
    pub name: String,
    pub priority: i32,
    pub capacity_bytes: i64,
    pub status: String,
    pub used_size: Arc<AtomicI64>,
    pub backend: Arc<dyn StorageBackend>,
    pub direct_mode: DirectMode,
    // ...
}
```

### 管理 API 端点

存储端点可以通过管理 API 进行管理：

- `GET /api/admin/storage/endpoints` - 列出所有端点及实时状态
- `POST /api/admin/storage/endpoints/{name}/update` - 更新描述/状态

可变字段：`description`、`status`（`active`/`disabled`）
配置不可变字段：`priority`、`capacity_bytes`（需要重启才能更改）

### 存储后端 Trait

要添加新的存储后端，实现 `StorageBackend` trait：

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put_object(&self, key: &str, data: Vec<u8>) -> AppResult<()>;
    async fn get_object(&self, key: &str) -> AppResult<Option<Vec<u8>>>;
    async fn head_object(&self, key: &str) -> AppResult<Option<ObjectMeta>>;
    async fn delete_object(&self, key: &str) -> AppResult<()>;
    async fn total_used_bytes(&self) -> AppResult<i64>;
    async fn presigned_get_url(&self, key: &str, expires_in: Duration) -> AppResult<Option<String>>;
    fn public_url(&self, key: &str) -> Option<String>;
}
```

参考实现请查看 `src/storage_backend/local.rs` 和 `src/storage_backend/s3.rs`。

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

### 存储层调试

启用 debug 日志查看存储路由决策：

```bash
RUST_LOG=debug cargo run
```

关键日志信息：
- `reconciled used_size` - 端点使用量的定期对账
- `eager preset generated` - 后台缓存生成
- `storage router lock poisoned` - 路由器恐慌（不应发生）

### 关注目录

开发上传、缩放或访问相关功能时，通常需要关注这些文件：

- `src/handlers/upload.rs` - 上传处理器
- `src/handlers/view.rs` - 查看/下载处理器（含缓存逻辑）
- `src/storage/mod.rs` - StorageManager 和 StorageRouter
- `src/storage_backend/` - 存储后端实现
- `src/router.rs` - HTTP 路由
- `src/config.rs` - 配置结构
- `src/auth.rs` - 鉴权

开发与存储端点相关功能时：

- `src/storage/mod.rs` - 核心路由逻辑
- `src/handlers/admin.rs` - 端点管理的管理 API
- `src/models/storage_endpoint.rs` - 数据模型

### 常见检查项

如果遇到接口调用失败，可以优先检查：

- `IMG_AUTH_TOKEN` 是否正确
- `IMG_DATABASE_URL` 是否可连接
- `config.toml` 中的 `url_pattern` 与图片访问路径是否一致
- `image.allow_show_origin` 是否影响原图访问
- OpenAPI 是否已经重新导出
- S3 端点的凭证是否设置正确
- MinIO 的 `force_path_style` 是否设置正确

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
