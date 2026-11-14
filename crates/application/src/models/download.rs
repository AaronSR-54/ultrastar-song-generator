use std::path::PathBuf;
use std::time::Duration;

use reqwest::header::RANGE;
use reqwest::StatusCode;
use tokio::io::AsyncWriteExt;
use usdx_ports::CancelFlag;

use crate::hash::hash_file;

use super::{ModelError, ModelSpec, ModelStore};

pub struct ModelDownloader {
    client: reqwest::Client,
    store: ModelStore,
}

impl ModelDownloader {
    pub fn new(store: ModelStore) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15 * 60))
            .build()
            .expect("failed to build HTTP client");
        Self { client, store }
    }

    pub fn store(&self) -> &ModelStore {
        &self.store
    }

    /// Downloads `spec` with resume support, verifies its SHA-256 and moves it
    /// into place. Progress receives `(downloaded_bytes, total_bytes)`.
    pub async fn download<F>(
        &self,
        spec: &ModelSpec,
        mut progress: F,
        cancel: Option<CancelFlag>,
    ) -> Result<PathBuf, ModelError>
    where
        F: FnMut(u64, u64),
    {
        let final_path = self.store.path_for(spec);
        if self.store.is_present(spec) {
            progress(spec.size_bytes, spec.size_bytes);
            return Ok(final_path);
        }

        self.store.ensure_root()?;
        let part_path = self.store.part_path(spec);
        let mut existing = std::fs::metadata(&part_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if existing > spec.size_bytes {
            let _ = std::fs::remove_file(&part_path);
            existing = 0;
        }
        self.store
            .check_space(spec.size_bytes.saturating_sub(existing))?;

        let mut response = self.request(spec, existing).await?;
        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE && existing > 0 {
            let _ = std::fs::remove_file(&part_path);
            existing = 0;
            response = self.request(spec, 0).await?;
        }
        if !response.status().is_success() {
            return Err(ModelError::HttpStatus(response.status().as_u16()));
        }

        let resumed = response.status() == StatusCode::PARTIAL_CONTENT;
        if !resumed {
            existing = 0;
        }
        let total = response
            .content_length()
            .map(|length| length + existing)
            .unwrap_or(spec.size_bytes);

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(resumed)
            .truncate(!resumed)
            .open(&part_path)
            .await?;

        let mut downloaded = existing;
        progress(downloaded, total);

        loop {
            if cancel.as_ref().is_some_and(CancelFlag::is_cancelled) {
                file.flush().await?;
                return Err(ModelError::Cancelled);
            }
            let Some(chunk) = response.chunk().await? else {
                break;
            };
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total);
        }
        file.flush().await?;
        drop(file);

        let actual = hash_file(&part_path)?;
        if actual != spec.sha256 {
            let _ = std::fs::remove_file(&part_path);
            return Err(ModelError::HashMismatch {
                expected: spec.sha256.clone(),
                actual,
            });
        }

        if final_path.exists() {
            let _ = std::fs::remove_file(&final_path);
        }
        std::fs::rename(&part_path, &final_path)?;
        progress(spec.size_bytes, spec.size_bytes);
        Ok(final_path)
    }

    async fn request(
        &self,
        spec: &ModelSpec,
        existing: u64,
    ) -> Result<reqwest::Response, ModelError> {
        let mut request = self.client.get(&spec.url);
        if existing > 0 {
            request = request.header(RANGE, format!("bytes={existing}-"));
        }
        Ok(request.send().await?)
    }

    pub fn path_for(&self, spec: &ModelSpec) -> PathBuf {
        self.store.path_for(spec)
    }

    pub fn is_present(&self, spec: &ModelSpec) -> bool {
        self.store.is_present(spec)
    }

    pub fn delete(&self, spec: &ModelSpec) -> std::io::Result<bool> {
        self.store.delete(spec)
    }
}

