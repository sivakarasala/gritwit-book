// Chapter 13 DSA Exercise: Strategy Pattern — Enum Dispatch vs Trait Objects
//
// GrindIt's StorageBackend uses enum dispatch (closed set of backends).
// This exercise compares enum dispatch with trait objects for polymorphism.

use std::fmt;

// ----------------------------------------------------------------
// Part 1: Enum dispatch — the GrindIt approach
// StorageBackend as enum with match-based method dispatch
// ----------------------------------------------------------------

#[derive(Debug)]
enum StorageBackend {
    Local {
        base_path: String,
    },
    S3 {
        bucket: String,
        region: String,
        public_url: String,
    },
    InMemory {
        files: Vec<(String, Vec<u8>)>,
    },
}

impl StorageBackend {
    fn upload(&mut self, key: &str, data: &[u8], content_type: &str) -> Result<String, String> {
        match self {
            StorageBackend::Local { base_path } => {
                // Simulate local file write
                let path = format!("{}/{}", base_path, key);
                println!(
                    "    [Local] Writing {} bytes to {} ({})",
                    data.len(),
                    path,
                    content_type
                );
                Ok(format!("/videos/{}", key))
            }
            StorageBackend::S3 {
                bucket,
                public_url,
                ..
            } => {
                // Simulate S3 upload
                println!(
                    "    [S3] Uploading {} bytes to s3://{}/{} ({})",
                    data.len(),
                    bucket,
                    key,
                    content_type
                );
                Ok(format!("{}/videos/{}", public_url, key))
            }
            StorageBackend::InMemory { files } => {
                // Store in memory (for tests)
                println!(
                    "    [InMemory] Storing {} bytes as '{}' ({})",
                    data.len(),
                    key,
                    content_type
                );
                files.push((key.to_string(), data.to_vec()));
                Ok(format!("mem://{}", key))
            }
        }
    }

    fn name(&self) -> &str {
        match self {
            StorageBackend::Local { .. } => "Local",
            StorageBackend::S3 { .. } => "S3",
            StorageBackend::InMemory { .. } => "InMemory",
        }
    }

    fn from_config(backend_type: &str) -> Result<Self, String> {
        match backend_type {
            "local" => Ok(StorageBackend::Local {
                base_path: "public/videos".to_string(),
            }),
            "s3" => Ok(StorageBackend::S3 {
                bucket: "grindit-videos".to_string(),
                region: "us-east-1".to_string(),
                public_url: "https://cdn.grindit.app".to_string(),
            }),
            "memory" => Ok(StorageBackend::InMemory { files: Vec::new() }),
            _ => Err(format!("Unknown backend: {}", backend_type)),
        }
    }
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StorageBackend::{}", self.name())
    }
}

// ----------------------------------------------------------------
// Part 2: Trait object dispatch — the extensible approach
// ----------------------------------------------------------------

trait Storage {
    fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String>;
    fn name(&self) -> &str;
}

struct LocalStorage {
    base_path: String,
}

impl Storage for LocalStorage {
    fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String> {
        println!(
            "    [Local/Trait] Writing {} bytes to {}/{} ({})",
            data.len(),
            self.base_path,
            key,
            content_type
        );
        Ok(format!("/videos/{}", key))
    }
    fn name(&self) -> &str {
        "Local"
    }
}

struct S3Storage {
    bucket: String,
    public_url: String,
}

impl Storage for S3Storage {
    fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String> {
        println!(
            "    [S3/Trait] Uploading {} bytes to s3://{}/{} ({})",
            data.len(),
            self.bucket,
            key,
            content_type
        );
        Ok(format!("{}/videos/{}", self.public_url, key))
    }
    fn name(&self) -> &str {
        "S3"
    }
}

/// Third-party can add this without modifying existing code
struct AzureBlobStorage {
    container: String,
    public_url: String,
}

impl Storage for AzureBlobStorage {
    fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String> {
        println!(
            "    [Azure/Trait] Uploading {} bytes to {}/{} ({})",
            data.len(),
            self.container,
            key,
            content_type
        );
        Ok(format!("{}/{}", self.public_url, key))
    }
    fn name(&self) -> &str {
        "Azure Blob"
    }
}

/// Factory returns a trait object — caller does not know the concrete type
fn create_storage(backend_type: &str) -> Box<dyn Storage> {
    match backend_type {
        "local" => Box::new(LocalStorage {
            base_path: "public/videos".to_string(),
        }),
        "s3" => Box::new(S3Storage {
            bucket: "grindit-videos".to_string(),
            public_url: "https://cdn.grindit.app".to_string(),
        }),
        "azure" => Box::new(AzureBlobStorage {
            container: "grindit-container".to_string(),
            public_url: "https://grindit.blob.core.windows.net".to_string(),
        }),
        _ => Box::new(LocalStorage {
            base_path: "public/videos".to_string(),
        }),
    }
}

// ----------------------------------------------------------------
// Part 3: Magic byte validation (pattern matching on bytes)
// ----------------------------------------------------------------

fn validate_video_magic_bytes(data: &[u8]) -> Result<&'static str, &'static str> {
    if data.len() < 12 {
        return Err("File too small");
    }

    // MP4: bytes 4-7 are "ftyp"
    if data[4..8] == *b"ftyp" {
        return Ok("MP4");
    }

    // WebM/MKV: EBML header
    if data[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return Ok("WebM/MKV");
    }

    // AVI: RIFF header with AVI subtype
    if data[0..4] == *b"RIFF" && data[8..12] == *b"AVI " {
        return Ok("AVI");
    }

    Err("Unknown or invalid video format")
}

// ----------------------------------------------------------------
// Part 4: Interview Problem — State Machine via Enum
// Model an upload pipeline with explicit state transitions.
// ----------------------------------------------------------------

