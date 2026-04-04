use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::presigning::PresigningConfig;

use crate::error::{AppError, AppResult};

use super::{ObjectMeta, StorageBackend};

pub struct S3Backend {
    client: Client,
    bucket: String,
    /// Optional second client configured with a public-facing endpoint URL,
    /// used exclusively for presigned/public URL generation.
    presign_client: Option<Client>,
    /// The endpoint URL used for constructing public URLs.
    endpoint_url: Option<String>,
}

impl S3Backend {
    pub fn new(
        client: Client,
        bucket: String,
        presign_client: Option<Client>,
        endpoint_url: Option<String>,
    ) -> Self {
        Self {
            client,
            bucket,
            presign_client,
            endpoint_url,
        }
    }
}

#[async_trait]
impl StorageBackend for S3Backend {
    async fn put_object(&self, key: &str, data: Vec<u8>) -> AppResult<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn get_object(&self, key: &str) -> AppResult<Option<Vec<u8>>> {
        let resp = match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if e.as_service_error()
                    .map(|se| se.is_no_such_key())
                    .unwrap_or(false)
                {
                    return Ok(None);
                }
                return Err(AppError::Internal(e.to_string()));
            }
        };
        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .into_bytes()
            .to_vec();
        Ok(Some(bytes))
    }

    async fn head_object(&self, key: &str) -> AppResult<Option<ObjectMeta>> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(r) => Ok(Some(ObjectMeta {
                size: r.content_length().unwrap_or(0),
            })),
            Err(e)
                if e.as_service_error()
                    .map(|se| se.is_not_found())
                    .unwrap_or(false) =>
            {
                Ok(None)
            }
            Err(e) => Err(AppError::Internal(e.to_string())),
        }
    }

    async fn delete_object(&self, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn total_used_bytes(&self) -> AppResult<i64> {
        let mut total: i64 = 0;
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self.client.list_objects_v2().bucket(&self.bucket);
            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            for obj in resp.contents() {
                total += obj.size().unwrap_or(0);
            }

            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(total)
    }

    async fn presigned_get_url(
        &self,
        key: &str,
        expires_in: Duration,
    ) -> AppResult<Option<String>> {
        let client = self.presign_client.as_ref().unwrap_or(&self.client);
        let config = PresigningConfig::expires_in(expires_in)
            .map_err(|e| AppError::Internal(format!("presigning config error: {e}")))?;
        let url = client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| AppError::Internal(format!("presigned URL error: {e}")))?;
        Ok(Some(url.uri().to_string()))
    }

    fn public_url(&self, key: &str) -> Option<String> {
        self.endpoint_url.as_ref().map(|endpoint| {
            format!("{}/{}/{}", endpoint.trim_end_matches('/'), self.bucket, key)
        })
    }
}
