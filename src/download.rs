use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use visper::Model;

const WEIGHT_REPOSITORY: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const REPORT_EVERY: Duration = Duration::from_millis(100);
/// ureq has no idle-read timeout, so a watchdog fails a download that stops
/// delivering bytes; otherwise it would sit in 下载中 with no 重试.
const STALL_AFTER: Duration = Duration::from_secs(60);

pub enum DownloadEvent {
    Progressed {
        model: Model,
        received: u64,
        total: u64,
    },
    Failed {
        model: Model,
        error: String,
    },
    Finished {
        model: Model,
    },
}

pub fn weight_path(support: &Path, model: Model) -> PathBuf {
    support.join(model.weight_file())
}

/// Downloads into a `.part` file and renames it only once every byte has
/// arrived, so a half-written weight never looks stored.
pub struct WeightDownloader {
    support: PathBuf,
    events: Sender<DownloadEvent>,
    /// Downloads started so far. One that is still running is not started
    /// again, as two would write the same `.part` file.
    attempts: Vec<Arc<Attempt>>,
}

impl WeightDownloader {
    pub fn new(support: PathBuf, events: Sender<DownloadEvent>) -> Self {
        Self {
            support,
            events,
            attempts: Vec::new(),
        }
    }
}

impl visper::Downloader for WeightDownloader {
    fn start(&mut self, model: Model) {
        self.attempts.retain(|attempt| !attempt.has_ended());
        if self.attempts.iter().any(|attempt| attempt.model == model) {
            return;
        }
        let attempt = Arc::new(Attempt::new(
            model,
            weight_path(&self.support, model),
            self.events.clone(),
        ));
        self.attempts.push(attempt.clone());
        let fetching = attempt.clone();
        std::thread::spawn(move || {
            if let Err(error) = fetching.fetch() {
                fetching.end(DownloadEvent::Failed { model, error });
            }
        });
        std::thread::spawn(move || {
            while !attempt.has_ended() {
                std::thread::sleep(Duration::from_secs(5));
                if attempt.idle() >= STALL_AFTER {
                    attempt.end(DownloadEvent::Failed {
                        model,
                        error: format!("超过 {} 秒没有收到数据。", STALL_AFTER.as_secs()),
                    });
                }
            }
        });
    }

    fn abandon(&mut self, model: Model) {
        for attempt in self
            .attempts
            .iter()
            .filter(|attempt| attempt.model == model)
        {
            attempt.abandon();
        }
        self.attempts.retain(|attempt| !attempt.has_ended());
    }
}

/// One run of the download. Whoever ends it first (the fetch, the stall
/// watchdog or an abandon) ends it for good; only the first two send the
/// terminal event, and an ended fetch stops writing.
struct Attempt {
    model: Model,
    target: PathBuf,
    events: Sender<DownloadEvent>,
    ended: AtomicBool,
    /// Held while the `.part` file is created, renamed or deleted, so that an
    /// abandon cannot fall between the check that the attempt still runs and
    /// the file operation that follows it.
    file: Mutex<()>,
    last_byte: Mutex<Instant>,
}

impl Attempt {
    fn new(model: Model, target: PathBuf, events: Sender<DownloadEvent>) -> Self {
        Self {
            model,
            target,
            events,
            ended: AtomicBool::new(false),
            file: Mutex::new(()),
            last_byte: Mutex::new(Instant::now()),
        }
    }

    fn end(&self, event: DownloadEvent) -> bool {
        let _file = self.file.lock();
        self.end_locked(event)
    }

    fn end_locked(&self, event: DownloadEvent) -> bool {
        if self.ended.swap(true, Ordering::SeqCst) {
            return false;
        }
        if let DownloadEvent::Failed { .. } = event {
            let _ = fs::remove_file(self.partial());
        }
        let _ = self.events.send(event);
        true
    }

    /// Stops the attempt without reporting anything and deletes its partial
    /// file. The fetch notices and never renames it into place.
    fn abandon(&self) {
        let _file = self.file.lock();
        if !self.ended.swap(true, Ordering::SeqCst) {
            let _ = fs::remove_file(self.partial());
        }
    }

    /// Creates the partial file, unless the attempt has already ended.
    fn create_partial(&self) -> Result<Option<File>, String> {
        let _file = self.file.lock();
        if self.has_ended() {
            return Ok(None);
        }
        File::create(self.partial())
            .map(Some)
            .map_err(|error| format!("写入模型失败：{error}"))
    }

