use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::error::AppResult;

use super::{ObjectMeta, StorageBackend};

pub struct LocalBackend {
    base_path: PathBuf,
}

impl LocalBackend {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    fn full_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl StorageBackend for LocalBackend {
    async fn put_object(&self, key: &str, data: Vec<u8>) -> AppResult<()> {
        let path = self.full_path(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    async fn get_object(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let path = self.full_path(key);
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn head_object(&self, key: &str) -> AppResult<Option<ObjectMeta>> {
        let path = self.full_path(key);
        match tokio::fs::metadata(&path).await {
            Ok(meta) => Ok(Some(ObjectMeta {
                size: meta.len() as i64,
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn delete_object(&self, key: &str) -> AppResult<()> {
        let path = self.full_path(key);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn total_used_bytes(&self) -> AppResult<i64> {
        let mut total: i64 = 0;
        let mut stack = vec![self.base_path.clone()];

        while let Some(dir) = stack.pop() {
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            while let Some(entry) = entries.next_entry().await? {
                let ft = entry.file_type().await?;
                if ft.is_dir() {
                    stack.push(entry.path());
                } else if ft.is_file() {
                    total += entry.metadata().await?.len() as i64;
                }
            }
        }

        Ok(total)
    }
}
