use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const WEIGHT_FILE: &str = "ggml-large-v3.bin";
const WEIGHT_REPOSITORY: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
/// Approximate size of the weight, shown only when the server sends no length.
const WEIGHT_BYTES: u64 = 3_095_033_483;
const REPORT_EVERY: Duration = Duration::from_millis(100);
/// ureq has no idle-read timeout, so a watchdog fails a download that stops
/// delivering bytes; otherwise it would sit in 下载中 with no 重试.
const STALL_AFTER: Duration = Duration::from_secs(60);

pub enum DownloadEvent {
    Progressed { received: u64, total: u64 },
    Failed(String),
    Finished,
}

pub fn weight_path(support: &Path) -> PathBuf {
    support.join(WEIGHT_FILE)
}

/// Downloads into a `.part` file and renames it only once every byte has
/// arrived, so a half-written weight never looks stored.
pub struct WeightDownloader {
    target: PathBuf,
    events: Sender<DownloadEvent>,
}

impl WeightDownloader {
    pub fn new(target: PathBuf, events: Sender<DownloadEvent>) -> Self {
        Self { target, events }
    }
}

impl visper::Downloader for WeightDownloader {
    fn start(&mut self) {
        let attempt = Arc::new(Attempt {
            target: self.target.clone(),
            events: self.events.clone(),
            ended: AtomicBool::new(false),
            last_byte: Mutex::new(Instant::now()),
        });
        let fetching = attempt.clone();
        std::thread::spawn(move || {
            if let Err(error) = fetching.fetch() {
                fetching.end(DownloadEvent::Failed(error));
            }
        });
        std::thread::spawn(move || {
            while !attempt.has_ended() {
                std::thread::sleep(Duration::from_secs(5));
                if attempt.idle() >= STALL_AFTER {
                    attempt.end(DownloadEvent::Failed(format!(
                        "超过 {} 秒没有收到数据。",
                        STALL_AFTER.as_secs()
                    )));
                }
            }
        });
    }
}

/// One run of the download. Whoever ends it first (the fetch or the stall
/// watchdog) sends the only terminal event; an abandoned fetch stops writing.
struct Attempt {
    target: PathBuf,
    events: Sender<DownloadEvent>,
    ended: AtomicBool,
    last_byte: Mutex<Instant>,
}

impl Attempt {
    fn end(&self, event: DownloadEvent) -> bool {
        if self.ended.swap(true, Ordering::SeqCst) {
            return false;
        }
        if let DownloadEvent::Failed(_) = event {
            let _ = fs::remove_file(self.partial());
        }
        let _ = self.events.send(event);
        true
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
            .get(format!("{WEIGHT_REPOSITORY}/{WEIGHT_FILE}"))
            .call()
            .map_err(|error| format!("连接下载地址失败：{error}"))?;
        let expected = response.body().content_length();
        let mut reader = response.into_body().into_reader();

        let file =
            File::create(self.partial()).map_err(|error| format!("写入模型失败：{error}"))?;
        let mut writer = BufWriter::new(file);
        let mut buffer = vec![0; 1 << 20];
        let mut received = 0;
        let mut reported = Instant::now();
        let report = |received: u64| {
            let total = expected.unwrap_or(WEIGHT_BYTES).max(received);
            let _ = self
                .events
                .send(DownloadEvent::Progressed { received, total });
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
        if self.has_ended() {
            return Ok(());
        }
        fs::rename(self.partial(), &self.target)
            .map_err(|error| format!("保存模型失败：{error}"))?;
        self.end(DownloadEvent::Finished);
        Ok(())
    }
}