#[derive(Debug)]
enum UploadState {
    Pending { file_name: String, size: usize },
    Validating { file_name: String, size: usize },
    Uploading { file_name: String, progress: f32 },
    Complete { url: String },
    Failed { error: String },
}

impl UploadState {
    fn next(self) -> Self {
        match self {
            UploadState::Pending { file_name, size } => {
                if size > 100 * 1024 * 1024 {
                    UploadState::Failed {
                        error: format!("{}: exceeds 100MB limit", file_name),
                    }
                } else {
                    UploadState::Validating { file_name, size }
                }
            }
            UploadState::Validating { file_name, size } => {
                if file_name.ends_with(".mp4") || file_name.ends_with(".webm") {
                    UploadState::Uploading {
                        file_name,
                        progress: 0.0,
                    }
                } else {
                    UploadState::Failed {
                        error: format!("{}: unsupported format", file_name),
                    }
                }
            }
            UploadState::Uploading {
                file_name,
                progress,
            } => {
                if progress >= 1.0 {
                    UploadState::Complete {
                        url: format!("/videos/{}", file_name),
                    }
                } else {
                    UploadState::Uploading {
                        file_name,
                        progress: (progress + 0.5).min(1.0),
                    }
                }
            }
            // Terminal states
            complete @ UploadState::Complete { .. } => complete,
            failed @ UploadState::Failed { .. } => failed,
        }
    }

    fn label(&self) -> &str {
        match self {
            UploadState::Pending { .. } => "PENDING",
            UploadState::Validating { .. } => "VALIDATING",
            UploadState::Uploading { .. } => "UPLOADING",
            UploadState::Complete { .. } => "COMPLETE",
            UploadState::Failed { .. } => "FAILED",
        }
    }
}

fn main() {
    println!("=== Strategy Pattern: Enum Dispatch vs Trait Objects ===\n");

    let sample_data = b"simulated video content for demonstration";

    // Part 1: Enum dispatch
    println!("--- Part 1: Enum Dispatch (GrindIt approach) ---");
    for backend_type in &["local", "s3", "memory"] {
        let mut backend = StorageBackend::from_config(backend_type).unwrap();
        println!("  Backend: {}", backend);
        let url = backend.upload("back-squat-demo.mp4", sample_data, "video/mp4");
        println!("    Result: {:?}\n", url);
    }

    // Part 2: Trait object dispatch
    println!("--- Part 2: Trait Object Dispatch (extensible approach) ---");
    for backend_type in &["local", "s3", "azure"] {
        let storage = create_storage(backend_type);
        println!("  Backend: {}", storage.name());
        let url = storage.upload("deadlift-demo.mp4", sample_data, "video/mp4");
        println!("    Result: {:?}\n", url);
    }

    // Part 3: Magic byte validation
    println!("--- Part 3: Magic Byte Validation ---");
    let test_files: Vec<(&str, Vec<u8>)> = vec![
        (
            "video.mp4",
            vec![0x00, 0x00, 0x00, 0x20, b'f', b't', b'y', b'p', 0x69, 0x73, 0x6F, 0x6D],
        ),
        (
            "video.webm",
            vec![0x1A, 0x45, 0xDF, 0xA3, 0x93, 0x42, 0x82, 0x88, 0x6D, 0x61, 0x74, 0x72],
        ),
        (
            "video.avi",
            vec![b'R', b'I', b'F', b'F', 0x00, 0x00, 0x00, 0x00, b'A', b'V', b'I', b' '],
        ),
        (
            "fake.mp4",
            vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01],
        ),
    ];

    for (name, bytes) in &test_files {
        match validate_video_magic_bytes(bytes) {
            Ok(format) => println!("  {} => Valid {} file", name, format),
            Err(e) => println!("  {} => REJECTED: {}", name, e),
        }
    }

    // Part 4: State machine
    println!("\n--- Part 4: Upload State Machine ---");

    let uploads = vec![
        UploadState::Pending {
            file_name: "squat.mp4".to_string(),
            size: 5_000_000,
        },
        UploadState::Pending {
            file_name: "huge.mp4".to_string(),
            size: 200_000_000,
        },
        UploadState::Pending {
            file_name: "photo.jpg".to_string(),
            size: 500_000,
        },
    ];

    for upload in uploads {
        println!("  Upload: {:?}", upload);
        let mut state = upload;
        loop {
            let prev_label = state.label().to_string();
            state = state.next();
            println!("    {} -> {}: {:?}", prev_label, state.label(), state);
            match &state {
                UploadState::Complete { .. } | UploadState::Failed { .. } => break,
                _ => {}
            }
        }
        println!();
    }

    // Comparison table
    println!("=== Enum Dispatch vs Trait Objects ===");
    println!("{:<15} {:<30} {:<30}", "Criterion", "Enum + match", "dyn Trait");
    println!("{}", "-".repeat(75));
    println!("{:<15} {:<30} {:<30}", "Dispatch", "Static (branch)", "Dynamic (vtable)");
    println!("{:<15} {:<30} {:<30}", "Extension", "Closed (add variant)", "Open (impl trait)");
    println!("{:<15} {:<30} {:<30}", "Exhaustiveness", "Compiler-checked", "Not checked");
    println!("{:<15} {:<30} {:<30}", "Overhead", "Zero", "1 pointer indirection");
    println!("{:<15} {:<30} {:<30}", "Size", "Largest variant", "Always 2 pointers");
    println!("{:<15} {:<30} {:<30}", "Best for", "2-3 known backends", "Plugin systems");
    println!();
    println!("GrindIt uses enum dispatch: 2 backends (Local, R2), fixed set.");
    println!("A plugin system would use trait objects: third parties add backends.");
}
