//! The `aiEnabled` state and the embedding model it needs: whether the
//! scan's third pass runs, and the model file it runs, downloaded on the
//! first enable (`riffle_core::embed::MODEL_URL`, checked against
//! `MODEL_SIZE` and `MODEL_SHA256`) to the path the caller gives, each state
//! change handed to the caller's callback.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use riffle_core::embed::{MODEL_SHA256, MODEL_SIZE, MODEL_URL};
use sha2::{Digest, Sha256};

use crate::index::{lock, with_suffix};

/// Shortest interval between two reports of a running download.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// The `aiEnabled` switch and the model download, shared with the download
/// thread.
#[derive(Clone)]
pub struct Ai(Arc<Inner>);

struct Inner {
    status: Mutex<Status>,
    /// The model file; `None` when its folder is unknown, which leaves the
    /// third pass idle.
    model: Option<PathBuf>,
    on_state: Box<dyn Fn(AiState) + Send + Sync>,
}

#[derive(Default)]
struct Status {
    enabled: bool,
    download: Option<Download>,
    error: Option<String>,
}

struct Download {
    done: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
}

/// What `Ai` reports. `downloaded` is the bytes so far of a running
/// download, `None` when none runs.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AiState {
    pub enabled: bool,
    pub ready: bool,
    pub downloaded: Option<u64>,
    pub total: u64,
    pub error: Option<String>,
}

impl Ai {
    /// `on_state` receives the state after every switch, during a download
    /// at most every `PROGRESS_INTERVAL`, and when a download ends.
    pub fn new(
        enabled: bool,
        model: Option<PathBuf>,
        on_state: impl Fn(AiState) + Send + Sync + 'static,
    ) -> Self {
        // Part files a previous run left behind; no download runs yet.
        if let Some(path) = &model {
            let prefix = format!(
                "{}.part",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            if let Some(Ok(entries)) = path.parent().map(std::fs::read_dir) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().starts_with(&prefix) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
        Ai(Arc::new(Inner {
            status: Mutex::new(Status {
                enabled,
                ..Status::default()
            }),
            model,
            on_state: Box::new(on_state),
        }))
    }

    pub fn enabled(&self) -> bool {
        lock(&self.0.status).enabled
    }

    /// The model file the third pass embeds with: `None` while `aiEnabled`
    /// is off or the file is not in place yet.
    pub fn model_if_enabled(&self) -> Option<PathBuf> {
        if !self.enabled() {
            return None;
        }
        self.0.model.clone().filter(|path| path.is_file())
    }

    pub fn state(&self) -> AiState {
        let status = lock(&self.0.status);
        AiState {
            enabled: status.enabled,
            ready: self.0.model.as_deref().is_some_and(Path::is_file),
            downloaded: status
                .download
                .as_ref()
                .map(|d| d.done.load(Ordering::Relaxed)),
            total: MODEL_SIZE,
            error: status.error.clone(),
        }
    }

    /// Turn AI analysis on or off and report the new state. On, with the
    /// model not in place and no download running, starts the download; off
    /// cancels a running one (its partial file is removed). A failed
    /// download is kept as the state's `error` until the next switch.
    pub fn switch(&self, enabled: bool) -> AiState {
        {
            let mut status = lock(&self.0.status);
            status.enabled = enabled;
            status.error = None;
            let ready = self.0.model.as_deref().is_some_and(Path::is_file);
            if !enabled {
                if let Some(download) = status.download.take() {
                    download.cancel.store(true, Ordering::Relaxed);
                }
            } else if !ready && status.download.is_none() {
                match self.0.model.clone() {
                    Some(path) => status.download = Some(self.spawn_download(path)),
                    None => status.error = Some("no local data dir for the model".to_string()),
                }
            }
        }
        let state = self.state();
        (self.0.on_state)(state.clone());
        state
    }

    fn spawn_download(&self, path: PathBuf) -> Download {
        let done = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let download = Download {
            done: done.clone(),
            cancel: cancel.clone(),
        };
        let ai = self.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let mut last = Instant::now();
            let result = fetch(MODEL_URL).and_then(|mut body| {
                install(
                    &mut body,
                    &path,
                    MODEL_SHA256,
                    MODEL_SIZE,
                    &cancel,
                    |bytes| {
                        done.store(bytes, Ordering::Relaxed);
                        if last.elapsed() >= PROGRESS_INTERVAL {
                            last = Instant::now();
                            (ai.0.on_state)(ai.state());
                        }
                    },
                )
            });
            {
                let mut status = lock(&ai.0.status);
                // A switch off took the entry; a switch back on may have
                // started a new download since, which owns the entry now.
                if status
                    .download
                    .as_ref()
                    .is_some_and(|d| Arc::ptr_eq(&d.cancel, &cancel))
                {
                    status.download = None;
                    if let Err(e) = &result {
                        status.error = Some(format!("The model download failed: {e}"));
                    }
                }
            }
            match &result {
                Ok(()) => log::info!(
                    "embedding model downloaded to {} in {}ms",
                    path.display(),
                    started.elapsed().as_millis()
                ),
                Err(e) => log::error!("embedding model download failed: {e}"),
            }
            (ai.0.on_state)(ai.state());
        });
        download
    }
}

/// The body of a GET of `url`, failing on a non-success status. Reading it
/// times out after 30 s without data (reqwest's blocking default), not on
/// the whole transfer.
fn fetch(url: &str) -> Result<reqwest::blocking::Response, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        // Only fails when another thread installed one first.
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .and_then(|client| client.get(url).send())
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|e| e.to_string())
}