    /// Moves the finished partial file into place and reports it, unless the
    /// attempt has ended meanwhile.
    fn complete(&self) -> Result<(), String> {
        let _file = self.file.lock();
        if self.has_ended() {
            return Ok(());
        }
        fs::rename(self.partial(), &self.target)
            .map_err(|error| format!("保存模型失败：{error}"))?;
        self.end_locked(DownloadEvent::Finished { model: self.model });
        Ok(())
    }

    fn has_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    fn idle(&self) -> Duration {
        self.last_byte
            .lock()
            .map(|at| at.elapsed())
            .unwrap_or_default()
    }

    fn touch(&self) {
        if let Ok(mut at) = self.last_byte.lock() {
            *at = Instant::now();
        }
    }

    fn partial(&self) -> PathBuf {
        self.target.with_extension("bin.part")
    }

    fn fetch(&self) -> Result<(), String> {
        if let Some(parent) = self.target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("无法创建模型目录：{error}"))?;
        }
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .build()
            .new_agent();
        let response = agent
            .get(format!("{WEIGHT_REPOSITORY}/{}", self.model.weight_file()))
            .call()
            .map_err(|error| format!("连接下载地址失败：{error}"))?;
        let expected = response.body().content_length();
        let mut reader = response.into_body().into_reader();

        let Some(file) = self.create_partial()? else {
            return Ok(());
        };
        let mut writer = BufWriter::new(file);
        let mut buffer = vec![0; 1 << 20];
        let mut received = 0;
        let mut reported = Instant::now();
        let report = |received: u64| {
            if self.has_ended() {
                return;
            }
            let total = expected.unwrap_or(self.model.bytes()).max(received);
            let _ = self.events.send(DownloadEvent::Progressed {
                model: self.model,
                received,
                total,
            });
        };
        report(0);
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| format!("下载中断：{error}"))?;
            if self.has_ended() {
                return Ok(());
            }
            if read == 0 {
                break;
            }
            self.touch();
            writer
                .write_all(&buffer[..read])
                .map_err(|error| format!("写入模型失败：{error}"))?;
            received += read as u64;
            if reported.elapsed() >= REPORT_EVERY {
                reported = Instant::now();
                report(received);
            }
        }
        let file = writer
            .into_inner()
            .map_err(|error| format!("写入模型失败：{}", error.error()))?;
        file.sync_all()
            .map_err(|error| format!("写入模型失败：{error}"))?;
        if let Some(expected) = expected
            && received != expected
        {
            return Err(format!("下载不完整：只收到 {received} / {expected} 字节。"));
        }
        report(received);
        self.complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{Receiver, channel};

    /// An attempt whose `.part` file already holds some bytes, in a fresh
    /// directory.
    fn attempt_with_partial(name: &str) -> (Attempt, Receiver<DownloadEvent>, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("visper-download-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let (sender, receiver) = channel();
        let attempt = Attempt::new(
            Model::LargeV3Turbo,
            weight_path(&dir, Model::LargeV3Turbo),
            sender,
        );
        fs::write(attempt.partial(), b"half a weight").unwrap();
        (attempt, receiver, dir)
    }

    #[test]
    fn abandoning_deletes_the_partial_file_and_it_is_never_renamed_into_place() {
        let (attempt, events, dir) = attempt_with_partial("abandon");

        attempt.abandon();
        // The fetch, which was already past its last read, now finishes.
        attempt.complete().unwrap();

        assert!(!attempt.partial().exists());
        assert!(!attempt.target.exists());
        assert!(events.try_recv().is_err());
        assert!(attempt.create_partial().unwrap().is_none());
        assert!(!attempt.partial().exists());
        assert!(!attempt.end(DownloadEvent::Failed {
            model: Model::LargeV3Turbo,
            error: "late".into()
        }));
        assert!(events.try_recv().is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_complete_download_is_renamed_into_place_and_reported_once() {
        let (attempt, events, dir) = attempt_with_partial("complete");

        attempt.complete().unwrap();

        assert!(!attempt.partial().exists());
        assert_eq!(fs::read(&attempt.target).unwrap(), b"half a weight");
        assert!(matches!(
            events.try_recv(),
            Ok(DownloadEvent::Finished {
                model: Model::LargeV3Turbo
            })
        ));
        attempt.abandon();
        assert!(attempt.target.exists());
        assert!(events.try_recv().is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_attempt_deletes_its_partial_file() {
        let (attempt, events, dir) = attempt_with_partial("failed");

        attempt.end(DownloadEvent::Failed {
            model: Model::LargeV3Turbo,
            error: "stalled".into(),
        });

        assert!(!attempt.partial().exists());
        assert!(matches!(
            events.try_recv(),
            Ok(DownloadEvent::Failed { .. })
        ));
        let _ = fs::remove_dir_all(dir);
    }
}
