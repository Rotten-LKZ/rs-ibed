---
title: Systemd 部署
description: 使用挂载的配置文件和持久化存储，将 RS-IBED 作为 systemd 服务运行。
---

本指南介绍如何在 Linux 主机上通过 `systemd` 直接运行 `rs-ibed` 服务。

## 准备条件

- 一台使用 `systemd` 的 Linux 主机
- 已安装好的 `rs-ibed` 二进制，例如 `/opt/rs-ibed/rs-ibed`
- 一个 `config.toml` 配置文件
- 可写的上传、缓存目录，以及可选的 SQLite 数据目录

服务端二进制名称是 `rs-ibed`。如果配置文件不在当前工作目录，需要通过 `--config /path/to/config.toml` 显式指定。

## 必需环境变量

默认情况下，RS-IBED 通过 `IMG_` 前缀读取敏感环境变量：

- `IMG_AUTH_TOKEN`
- `IMG_JWT_SECRET`
- `IMG_DATABASE_URL`

如果你在 `config.toml` 中修改了 `server.env_prefix`，这里的变量名也要一起调整。

## 准备目录

先创建工作目录和可写的数据目录。仓库内示例配置使用的是 `./data/uploads` 与 `./data/cache`，因此它们会相对于服务的工作目录解析。

```bash
sudo mkdir -p /opt/rs-ibed/data/uploads
sudo mkdir -p /opt/rs-ibed/data/cache
sudo mkdir -p /opt/rs-ibed/data
```

如果你使用 SQLite，还要确保数据库文件的父目录可写。例如下面这个 URL 需要 `/opt/rs-ibed/data` 存在且可写：

```bash
export IMG_DATABASE_URL="sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc"
```

## 环境变量文件示例

通常建议把敏感信息放到环境变量文件，而不是直接写进 unit。

```ini
IMG_AUTH_TOKEN=replace-with-a-long-random-token
IMG_JWT_SECRET=replace-with-a-long-random-secret
IMG_DATABASE_URL=sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc
```

将它保存为 `/etc/rs-ibed/rs-ibed.env`，并收紧文件权限。

## Unit 文件示例

创建 `/etc/systemd/system/rs-ibed.service`：

```ini
[Unit]
Description=RS-IBED image hosting service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/opt/rs-ibed
ExecStart=/opt/rs-ibed/rs-ibed --config /opt/rs-ibed/config.toml
EnvironmentFile=/etc/rs-ibed/rs-ibed.env
# 也可以不用 EnvironmentFile，直接写成：
# Environment=IMG_AUTH_TOKEN=replace-me
# Environment=IMG_JWT_SECRET=replace-me
# Environment=IMG_DATABASE_URL=sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## 启动与排查

重新加载 unit，启用并启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now rs-ibed
```

查看当前状态：

```bash
sudo systemctl status rs-ibed
```

持续查看日志：

```bash
sudo journalctl -u rs-ibed -f
```

## SQLite 与 PostgreSQL 注意事项

### SQLite

- 将 `IMG_DATABASE_URL` 设置为 SQLite 路径，例如 `sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc`
- SQLite 数据库文件必须放在持久化存储上
- 上传目录和缓存目录也必须放在持久化存储上，否则重启或清理后图片文件与缓存会丢失

### PostgreSQL

- 将 `IMG_DATABASE_URL` 设置为 PostgreSQL 连接串，例如 `postgres://ibed:secret@db.example.com:5432/ibed`
- 应用启动时会先连接维护库，再尝试自动创建目标数据库
- 因此 PostgreSQL 用户通常需要有连接 `postgres` 库以及创建目标数据库的权限，除非目标数据库已经存在，或者你直接把数据库名设为 `postgres`

## 部署验证

服务运行后，在浏览器中打开 `http://your-host:3000/` 并上传一张测试图片。

然后重启服务，确认元数据、上传文件与缓存都仍然可用：

```bash
sudo systemctl restart rs-ibed
sudo systemctl status rs-ibed
```
