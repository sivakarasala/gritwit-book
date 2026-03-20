// Chapter 13: Video Uploads
// Spotlight: Smart Pointers (Arc) & Enum-Based Abstraction
//
// StorageBackend — deep module: simple upload() interface, complex internals.

use std::sync::Arc;

pub enum StorageBackend {
    Local,
    R2 {
        bucket: Box<s3::Bucket>,
        public_url: String,
    },
}

impl StorageBackend {
    pub fn from_config(config: &StorageSettings) -> Self {
        match config.backend.as_str() {
            "r2" => {
                let credentials = s3::creds::Credentials::new(
                    Some(&config.access_key_id),
                    Some(&config.secret_access_key),
                    None, None, None,
                ).expect("Invalid R2 credentials");

                let region = s3::Region::Custom {
                    region: "auto".to_string(),
                    endpoint: config.endpoint.clone(),
                };

                let bucket = s3::Bucket::new(&config.bucket_name, region, credentials)
                    .expect("Invalid bucket config")
                    .with_path_style();

                StorageBackend::R2 {
                    bucket: Box::new(bucket),
                    public_url: config.public_url.clone(),
                }
            }
            _ => StorageBackend::Local,
        }
    }

    pub async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<String, String> {
        match self {
            StorageBackend::Local => {
                let path = format!("public/videos/{}", key);
                tokio::fs::create_dir_all("public/videos")
                    .await
                    .map_err(|e| e.to_string())?;
                tokio::fs::write(&path, data)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!("/videos/{}", key))
            }
            StorageBackend::R2 { bucket, public_url } => {
                bucket
                    .put_object_with_content_type(key, data, content_type)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!("{}/{}", public_url, key))
            }
        }
    }
}

/// Validate video file by checking magic bytes
pub fn validate_magic_bytes(data: &[u8]) -> Result<&'static str, String> {
    if data.len() < 12 {
        return Err("File too small to validate".into());
    }

    // MP4: bytes 4-7 are "ftyp"
    if &data[4..8] == b"ftyp" {
        return Ok("video/mp4");
    }

    // WebM: starts with EBML header 0x1A45DFA3
    if &data[0..4] == &[0x1A, 0x45, 0xDF, 0xA3] {
        return Ok("video/webm");
    }

    // AVI: starts with "RIFF" and contains "AVI "
    if &data[0..4] == b"RIFF" && &data[8..12] == b"AVI " {
        return Ok("video/avi");
    }

    Err("Unsupported video format. Accepted: MP4, WebM, AVI".into())
}
