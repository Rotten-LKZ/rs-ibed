---
title: Docker 部署（SQLite）
description: 使用仓库根目录中的共享生产镜像，将 RS-IBED 以单容器方式部署到 SQLite。
---

本指南使用仓库根目录的共享生产 `Dockerfile`，并将服务与 SQLite 放在同一个容器中运行。

## 构建镜像

在仓库根目录执行：

```bash
docker build -t rs-ibed:latest .
```

该镜像会先构建前端，再编译嵌入 `frontend/build` 资源的 Rust release 二进制，最后只把最终的 `rs-ibed` 二进制复制到运行时镜像中。

## 准备本地目录

为配置文件和持久化数据准备目录：

```bash
mkdir -p deploy data
cp config.toml deploy/config.toml
```

仓库中的示例配置会把文件写入 `./data/uploads` 和 `./data/cache`，所以容器里需要挂载 `/app/data`。

## 配置 SQLite 数据库 URL

SQLite 必须指向容器内一个可写且已挂载的路径：

```bash
export IMG_DATABASE_URL="sqlite:///app/data/image_host.db?mode=rwc"
```

同时还需要设置另外两个必需环境变量：

```bash
export IMG_AUTH_TOKEN="replace-with-a-long-random-token"
export IMG_JWT_SECRET="replace-with-a-long-random-secret"
```

## 运行容器

```bash
docker run -d \
  --name rs-ibed \
  -p 3000:3000 \
  -v "$(pwd)/deploy/config.toml:/app/config.toml:ro" \
  -v "$(pwd)/data:/app/data" \
  -e IMG_AUTH_TOKEN="$IMG_AUTH_TOKEN" \
  -e IMG_JWT_SECRET="$IMG_JWT_SECRET" \
  -e IMG_DATABASE_URL="$IMG_DATABASE_URL" \
  rs-ibed:latest \
  --config /app/config.toml
```

## 持久化注意事项

使用 SQLite 时，下列内容都必须位于挂载卷中：

- SQLite 数据库文件
- 上传后的原图
- 生成后的缓存文件

如果不挂载 `/app/data`，那么容器删除后数据库和图片文件都会丢失。

## 查看容器状态

查看日志：

```bash
docker logs -f rs-ibed
```

重启容器：

```bash
docker restart rs-ibed
```

## 部署验证

打开 `http://localhost:3000/`，上传一张测试图片，然后重启容器，确认图片元数据和文件仍然存在。