/// Copy `src` to `dest` through a `dest` + `.part-{n}` file, reporting the bytes copied
/// so far to `progress`, and rename it into place only when it is exactly
/// `size` bytes long with the lowercase hex SHA-256 `sha256`. Stops with an
/// `Err` once `cancel` is set or `src` runs past `size`; every `Err` removes
/// the partial file and leaves `dest` alone.
pub fn install(
    src: &mut dyn Read,
    dest: &Path,
    sha256: &str,
    size: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<(), String> {
    // One part file per call, so a canceled download still winding down
    // cannot truncate or delete the one a newer download is writing.
    static PART_SEQ: AtomicU64 = AtomicU64::new(0);
    let part = with_suffix(
        dest,
        &format!(".part-{}", PART_SEQ.fetch_add(1, Ordering::Relaxed)),
    );
    let result = (|| {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let mut file = File::create(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        let mut hasher = Sha256::new();
        let mut buf = vec![0u8; 1 << 20];
        let mut copied = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("canceled".to_string());
            }
            let n = match src.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
            };
            copied += n as u64;
            if copied > size {
                return Err(format!("more than the expected {size} bytes"));
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])
                .map_err(|e| format!("{}: {e}", part.display()))?;
            progress(copied);
        }
        if copied != size {
            return Err(format!("{copied} bytes, expected {size}"));
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if digest != sha256 {
            return Err(format!("SHA-256 {digest}, expected {sha256}"));
        }
        file.sync_all()
            .map_err(|e| format!("{}: {e}", part.display()))?;
        drop(file);
        std::fs::rename(&part, dest).map_err(|e| format!("{}: {e}", dest.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8] = b"not a model, just bytes";
    /// Not the SHA-256 of `DATA`.
    const WRONG_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("riffle-ai-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn part_files(dest: &Path) -> Vec<PathBuf> {
        let prefix = format!("{}.part", dest.file_name().unwrap().to_string_lossy());
        std::fs::read_dir(dest.parent().unwrap())
            .map(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(&prefix)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sha256_of(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn a_matching_download_lands_in_place_and_reports_its_progress() {
        let dir = temp_dir("ok");
        let dest = dir.join("models").join("m.onnx");
        let mut seen = Vec::new();
        install(
            &mut &DATA[..],
            &dest,
            &sha256_of(DATA),
            DATA.len() as u64,
            &AtomicBool::new(false),
            |n| seen.push(n),
        )
        .unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), DATA);
        assert!(part_files(&dest).is_empty());
        assert_eq!(seen.last(), Some(&(DATA.len() as u64)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wrong_hash_leaves_no_file() {
        let dir = temp_dir("hash");
        let dest = dir.join("m.onnx");
        let e = install(
            &mut &DATA[..],
            &dest,
            WRONG_SHA256,
            DATA.len() as u64,
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_err();
        assert!(e.contains("SHA-256"), "{e}");
        assert!(!dest.exists());
        assert!(part_files(&dest).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_short_or_long_download_leaves_no_file() {
        let dir = temp_dir("size");
        let dest = dir.join("m.onnx");
        let sha = sha256_of(DATA);
        let never = AtomicBool::new(false);
        let short = install(
            &mut &DATA[..],
            &dest,
            &sha,
            DATA.len() as u64 + 1,
            &never,
            |_| {},
        );
        assert!(short.unwrap_err().contains("expected"));
        let long = install(
            &mut &DATA[..],
            &dest,
            &sha,
            DATA.len() as u64 - 1,
            &never,
            |_| {},
        );
        assert!(long.unwrap_err().contains("more than"));
        assert!(!dest.exists());
        assert!(part_files(&dest).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_canceled_download_leaves_no_file() {
        let dir = temp_dir("cancel");
        let dest = dir.join("m.onnx");
        let e = install(
            &mut &DATA[..],
            &dest,
            &sha256_of(DATA),
            DATA.len() as u64,
            &AtomicBool::new(true),
            |_| {},
        )
        .unwrap_err();
        assert_eq!(e, "canceled");
        assert!(!dest.exists());
        assert!(part_files(&dest).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real download over the network: `MODEL_URL` through `fetch` and
    /// `install` into a temporary folder.
    #[test]
    #[ignore = "downloads 378 MB from huggingface.co"]
    fn the_pinned_model_downloads_and_matches_its_hash() {
        let dir = temp_dir("download");
        let dest = dir.join(riffle_core::embed::MODEL_FILE);
        let started = Instant::now();
        fetch(MODEL_URL)
            .and_then(|mut body| {
                install(
                    &mut body,
                    &dest,
                    MODEL_SHA256,
                    MODEL_SIZE,
                    &AtomicBool::new(false),
                    |_| {},
                )
            })
            .unwrap();
        eprintln!("downloaded in {:.1} s", started.elapsed().as_secs_f64());
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), MODEL_SIZE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_model_is_ready_only_once_its_file_is_in_place_and_ai_is_on() {
        let dir = temp_dir("ready");
        let path = dir.join("m.onnx");
        let ai = Ai::new(true, Some(path.clone()), |_| {});
        assert_eq!(ai.model_if_enabled(), None);
        assert!(!ai.state().ready);
        std::fs::write(&path, DATA).unwrap();
        assert_eq!(ai.model_if_enabled(), Some(path.clone()));
        assert!(ai.state().ready);
        let off = Ai::new(false, Some(path), |_| {});
        assert_eq!(off.model_if_enabled(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
