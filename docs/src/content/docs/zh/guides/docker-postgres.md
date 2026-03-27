---
title: Docker 部署（PostgreSQL）
description: 使用共享生产镜像与 compose 示例，将 RS-IBED 部署到 PostgreSQL。
---

本指南与 SQLite 版本共用同一个生产镜像，但通过 Compose 将应用与 PostgreSQL 组合部署。

## 为什么推荐 Compose

PostgreSQL 方案通常需要同时管理应用容器、数据库容器、持久化卷和环境变量。使用 `docker compose` 可以把这些内容放在同一个文件里统一管理。

## 必需环境变量

应用侧仍然需要以下变量：

- `IMG_AUTH_TOKEN`
- `IMG_JWT_SECRET`
- `IMG_DATABASE_URL`

PostgreSQL 容器还需要自己的初始化变量，例如：

- `POSTGRES_DB`
- `POSTGRES_USER`
- `POSTGRES_PASSWORD`

示例 compose 中，应用使用的是：

```text
postgres://ibed:ibed-password@postgres:5432/ibed
```

## Compose 文件

先下载 compose 文件。它会使用根目录 `Dockerfile` 构建应用镜像，启动 PostgreSQL，并为数据库和应用数据挂载持久化卷。

```bash
curl -L https://docsib.rotcool.me/docker-compose.pgsql.yml -o docker-compose.pgsql.yml
```

启动前，先把配置文件复制到仓库根目录的 `./deploy/config.toml`：

```bash
mkdir -p deploy
cp config.toml deploy/config.toml
```

## 启动服务栈

```bash
docker compose -f docker-compose.pgsql.yml up -d --build
```

## 就绪与权限注意事项

- 应用只有在 PostgreSQL 可连接后才能完成启动
- 示例 compose 为 PostgreSQL 配置了 healthcheck，并等待数据库容器进入 healthy 状态
- RS-IBED 启动时可能会先连接维护库，然后尝试自动创建目标数据库
- 因此，配置中的 PostgreSQL 用户通常需要具备连接 `postgres` 数据库以及创建目标数据库的权限，除非目标数据库已经存在，或者你刻意把目标数据库名设为 `postgres`

## 查看服务状态

查看服务列表：

```bash
docker compose -f docker-compose.pgsql.yml ps
```

查看应用日志：

```bash
docker compose -f docker-compose.pgsql.yml logs -f app
```

查看 PostgreSQL 日志：

```bash
docker compose -f docker-compose.pgsql.yml logs -f postgres
```

## 部署验证

打开 `http://localhost:3000/`，上传一张测试图片，然后重启整套服务，确认数据库元数据和图片文件都能持久保留。
