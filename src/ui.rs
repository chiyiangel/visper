use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use gpui::{
    Context, FocusHandle, PathPromptOptions, PromptButton, PromptLevel, ScrollHandle, Subscription,
    Timer, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

mod motion;
#[cfg(debug_assertions)]
mod preview;
mod render;
#[cfg(debug_assertions)]
pub use preview::open_preview;
pub use render::open;

use crate::download::{DownloadEvent, WeightDownloader};
use crate::markdown::MarkdownFile;
use crate::playback::{PlaybackState, Player};
use crate::store::Store;
use crate::transcribe::{TranscriptionEvent, WhisperTranscriber};

pub struct VisperWindow {
    library: visper::Library,
    notice: Option<String>,
    /// The Recording just exported, for a short "已导出" beside the button.
    exported: Option<PathBuf>,
    /// The 更多 menu holding 从资料库移除 is open.
    more_open: bool,
    player: Player,
    was_playing: bool,
    store: Store,
    downloader: WeightDownloader,
    download_events: Receiver<DownloadEvent>,
    transcriber: WhisperTranscriber,
    transcription_events: Receiver<TranscriptionEvent>,
    appearance: Option<Subscription>,
    selection: Rc<RefCell<render::TextSelection>>,
    selection_focus: FocusHandle,
    transcript_scroll: ScrollHandle,
    motion: motion::Motion,
    clock: Option<TranscriptionClock>,
    #[cfg(debug_assertions)]
    preview_appearance: Option<gpui::WindowAppearance>,
    #[cfg(debug_assertions)]
    preview_playback: Option<PlaybackState>,
    #[cfg(debug_assertions)]
    preview: bool,
    /// Quitting by any other route settles the Library the same way.
    _quitting: Subscription,
}

impl VisperWindow {
    pub fn new(store: Store, weight: PathBuf, cx: &mut Context<Self>) -> Self {
        Self::build(store, weight, None, cx)
    }

    fn build(
        mut store: Store,
        weight: PathBuf,
        preview_library: Option<visper::Library>,
        cx: &mut Context<Self>,
    ) -> Self {
        let preview = preview_library.is_some();
        let mut library = preview_library.unwrap_or_else(|| store.load());
        let (sender, download_events) = channel();
        let mut downloader = WeightDownloader::new(weight.clone(), sender);
        let (sender, transcription_events) = channel();
        let mut transcriber = WhisperTranscriber::new(weight.clone(), sender);
        let paths: Vec<PathBuf> = library
            .recordings()
            .iter()
            .map(|recording| recording.path.clone())
            .collect();
        if !preview {
            for path in paths {
                library.file_checked(&path, file_opens(&path), &mut transcriber);
            }
            library.prepare_weight(weight.is_file(), &mut downloader, &mut transcriber);
            store.save(&library);
        }
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(100)).await;
                if this.update(cx, |this, cx| this.take_events(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            library,
            notice: None,
            exported: None,
            more_open: false,
            player: Player::default(),
            was_playing: false,
            store,
            downloader,
            download_events,
            transcriber,
            transcription_events,
            appearance: None,
            selection: Rc::default(),
            selection_focus: cx.focus_handle(),
            transcript_scroll: ScrollHandle::new(),
            motion: motion::Motion::default(),
            clock: None,
            #[cfg(debug_assertions)]
            preview_appearance: None,
            #[cfg(debug_assertions)]
            preview_playback: None,
            #[cfg(debug_assertions)]
            preview,
            _quitting: cx.on_app_quit(|this, _| {
                this.quit();
                async {}
            }),
        }
    }

    fn take_events(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        let mut finished = false;
        let mut appended = false;
        let follow = self.transcript_scroll.max_offset().height + self.transcript_scroll.offset().y
            <= gpui::px(24.0);
        while let Ok(event) = self.download_events.try_recv() {
            changed = true;
            match event {
                DownloadEvent::Progressed { received, total } => {
                    self.library.download_progressed(received, total)
                }
                DownloadEvent::Failed(error) => self.library.download_failed(error),
                DownloadEvent::Finished => self.library.download_finished(&mut self.transcriber),
            }
        }
        while let Ok(event) = self.transcription_events.try_recv() {
            changed = true;
            match event {
                TranscriptionEvent::Segment { path, segment } => {
                    appended |= self
                        .library
                        .selected()
                        .is_some_and(|recording| recording.path == path);
                    self.library.segment_transcribed(&path, segment)
                }
                TranscriptionEvent::Progressed {
                    path,
                    processed_ms,
                    duration_ms,
                } => self
                    .library
                    .transcription_progressed(&path, processed_ms, duration_ms),
                TranscriptionEvent::Finished { path } => {
                    let elapsed_ms = self
                        .clock
                        .as_ref()
                        .filter(|clock| clock.path == path)
                        .map_or(0, |clock| clock.started.elapsed().as_millis() as u64);
                    self.library
                        .transcription_finished(&path, elapsed_ms, &mut self.transcriber);
                    finished = true;
                }
                TranscriptionEvent::Failed { path, error } => {
                    self.library
                        .transcription_failed(&path, error, &mut self.transcriber);
                    self.library
                        .file_checked(&path, file_opens(&path), &mut self.transcriber);
                    finished = true;
                }
            }
        }
        if appended && follow {
            self.transcript_scroll.scroll_to_bottom();
        }
        if finished {
            self.store.save(&self.library);
        }
        self.keep_playback_on_the_selection();
        changed |= self.follow_transcription();
        // Redraw while audio plays, and once more when it stops on its own.
        let playing = self
            .player
            .loaded_path()
            .and_then(|path| self.player.state(path))
            .is_some_and(|playback| playback.playing);
        if changed || playing || playing != self.was_playing {
            cx.notify();
        }
        self.was_playing = playing;
    }

    /// Starts the clock when a Recording becomes Transcribing and stops it
    /// when none is. Returns whether the shown time changed.
    fn follow_transcription(&mut self) -> bool {
        let running = self
            .library
            .recordings()
            .iter()
            .find(|recording| recording.status == visper::Status::Transcribing)
            .map(|recording| recording.path.clone());
        match (running, &mut self.clock) {
            (None, clock) => clock.take().is_some(),
            (Some(path), Some(clock)) if clock.path == path => {
                let second = clock.started.elapsed().as_secs();
                std::mem::replace(&mut clock.shown_second, second) != second
            }
            (Some(path), clock) => {
                *clock = Some(TranscriptionClock::new(path, Instant::now()));
                true
            }
        }
    }

    /// Elapsed time and the estimate for the running Transcription. The
    /// estimate is taken afresh every 5 seconds so the numbers do not jitter.
    fn timing(&mut self) -> Option<render::Timing> {
        let clock = self.clock.as_mut()?;
        let progress = self.library.recording(&clock.path)?.progress();
        let elapsed_ms = clock.started.elapsed().as_millis() as u64;
        if clock
            .estimate
            .as_ref()
            .is_none_or(|estimate| estimate.taken.elapsed() >= Duration::from_secs(5))
        {
            clock.estimate = Some(Estimate {
                taken: Instant::now(),
                remaining_ms: progress.and_then(|progress| progress.remaining_ms(elapsed_ms)),
                speed: progress.and_then(|progress| progress.speed(elapsed_ms)),
            });
        }
        let estimate = clock.estimate.as_ref()?;
        let since = estimate.taken.elapsed().as_millis() as u64;
        Some(render::Timing {
            path: clock.path.clone(),
            elapsed_ms,
            remaining_ms: estimate
                .remaining_ms
                .map(|remaining| remaining.saturating_sub(since)),
            speed: estimate.speed,
        })
    }

    /// Audio belongs to the selected Recording and only while it can play,
    /// whatever changed the selection or the status.
    fn keep_playback_on_the_selection(&mut self) {
        let Some(loaded) = self.player.loaded_path() else {
            return;
        };
        let still_playable = self
            .library
            .selected()
            .is_some_and(|recording| recording.path == loaded && recording.can_play());
        if !still_playable {
            self.player.stop();
        }
    }

    /// Cancels the Transcribing Recording, keeps the queue, and saves the
    /// snapshot the next launch resumes from. No notification is sent.
    fn quit(&mut self) {
        self.library.quit(&mut self.transcriber);
        self.player.stop();
        self.store.save(&self.library);
    }

    fn select(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self
            .library
            .selected()
            .is_none_or(|recording| recording.path != path)
        {
            *self.selection.borrow_mut() = render::TextSelection::default();
            self.transcript_scroll = ScrollHandle::new();
            #[cfg(debug_assertions)]
            {
                self.preview_playback = None;
            }
        }
        self.library.select(path);
        self.more_open = false;
        #[cfg(debug_assertions)]
        if self.preview {
            cx.notify();
            return;
        }
        self.library
            .file_checked(path, file_opens(path), &mut self.transcriber);
        self.keep_playback_on_the_selection();
        self.store.save(&self.library);
        cx.notify();
    }

    fn toggle_playback(&mut self, path: &Path, cx: &mut Context<Self>) {
        #[cfg(debug_assertions)]
        if self.preview {
            let mut state = self.preview_playback.unwrap_or(PlaybackState {
                playing: false,
                position_ms: 0,
                duration_ms: self
                    .library
                    .recording(path)
                    .and_then(|recording| recording.duration_ms()),
            });
            state.playing = !state.playing;
            self.preview_playback = Some(state);
            cx.notify();
            return;
        }
        let outcome = self.player.toggle(path);
        self.show_playback_error(outcome, cx);
    }

    fn play_from(&mut self, path: &Path, start_ms: u64, cx: &mut Context<Self>) {
        #[cfg(debug_assertions)]
        if self.preview {
            self.preview_playback = Some(PlaybackState {
                playing: true,
                position_ms: start_ms,
                duration_ms: self
                    .library
                    .recording(path)
                    .and_then(|recording| recording.duration_ms()),
            });
            cx.notify();
            return;
        }
        let outcome = self.player.play_from(path, start_ms);
        self.show_playback_error(outcome, cx);
    }

    fn show_playback_error(&mut self, outcome: Result<(), String>, cx: &mut Context<Self>) {
        if let Err(error) = outcome {
            self.notice = Some(error);
        }
        cx.notify();
    }

    fn set_language(&mut self, path: &Path, language: visper::Language, cx: &mut Context<Self>) {
        self.library.set_language(path, language);
        self.store.save(&self.library);
        cx.notify();
    }

    fn cancel(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.library.cancel(path, &mut self.transcriber);
        self.store.save(&self.library);
        cx.notify();
    }

    /// Asks first when a stored Transcript would go; the audio file stays.
    ///
    /// Dialogs here are GPUI's asynchronous ones: a synchronous modal would
    /// run the event loop while this listener still holds the App, and the
    /// event poll would then borrow it a second time.
    fn remove(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let needs_confirmation = self
            .library
            .recording(path)
            .is_some_and(|recording| recording.needs_removal_confirmation());
        if !needs_confirmation {
            self.remove_now(path, cx);
            return;
        }
        let answer = window.prompt(
            PromptLevel::Warning,
            "从资料库移除",
            Some("已保存的转写稿将被移除，原录音文件会保留。"),
            &[
                PromptButton::Ok("移除".into()),
                PromptButton::Cancel("取消".into()),
            ],
            cx,
        );
        let path = path.to_path_buf();
        cx.spawn(async move |this, cx| {
            if answer.await == Ok(0) {
                let _ = this.update(cx, |this, cx| this.remove_now(&path, cx));
            }
        })
        .detach();
    }

    fn remove_now(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.library.remove(path, &mut self.transcriber);
        self.keep_playback_on_the_selection();
        self.store.save(&self.library);
        cx.notify();
    }

    fn rerun(&mut self, path: &Path, cx: &mut Context<Self>) {
        // Events of an earlier run of this Recording land while it is not
        // Transcribing, where the Library ignores them.
        self.take_events(cx);
        if self
            .library
            .rerun(path, file_opens(path), &mut self.transcriber)
            == visper::RerunOutcome::CannotOpen
        {
            self.notice = Some("文件缺失，不能重新转写。".into());
        }
        self.store.save(&self.library);
        cx.notify();
    }

    fn export(&mut self, path: &Path, cx: &mut Context<Self>) {
        let folder = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("导出".into()),
        });
        let path = path.to_path_buf();
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut folders))) = folder.await
                && let Some(folder) = folders.pop()
            {
                let _ = this.update(cx, |this, cx| this.export_into(&path, &folder, cx));
            }
        })
        .detach();
    }

    fn export_into(&mut self, path: &Path, folder: &Path, cx: &mut Context<Self>) {
        match self.library.export(path, folder, &mut MarkdownFile) {
            visper::ExportOutcome::Written(_) => {
                self.exported = Some(path.to_path_buf());
                let shown = path.to_path_buf();
                cx.spawn(async move |this, cx| {
                    Timer::after(Duration::from_secs(3)).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.exported.as_ref() == Some(&shown) {
                            this.exported = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            visper::ExportOutcome::NothingStored => {
                self.notice = Some("不能导出。没有已保存的分段。".into())
            }
            visper::ExportOutcome::Failed(error) => {
                self.notice = Some(format!("导出失败：{error}"))
            }
        }
        cx.notify();
    }

    fn retry_download(&mut self, cx: &mut Context<Self>) {
        self.library.retry_download(&mut self.downloader);
        cx.notify();
    }

    fn admit(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let previous_selection = self
            .library
            .selected()
            .map(|recording| recording.path.clone());
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let (path, can_open) = inspect_recording(path);
        // As in `rerun`: an earlier run's events must not reach a new one.
        self.take_events(cx);
        self.notice = match self.library.add(path, can_open, &mut self.transcriber) {
            visper::AddOutcome::Queued => None,
            visper::AddOutcome::AlreadyQueued => {
                Some(format!("「{name}」已经在等待中，没有再加一条。"))
            }
            visper::AddOutcome::AlreadyTranscribing => Some(format!("「{name}」正在转写。")),
            visper::AddOutcome::Rejected(reason) => Some(reason),
        };
        if self.library.selected().map(|recording| &recording.path) != previous_selection.as_ref() {
            *self.selection.borrow_mut() = render::TextSelection::default();
            self.transcript_scroll = ScrollHandle::new();
        }
        self.keep_playback_on_the_selection();
        self.store.save(&self.library);
        cx.notify();
    }

    fn pick_file(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("加入".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await {
                let _ = this.update(cx, |this, cx| {
                    for path in paths {
                        this.admit(path, cx);
                    }
                });
            }
        })
        .detach();
    }
}

/// Times the running Transcription; the Library itself keeps no clock.
struct TranscriptionClock {
    path: PathBuf,
    started: Instant,
    shown_second: u64,
    estimate: Option<Estimate>,
}

struct Estimate {
    taken: Instant,
    remaining_ms: Option<u64>,
    speed: Option<f64>,
}

impl TranscriptionClock {
    fn new(path: PathBuf, started: Instant) -> Self {
        Self {
            path,
            started,
            shown_second: 0,
            estimate: None,
        }
    }
}

fn file_opens(path: &Path) -> bool {
    path.is_file() && fs::File::open(path).is_ok()
}

fn inspect_recording(path: PathBuf) -> (PathBuf, bool) {
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    if file_opens(&absolute) {
        (absolute.canonicalize().unwrap_or(absolute), true)
    } else {
        (absolute, false)
    }
}

pub fn support_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/Visper")
}
