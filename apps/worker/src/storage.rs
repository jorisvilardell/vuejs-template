use anyhow::{Context, Result};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::{BehaviorVersion, Builder, Region},
    presigning::PresigningConfig,
    primitives::ByteStream,
    Client,
};
use std::time::Duration;

#[derive(Clone)]
pub struct Storage {
    upload_client: Client,
    presign_client: Client,
    pub bucket: String,
    pub presign_ttl: Duration,
}

#[derive(Debug)]
pub struct Uploaded {
    pub key: String,
    pub url: String,
}

fn build_client(endpoint: &str, region: &str, access_key: &str, secret_key: &str) -> Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "env");
    let conf = Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .endpoint_url(endpoint)
        .credentials_provider(creds)
        .force_path_style(true)
        .build();
    Client::from_conf(conf)
}

impl Storage {
    pub async fn from_env() -> Result<Self> {
        let endpoint = std::env::var("S3_ENDPOINT")
            .context("S3_ENDPOINT not set")?;
        let public_endpoint = std::env::var("S3_PUBLIC_ENDPOINT")
            .unwrap_or_else(|_| endpoint.clone());
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "worlds".to_string());
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let access_key = std::env::var("S3_ACCESS_KEY_ID")
            .context("S3_ACCESS_KEY_ID not set")?;
        let secret_key = std::env::var("S3_SECRET_ACCESS_KEY")
            .context("S3_SECRET_ACCESS_KEY not set")?;
        let presign_ttl = Duration::from_secs(
            std::env::var("S3_PRESIGN_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
        );

        let upload_client = build_client(&endpoint, &region, &access_key, &secret_key);
        let presign_client = if public_endpoint == endpoint {
            upload_client.clone()
        } else {
            build_client(&public_endpoint, &region, &access_key, &secret_key)
        };

        let storage = Self {
            upload_client,
            presign_client,
            bucket,
            presign_ttl,
        };
        storage.ensure_bucket().await?;
        Ok(storage)
    }

    async fn ensure_bucket(&self) -> Result<()> {
        match self.upload_client.head_bucket().bucket(&self.bucket).send().await {
            Ok(_) => Ok(()),
            Err(_) => {
                self.upload_client
                    .create_bucket()
                    .bucket(&self.bucket)
                    .send()
                    .await
                    .context("create_bucket")?;
                Ok(())
            }
        }
    }

    pub async fn put(&self, key: &str, body: Vec<u8>, content_type: &str) -> Result<Uploaded> {
        self.upload_client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(body))
            .content_type(content_type)
            .send()
            .await
            .context("put_object")?;

        let presigned = self
            .presign_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(PresigningConfig::expires_in(self.presign_ttl)?)
            .await
            .context("presign get_object")?;

        Ok(Uploaded {
            key: key.to_string(),
            url: presigned.uri().to_string(),
        })
    }
}
