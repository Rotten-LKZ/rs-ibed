---
title: Systemd Deployment
description: Run RS-IBED as a systemd service with a mounted config file and persistent storage.
---

This guide shows how to run the `rs-ibed` server directly on a Linux host with `systemd`.

## What you need

- A Linux host with `systemd`
- The `rs-ibed` binary installed somewhere stable such as `/opt/rs-ibed/rs-ibed`
- A `config.toml` file
- Writable storage directories for uploads, cache, and optionally SQLite data

The server binary is `rs-ibed`. If your config file is not in the working directory, start it with `--config /path/to/config.toml`.

## Required environment variables

By default, RS-IBED reads secrets from environment variables using the `IMG_` prefix:

- `IMG_AUTH_TOKEN`
- `IMG_JWT_SECRET`
- `IMG_DATABASE_URL`

If you changed `server.env_prefix` in `config.toml`, rename these variables to match.

## Prepare directories

Create a working directory that contains your config file and writable data paths. The checked-in sample config uses `./data/uploads` and `./data/cache`, so those paths resolve relative to the service working directory.

```bash
sudo mkdir -p /opt/rs-ibed/data/uploads
sudo mkdir -p /opt/rs-ibed/data/cache
sudo mkdir -p /opt/rs-ibed/data
```

If you use SQLite, make sure the parent directory of the database file is also writable. For example, this URL needs `/opt/rs-ibed/data` to exist and be writable:

```bash
export IMG_DATABASE_URL="sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc"
```

## Example environment file

Storing secrets in an environment file is usually cleaner than embedding them directly in the unit.

```ini
IMG_AUTH_TOKEN=replace-with-a-long-random-token
IMG_JWT_SECRET=replace-with-a-long-random-secret
IMG_DATABASE_URL=sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc
```

Save that as `/etc/rs-ibed/rs-ibed.env` and restrict its permissions.

## Example unit file

Create `/etc/systemd/system/rs-ibed.service`:

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
# You can also use inline values instead of EnvironmentFile:
# Environment=IMG_AUTH_TOKEN=replace-me
# Environment=IMG_JWT_SECRET=replace-me
# Environment=IMG_DATABASE_URL=sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Start and inspect the service

Reload the unit files, enable the service, and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now rs-ibed
```

Check current status:

```bash
sudo systemctl status rs-ibed
```

Follow logs:

```bash
sudo journalctl -u rs-ibed -f
```

## Notes for SQLite and PostgreSQL

### SQLite

- Set `IMG_DATABASE_URL` to a SQLite path such as `sqlite:///opt/rs-ibed/data/image_host.db?mode=rwc`
- Keep the SQLite database file on persistent storage
- Keep uploads and cache on persistent storage too, otherwise image files and generated variants will disappear after cleanup or restart

### PostgreSQL

- Set `IMG_DATABASE_URL` to a PostgreSQL URL such as `postgres://ibed:secret@db.example.com:5432/ibed`
- On startup, the app connects to the maintenance database first and may create the target database automatically
- The PostgreSQL user therefore needs permission to connect to `postgres` and create the target database, unless the target database already exists or you use `postgres` itself as the database name

## Verify the deployment

After the service is running, open `http://your-host:3000/` in a browser and upload a test image.

Then restart the service and confirm metadata, uploads, and cached variants still work:

```bash
sudo systemctl restart rs-ibed
sudo systemctl status rs-ibed
```