impl std::fmt::Debug for ModelDownloader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelDownloader")
            .field("store", &self.store.root())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModelPurpose;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    struct TestServer {
        url: String,
        requests: mpsc::Receiver<(Option<String>, usize)>,
        _handle: std::thread::JoinHandle<()>,
    }

    impl TestServer {
        fn start(body: Vec<u8>, responses: usize, cut_first_at: Option<usize>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/model.bin", listener.local_addr().unwrap());
            let (sender, requests) = mpsc::channel();
            let handle = std::thread::spawn(move || {
                for index in 0..responses {
                    let Ok((mut stream, _)) = listener.accept() else {
                        return;
                    };
                    let mut buffer = Vec::new();
                    let mut chunk = [0_u8; 1024];
                    loop {
                        let read = stream.read(&mut chunk).unwrap_or(0);
                        if read == 0 {
                            break;
                        }
                        buffer.extend_from_slice(&chunk[..read]);
                        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let request = String::from_utf8_lossy(&buffer).to_string();
                    let range = request.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("range")
                            .then(|| value.trim().to_string())
                    });
                    let start = range
                        .as_deref()
                        .and_then(|value| value.strip_prefix("bytes="))
                        .and_then(|value| value.split('-').next())
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0)
                        .min(body.len());
                    let remaining = &body[start..];
                    let status = if start > 0 {
                        "206 Partial Content"
                    } else {
                        "200 OK"
                    };
                    let header = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        remaining.len()
                    );
                    stream.write_all(header.as_bytes()).unwrap();
                    let send = if index == 0 {
                        cut_first_at.unwrap_or(remaining.len()).min(remaining.len())
                    } else {
                        remaining.len()
                    };
                    stream.write_all(&remaining[..send]).unwrap();
                    stream.flush().unwrap();
                    let _ = sender.send((range, send));
                }
            });
            Self {
                url,
                requests,
                _handle: handle,
            }
        }
    }

    fn body() -> Vec<u8> {
        (0..8192).map(|index| (index % 251) as u8).collect()
    }

    fn spec_for(url: &str, body: &[u8]) -> ModelSpec {
        ModelSpec {
            id: "test".into(),
            name: "Test model".into(),
            purpose: ModelPurpose::Separation,
            filename: "model.bin".into(),
            url: url.into(),
            size_bytes: body.len() as u64,
            sha256: crate::hash::hash_bytes(body),
            license: "MIT".into(),
            requirements: "CPU".into(),
        }
    }

    fn store() -> (tempfile::TempDir, ModelStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = ModelStore::new(directory.path());
        (directory, store)
    }

    #[tokio::test]
    async fn downloads_and_verifies_a_model() {
        let body = body();
        let server = TestServer::start(body.clone(), 1, None);
        let (_directory, store) = store();
        let downloader = ModelDownloader::new(store.clone());
        let spec = spec_for(&server.url, &body);

        let mut last = (0_u64, 0_u64);
        let path = downloader
            .download(&spec, |downloaded, total| last = (downloaded, total), None)
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert_eq!(last, (body.len() as u64, body.len() as u64));
        assert!(store.is_present(&spec));
        assert!(!store.part_path(&spec).exists());
        server.requests.recv().unwrap();
    }

    #[tokio::test]
    async fn resumes_after_a_cut_connection() {
        let body = body();
        let server = TestServer::start(body.clone(), 2, Some(1024));
        let (_directory, store) = store();
        let downloader = ModelDownloader::new(store.clone());
        let spec = spec_for(&server.url, &body);

        assert!(downloader.download(&spec, |_, _| {}, None).await.is_err());
        assert!(store.part_path(&spec).exists());
        assert_eq!(
            std::fs::metadata(store.part_path(&spec)).unwrap().len(),
            1024
        );

        let path = downloader.download(&spec, |_, _| {}, None).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), body);
        assert!(store.is_present(&spec));

        let (first_range, _) = server.requests.recv().unwrap();
        let (second_range, _) = server.requests.recv().unwrap();
        assert!(first_range.is_none());
        assert_eq!(second_range.as_deref(), Some("bytes=1024-"));
    }

    #[tokio::test]
    async fn bad_hash_removes_the_partial_file() {
        let body = body();
        let server = TestServer::start(body.clone(), 1, None);
        let (_directory, store) = store();
        let downloader = ModelDownloader::new(store.clone());
        let mut spec = spec_for(&server.url, &body);
        spec.sha256 = "0".repeat(64);

        let error = downloader
            .download(&spec, |_, _| {}, None)
            .await
            .unwrap_err();
        assert!(matches!(error, ModelError::HashMismatch { .. }));
        assert!(!store.path_for(&spec).exists());
        assert!(!store.part_path(&spec).exists());
        server.requests.recv().unwrap();
    }

    #[tokio::test]
    async fn offline_server_is_a_network_error() {
        let (_directory, store) = store();
        let downloader = ModelDownloader::new(store);
        let spec = spec_for("http://127.0.0.1:1/model.bin", b"x");
        let error = downloader
            .download(&spec, |_, _| {}, None)
            .await
            .unwrap_err();
        assert!(matches!(error, ModelError::Http(_)));
    }
}
