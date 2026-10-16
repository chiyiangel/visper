use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ferrous_opencc::OpenCC;
use ferrous_opencc::config::BuiltinConfig;

use serde::{Deserialize, Serialize};

const AUDIO_EXTENSIONS: &[&str] = &["m4a", "mp3", "wav", "flac", "aac"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Queued,
    Transcribing,
    Completed,
    Failed,
    Cancelled,
}

impl Status {
    pub fn group(&self) -> Group {
        match self {
            Status::Transcribing => Group::Transcribing,
            Status::Queued => Group::Queued,
            Status::Completed | Status::Failed | Status::Cancelled => Group::Finished,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Status::Queued => "等待中",
            Status::Transcribing => "转写中",
            Status::Completed => "完成",
            Status::Failed => "失败",
            Status::Cancelled => "已取消",
        }
    }
}

/// The three sections of the Library list, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Transcribing,
    Queued,
    /// Completed, Failed and Cancelled, most recently finished first.
    Finished,
}

impl Group {
    pub fn label(&self) -> &'static str {
        match self {
            Group::Transcribing => "转写中",
            Group::Queued => "等待中",
            Group::Finished => "已结束",
        }
    }
}

/// How a Transcription treats speech: per-chunk Chinese or English in Auto
/// (ADR 0002), or the whole Recording locked to one language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Auto,
    Chinese,
    English,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Language::Auto => "自动",
            Language::Chinese => "中文",
            Language::English => "英文",
        }
    }
}

/// A stretch of Transcript text. Times are milliseconds from the start of
/// the Recording; the text is already Simplified Chinese and/or English.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

impl Segment {
    pub fn start_label(&self) -> String {
        clock(self.start_ms)
    }
}

/// How far the running Transcription has got through the Recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub processed_ms: u64,
    pub duration_ms: u64,
}

impl Progress {
    pub fn label(&self) -> String {
        format!(
            "{} / {} · {}%",
            clock(self.processed_ms),
            clock(self.duration_ms),
            (self.fraction() * 100.0).floor()
        )
    }

    pub fn fraction(&self) -> f32 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        (self.processed_ms as f32 / self.duration_ms as f32).min(1.0)
    }

    /// Seconds of audio transcribed per second of work so far. None in the
    /// first 30 seconds or 3 percent, before the pace is steady enough to
    /// promise anything.
    pub fn speed(&self, elapsed_ms: u64) -> Option<f64> {
        if elapsed_ms < 30_000 || self.processed_ms == 0 || self.fraction() < 0.03 {
            return None;
        }
        Some(self.processed_ms as f64 / elapsed_ms as f64)
    }

    /// How long the rest of the Recording should take at the pace so far.
    pub fn remaining_ms(&self, elapsed_ms: u64) -> Option<u64> {
        let speed = self.speed(elapsed_ms)?;
        let left = self.duration_ms.saturating_sub(self.processed_ms);
        Some((left as f64 / speed).round() as u64)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recording {
    pub path: PathBuf,
    pub status: Status,
    #[serde(default)]
    pub language: Language,
    /// Actual decoded duration, retained after progress is cleared on completion.
    #[serde(default)]
    duration_ms: Option<u64>,
    /// The Transcript as last stored by a successful Transcription.
    #[serde(default)]
    segments: Vec<Segment>,
    /// Whether `segments` came from a success; an empty Transcript counts.
    #[serde(default)]
    transcript_stored: bool,
    /// Larger means it finished more recently.
    #[serde(default)]
    finished_order: u64,
    /// The path no longer opens. Not a status: the status stays as it was.
    #[serde(default)]
    missing: bool,
    /// Why the last Transcription failed.
    #[serde(default)]
    error: Option<String>,
    /// How long the last successful Transcription took.
    #[serde(default)]
    transcription_ms: Option<u64>,
    /// The Model that produced `segments`. Missing in a Transcript stored
    /// before there was a choice: that was Large v3.
    #[serde(default)]
    transcript_model: Option<Model>,
    /// The Model the running Transcription started with, which may no
    /// longer be the current one.
    #[serde(skip)]
    running_model: Option<Model>,
    /// What the running Transcription has produced so far.
    #[serde(skip)]
    live: Vec<Segment>,
    #[serde(skip)]
    progress: Option<Progress>,
}

impl Recording {
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The live Segments while Transcribing, otherwise the stored ones.
    pub fn visible_segments(&self) -> &[Segment] {
        if self.status == Status::Transcribing {
            &self.live
        } else {
            &self.segments
        }
    }

    pub fn stored_segments(&self) -> &[Segment] {
        &self.segments
    }

    /// There is something to align the audio with: visible Segments, a
    /// stored Transcript, or a Completed one even if it is empty.
    pub fn can_play(&self) -> bool {
        !self.missing && self.has_something_to_play()
    }

    fn has_something_to_play(&self) -> bool {
        !self.visible_segments().is_empty() || self.transcript_stored
    }

    /// A Transcription has succeeded at some point, even with no Segments.
    pub fn has_stored_transcript(&self) -> bool {
        self.transcript_stored
    }

    /// The index of the visible Segment playback at `position_ms` is in.
    pub fn segment_at(&self, position_ms: u64) -> Option<usize> {
        self.visible_segments()
            .iter()
            .position(|segment| (segment.start_ms..segment.end_ms).contains(&position_ms))
    }

    /// Removing would drop a stored Transcript, even an empty Completed one.
    pub fn needs_removal_confirmation(&self) -> bool {
        self.transcript_stored
    }

    /// Only a finished Recording can be queued again.
    pub fn can_rerun(&self) -> bool {
        !self.missing && self.is_finished()
    }

    fn is_finished(&self) -> bool {
        self.status.group() == Group::Finished
    }

    pub fn is_missing(&self) -> bool {
        self.missing
    }

    /// A rerun is showing new Segments while export still writes the
    /// stored Transcript.
    pub fn exports_previous_transcript(&self) -> bool {
        self.status == Status::Transcribing && self.can_export()
    }

    /// Export needs at least one stored Segment.
    pub fn can_export(&self) -> bool {
        !self.segments.is_empty()
    }

    /// The Recording's file name with `.md` in place of its extension.
    pub fn markdown_file_name(&self) -> String {
        let stem = self.path.file_stem().unwrap_or(self.path.as_os_str());
        format!("{}.md", stem.to_string_lossy())
    }

    /// One `[HH:MM:SS] text` block per stored Segment, blank line between.
    pub fn markdown(&self) -> String {
        let blocks: Vec<String> = self
            .segments
            .iter()
            .map(|segment| format!("[{}] {}", segment.start_label(), segment.text))
            .collect();
        format!("{}\n", blocks.join("\n\n"))
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn progress(&self) -> Option<Progress> {
        self.progress
    }

    /// How long the Transcription took, shown only while it is the Completed
    /// one; Recordings stored before this was kept have none.
    pub fn transcription_ms(&self) -> Option<u64> {
        (self.status == Status::Completed)
            .then_some(self.transcription_ms)
            .flatten()
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }

    /// The Model of the stored Transcript, if there is one.
    pub fn transcript_model(&self) -> Option<Model> {
        self.transcript_stored
            .then(|| self.transcript_model.unwrap_or(Model::LargeV3))
    }

    /// The Model to show beside a finished Recording: the stored
    /// Transcript's, but not while a new Transcription is queued or running.
    pub fn shown_model(&self) -> Option<Model> {
        self.transcript_model().filter(|_| self.is_finished())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddOutcome {
    Queued,
    AlreadyQueued,
    AlreadyTranscribing,
    Rejected(String),
}

/// A Whisper model Visper can transcribe with (ADR 0004). Each one is a
/// weight file that is downloaded once and then kept on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Model {
    LargeV3,
    LargeV3Turbo,
}

impl Model {
    /// In the order the Model popover lists them.
    pub const ALL: [Model; 2] = [Model::LargeV3, Model::LargeV3Turbo];

    pub fn full_name(self) -> &'static str {
        match self {
            Model::LargeV3 => "Large v3",
            Model::LargeV3Turbo => "Large v3 Turbo",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Model::LargeV3 => "Large",
            Model::LargeV3Turbo => "Turbo",
        }
    }

    /// The weight's file name, both on disk and in the download repository.
    pub fn weight_file(self) -> &'static str {
        match self {
            Model::LargeV3 => "ggml-large-v3.bin",
            Model::LargeV3Turbo => "ggml-large-v3-turbo.bin",
        }
    }

    /// Approximate size of the weight in bytes, for when the server sends
    /// no length.
    pub fn bytes(self) -> u64 {
        match self {
            Model::LargeV3 => 3_095_033_483,
            Model::LargeV3Turbo => 1_624_555_275,
        }
    }
}

/// Fetches the weight of a Model in the background and reports back through
/// the `download_*` actions on [`Library`].
pub trait Downloader {
    fn start(&mut self, model: Model);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RerunOutcome {
    Queued,
    /// Only a Completed, Failed or Cancelled Recording can run again.
    NotFinished,
    /// The Recording's path no longer opens.
    CannotOpen,
}

/// Puts an exported Transcript on disk, replacing a file of the same name.
/// It does not open the file or reveal it.
pub trait MarkdownWriter {
    fn write(&mut self, file: &Path, body: &str) -> Result<(), String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportOutcome {
    Written(PathBuf),
    /// There is no stored Segment to export.
    NothingStored,
    Failed(String),
}

/// Runs one Transcription in the background and reports back through the
/// `transcription_*` actions on [`Library`].
pub trait Transcriber {
    fn start(&mut self, path: &Path, language: Language, model: Model);
    /// Cancels the Transcription that was started last.
    fn cancel(&mut self);
}

/// The one weight being fetched, or the last fetch that failed. It is not
/// tied to the current Model: a download can outlive a switch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Download {
    #[default]
    Idle,
    Running {
        model: Model,
        received: u64,
        total: u64,
    },
    Failed {
        model: Model,
        error: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Library {
    recordings: Vec<Recording>,
    selected: Option<PathBuf>,
    /// The Model that Transcriptions starting from now on use. Missing in a
    /// library saved before the choice existed.
    #[serde(default)]
    model: Option<Model>,
    /// Which weights are stored; told at [`Library::prepare`] and kept up to
    /// date by finished downloads.
    #[serde(skip)]
    on_disk: Vec<Model>,
    #[serde(skip)]
    download: Download,
    #[serde(default)]
    finished_count: u64,
    /// Set by [`Library::quit`]; from then on nothing starts.
    #[serde(skip)]
    quitting: bool,
}

impl Library {
    pub fn new() -> Self {
        Self {
            recordings: Vec::new(),
            selected: None,
            model: None,
            on_disk: Vec::new(),
            download: Download::Idle,
            finished_count: 0,
            quitting: false,
        }
    }

    pub fn model(&self) -> Option<Model> {
        self.model
    }

    pub fn download(&self) -> &Download {
        &self.download
    }

    pub fn is_on_disk(&self, model: Model) -> bool {
        self.on_disk.contains(&model)
    }

    /// Whether the current Model's weight is on disk, so Queued Recordings
    /// can start.
    pub fn is_ready(&self) -> bool {
        self.model.is_some_and(|model| self.is_on_disk(model))
    }

    /// Run once at startup, with the Models whose weight is stored.
    pub fn prepare(
        &mut self,
        on_disk: &[Model],
        downloader: &mut dyn Downloader,
        transcriber: &mut dyn Transcriber,
    ) {
        self.on_disk = on_disk.to_vec();
        // Large v3 was the only Model before there was a choice.
        let model = *self.model.get_or_insert(Model::LargeV3);
        if self.is_on_disk(model) {
            self.start_next(transcriber);
        } else {
            self.start_download(model, downloader);
        }
    }

    /// Makes `model` the one later Transcriptions use, fetching its weight
    /// first when it is not stored. A running Transcription is left alone.
    pub fn choose(
        &mut self,
        model: Model,
        downloader: &mut dyn Downloader,
        transcriber: &mut dyn Transcriber,
    ) {
        if self.model == Some(model) {
            return;
        }
        self.model = Some(model);
        if self.is_on_disk(model) {
            self.start_next(transcriber);
        } else if !self.is_downloading(model) {
            // A download of the other Model, if one runs, is not stopped; it
            // only stops being shown.
            self.start_download(model, downloader);
        }
    }

    pub fn download_progressed(&mut self, model: Model, received: u64, total: u64) {
        if self.is_downloading(model) {
            self.download = Download::Running {
                model,
                received,
                total,
            };
        }
    }

    pub fn download_failed(&mut self, model: Model, error: String) {
        if self.is_downloading(model) {
            self.download = Download::Failed { model, error };
        }
    }

    pub fn download_finished(&mut self, model: Model, transcriber: &mut dyn Transcriber) {
        // The weight is stored whichever download the window is showing.
        if !self.is_on_disk(model) {
            self.on_disk.push(model);
        }
        if self.is_downloading(model) {
            self.download = Download::Idle;
        }
        if self.model == Some(model) {
            self.start_next(transcriber);
        }
    }

    pub fn retry_download(&mut self, downloader: &mut dyn Downloader) {
        if let Download::Failed { model, .. } = self.download {
            self.start_download(model, downloader);
        }
    }

    fn start_download(&mut self, model: Model, downloader: &mut dyn Downloader) {
        self.download = Download::Running {
            model,
            received: 0,
            total: 0,
        };
        downloader.start(model);
    }

    /// The language can change only while the Recording is Queued.
    pub fn set_language(&mut self, path: &Path, language: Language) {
        if let Some(recording) = self
            .recordings
            .iter_mut()
            .find(|recording| recording.path == path && recording.status == Status::Queued)
        {
            recording.language = language;
        }
    }

    pub fn segment_transcribed(&mut self, path: &Path, segment: Segment) {
        if let Some(recording) = self.transcribing(path) {
            recording.live.push(Segment {
                text: to_simplified(&segment.text),
                ..segment
            });
        }
    }

    pub fn transcription_progressed(&mut self, path: &Path, processed_ms: u64, duration_ms: u64) {
        if let Some(recording) = self.transcribing(path) {
            if duration_ms > 0 {
                recording.duration_ms = Some(duration_ms);
            }
            recording.progress = Some(Progress {
                processed_ms,
                duration_ms,
            });
        }
    }

    /// `elapsed_ms` is how long the Transcription ran, kept with the Transcript.
    pub fn transcription_finished(
        &mut self,
        path: &Path,
        elapsed_ms: u64,
        transcriber: &mut dyn Transcriber,
    ) {
        let Some(recording) = self.transcribing(path) else {
            return;
        };
        recording.segments = std::mem::take(&mut recording.live);
        recording.transcript_stored = true;
        recording.transcription_ms = Some(elapsed_ms);
        recording.transcript_model = recording
            .running_model
            .take()
            .or(recording.transcript_model);
        self.end(path, Status::Completed, transcriber);
    }

    /// Cancels the Transcribing Recording. Its live Segments are dropped; the
    /// Recording and any stored Transcript stay.
    pub fn cancel(&mut self, path: &Path, transcriber: &mut dyn Transcriber) {
        if self.transcribing(path).is_some() {
            transcriber.cancel();
            self.end(path, Status::Cancelled, transcriber);
        }
    }

    /// Records whether the Recording's path still opens, as found by the
    /// shell or by a Transcription that could not read it.
    /// A Queued Recording that opens again may be next in line.
    pub fn file_checked(&mut self, path: &Path, can_open: bool, transcriber: &mut dyn Transcriber) {
        if let Some(recording) = self.recording_mut(path) {
            recording.missing = !can_open;
        }
        self.start_next(transcriber);
    }

    /// Takes the Recording out of the Library; the audio file stays where it
    /// is. A Transcribing Recording is cancelled first. Whether to ask is
    /// [`Recording::needs_removal_confirmation`], decided before calling this.
    pub fn remove(&mut self, path: &Path, transcriber: &mut dyn Transcriber) {
        self.cancel(path, transcriber);
        self.recordings.retain(|recording| recording.path != path);
        if self.selected.as_deref() == Some(path) {
            self.selected = None;
        }
    }

    /// Like a cancel, but the error stays for the detail to show.
    pub fn transcription_failed(
        &mut self,
        path: &Path,
        error: String,
        transcriber: &mut dyn Transcriber,
    ) {
        if let Some(recording) = self.transcribing(path) {
            recording.error = Some(error);
            self.end(path, Status::Failed, transcriber);
        }
    }

    /// Ends the Transcription in progress. Live Segments not stored by now
    /// are dropped.
    fn end(&mut self, path: &Path, status: Status, transcriber: &mut dyn Transcriber) {
        self.end_without_starting_next(path, status);
        self.start_next(transcriber);
    }

    /// Like [`Library::end`], without starting the next Recording.
    fn end_without_starting_next(&mut self, path: &Path, status: Status) {
        let finished_order = self.finished_count + 1;
        let Some(recording) = self.transcribing(path) else {
            return;
        };
        recording.finished_order = finished_order;
        recording.status = status;
        recording.live.clear();
        recording.progress = None;
        self.finished_count = finished_order;
    }

    /// Closing Visper: the Transcribing Recording is Cancelled as by a user
    /// cancel, Queued ones stay Queued for the next launch, and nothing new
    /// starts.
    pub fn quit(&mut self, transcriber: &mut dyn Transcriber) {
        self.quitting = true;
        if let Some(path) = self.transcribing_path() {
            transcriber.cancel();
            self.end_without_starting_next(&path, Status::Cancelled);
        }
    }

    fn transcribing_path(&self) -> Option<PathBuf> {
        self.recordings
            .iter()
            .find(|recording| recording.status == Status::Transcribing)
            .map(|recording| recording.path.clone())
    }

    fn transcribing(&mut self, path: &Path) -> Option<&mut Recording> {
        self.recordings
            .iter_mut()
            .find(|recording| recording.path == path && recording.status == Status::Transcribing)
    }

    fn is_downloading(&self, model: Model) -> bool {
        matches!(self.download, Download::Running { model: current, .. } if current == model)
    }

    /// Queues a finished Recording again, behind everything already Queued.
    /// Its stored Transcript stays until the new Transcription succeeds.
    pub fn rerun(
        &mut self,
        path: &Path,
        can_open: bool,
        transcriber: &mut dyn Transcriber,
    ) -> RerunOutcome {
        let Some(index) = self
            .recordings
            .iter()
            .position(|recording| recording.path == path)
        else {
            return RerunOutcome::NotFinished;
        };
        if !self.recordings[index].is_finished() {
            return RerunOutcome::NotFinished;
        }
        self.recordings[index].missing = !can_open;
        if !can_open {
            return RerunOutcome::CannotOpen;
        }
        let mut recording = self.recordings.remove(index);
        recording.status = Status::Queued;
        recording.error = None;
        self.recordings.push(recording);
        self.start_next(transcriber);
        RerunOutcome::Queued
    }

    /// The single pump: once the current Model's weight is stored and
    /// nothing is Transcribing, the earliest Queued Recording starts.
    fn start_next(&mut self, transcriber: &mut dyn Transcriber) {
        let Some(model) = self.model.filter(|model| self.is_on_disk(*model)) else {
            return;
        };
        if self.quitting || self.transcribing_path().is_some() {
            return;
        }
        let Some(next) = self
            .recordings
            .iter_mut()
            .find(|recording| recording.status == Status::Queued && !recording.missing)
        else {
            return;
        };
        next.status = Status::Transcribing;
        next.error = None;
        next.running_model = Some(model);
        transcriber.start(&next.path, next.language, model);
    }

    pub fn recordings(&self) -> &[Recording] {
        &self.recordings
    }

    /// The non-empty sections of the list, in display order.
    pub fn groups(&self) -> Vec<(Group, Vec<&Recording>)> {
        let in_group = |group: Group| -> Vec<&Recording> {
            self.recordings
                .iter()
                .filter(|recording| recording.status.group() == group)
                .collect()
        };
        let mut finished = in_group(Group::Finished);
        finished.sort_by_key(|recording| std::cmp::Reverse(recording.finished_order));
        [
            (Group::Transcribing, in_group(Group::Transcribing)),
            (Group::Queued, in_group(Group::Queued)),
            (Group::Finished, finished),
        ]
        .into_iter()
        .filter(|(_, recordings)| !recordings.is_empty())
        .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.recordings.is_empty()
    }

    pub fn selected(&self) -> Option<&Recording> {
        self.recording(self.selected.as_ref()?)
    }

    pub fn recording(&self, path: &Path) -> Option<&Recording> {
        self.recordings
            .iter()
            .find(|recording| recording.path == path)
    }

    fn recording_mut(&mut self, path: &Path) -> Option<&mut Recording> {
        self.recordings
            .iter_mut()
            .find(|recording| recording.path == path)
    }

    pub fn select(&mut self, path: &Path) {
        if self.recording(path).is_some() {
            self.selected = Some(path.to_path_buf());
        }
    }

    pub fn add(
        &mut self,
        path: PathBuf,
        can_open: bool,
        transcriber: &mut dyn Transcriber,
    ) -> AddOutcome {
        let name = file_name(&path);
        if !is_audio(&path) {
            return AddOutcome::Rejected(format!(
                "不能加入「{name}」。第一期只收 m4a、mp3、wav、flac、aac。"
            ));
        }
        if !can_open {
            return AddOutcome::Rejected(format!("打不开「{name}」，没有加入资料库。"));
        }
        match self
            .recording(&path)
            .map(|recording| recording.status.group())
        {
            Some(Group::Queued) => return AddOutcome::AlreadyQueued,
            Some(Group::Transcribing) => return AddOutcome::AlreadyTranscribing,
            Some(Group::Finished) => {
                self.selected = Some(path.clone());
                return match self.rerun(&path, can_open, transcriber) {
                    RerunOutcome::Queued => AddOutcome::Queued,
                    RerunOutcome::NotFinished => AddOutcome::AlreadyQueued,
                    RerunOutcome::CannotOpen => {
                        AddOutcome::Rejected(format!("打不开「{name}」，没有加入资料库。"))
                    }
                };
            }
            None => {}
        }
        self.recordings.push(Recording {
            path: path.clone(),
            status: Status::Queued,
            language: Language::Auto,
            duration_ms: None,
            segments: Vec::new(),
            transcript_stored: false,
            finished_order: 0,
            missing: false,
            error: None,
            transcription_ms: None,
            transcript_model: None,
            running_model: None,
            live: Vec::new(),
            progress: None,
        });
        self.selected = Some(path);
        self.start_next(transcriber);
        AddOutcome::Queued
    }

    /// Writes the stored Transcript of `path` as `<file stem>.md` in `folder`.
    pub fn export(
        &self,
        path: &Path,
        folder: &Path,
        writer: &mut dyn MarkdownWriter,
    ) -> ExportOutcome {
        let Some(recording) = self
            .recording(path)
            .filter(|recording| recording.can_export())
        else {
            return ExportOutcome::NothingStored;
        };
        let file = folder.join(recording.markdown_file_name());
        match writer.write(&file, &recording.markdown()) {
            Ok(()) => ExportOutcome::Written(file),
            Err(error) => ExportOutcome::Failed(error),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Nothing is running in a freshly loaded Library, so a Recording that
    /// was Transcribing when the snapshot was taken waits its turn again.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut library: Self = serde_json::from_str(json)?;
        for recording in &mut library.recordings {
            if recording.status == Status::Transcribing {
                recording.status = Status::Queued;
            }
            // Snapshots from before the flag existed.
            recording.transcript_stored |=
                recording.status == Status::Completed || !recording.segments.is_empty();
        }
        Ok(library)
    }
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

/// `HH:MM:SS`, also for Recordings shorter than an hour.
pub fn clock(ms: u64) -> String {
    let seconds = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}

/// Traditional characters become Simplified; Latin text passes through.
fn to_simplified(text: &str) -> String {
    static CONVERTER: OnceLock<OpenCC> = OnceLock::new();
    CONVERTER
        .get_or_init(|| {
            OpenCC::from_config(BuiltinConfig::T2s).expect("the built-in T2s dictionary loads")
        })
        .convert(text)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> PathBuf {
        PathBuf::from(value)
    }

    #[test]
    fn adding_an_m4a_recording_queues_it_and_selects_it() {
        let mut library = Library::new();
        let recording = path("/Users/me/录音/周会.m4a");

        let outcome = library.add(recording.clone(), true, &mut FakeTranscriber::default());

        assert_eq!(outcome, AddOutcome::Queued);
        assert_eq!(library.recordings().len(), 1);
        assert_eq!(library.recordings()[0].path, recording);
        assert_eq!(library.recordings()[0].status, Status::Queued);
        assert_eq!(library.recordings()[0].status.label(), "等待中");
        assert_eq!(library.recordings()[0].file_name(), "周会.m4a");
        assert_eq!(
            library.selected().map(|recording| &recording.path),
            Some(&recording)
        );
    }

    #[test]
    fn mp3_wav_flac_and_aac_are_queued() {
        let mut library = Library::new();
        for name in ["a.mp3", "b.wav", "c.flac", "d.aac", "e.M4A"] {
            let outcome = library.add(
                path(&format!("/Users/me/录音/{name}")),
                true,
                &mut FakeTranscriber::default(),
            );
            assert_eq!(outcome, AddOutcome::Queued, "{name}");
        }
        assert_eq!(library.recordings().len(), 5);
    }

    #[test]
    fn a_video_file_is_refused_and_stays_out_of_the_library() {
        let mut library = Library::new();

        let outcome = library.add(
            path("/Users/me/影片/录像.mp4"),
            true,
            &mut FakeTranscriber::default(),
        );

        assert_eq!(
            outcome,
            AddOutcome::Rejected(
                "不能加入「录像.mp4」。第一期只收 m4a、mp3、wav、flac、aac。".into()
            )
        );
        assert!(library.is_empty());
    }

    #[test]
    fn an_unreadable_audio_file_is_refused() {
        let mut library = Library::new();

        let outcome = library.add(
            path("/Users/me/录音/锁住.m4a"),
            false,
            &mut FakeTranscriber::default(),
        );

        assert_eq!(
            outcome,
            AddOutcome::Rejected("打不开「锁住.m4a」，没有加入资料库。".into())
        );
        assert!(library.is_empty());
    }

    #[test]
    fn the_same_file_name_in_two_folders_are_two_recordings() {
        let mut library = Library::new();
        library.add(
            path("/Users/me/录音/meeting.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );
        library.add(
            path("/Users/me/归档/meeting.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );

        assert_eq!(library.recordings().len(), 2);
        assert_eq!(library.recordings()[0].file_name(), "meeting.m4a");
        assert_eq!(library.recordings()[1].file_name(), "meeting.m4a");
    }

    #[test]
    fn adding_a_queued_path_again_does_not_create_a_second_entry() {
        let mut library = Library::new();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut FakeTranscriber::default());

        let outcome = library.add(recording.clone(), true, &mut FakeTranscriber::default());

        assert_eq!(outcome, AddOutcome::AlreadyQueued);
        assert_eq!(library.recordings().len(), 1);
        assert_eq!(
            library.selected().map(|recording| recording.path.clone()),
            Some(recording)
        );
    }

    #[test]
    fn only_one_recording_is_selected() {
        let mut library = Library::new();
        let first = path("/Users/me/录音/一.m4a");
        let second = path("/Users/me/录音/二.mp3");
        library.add(first.clone(), true, &mut FakeTranscriber::default());
        library.add(second.clone(), true, &mut FakeTranscriber::default());
        library.select(&first);

        assert_eq!(
            library.selected().map(|recording| recording.path.clone()),
            Some(first)
        );
        library.select(&second);
        assert_eq!(
            library.selected().map(|recording| recording.path.clone()),
            Some(second)
        );
    }

    #[test]
    fn a_restored_library_keeps_recordings_and_the_selection() {
        let mut library = Library::new();
        let first = path("/Users/me/录音/一.wav");
        let second = path("/Users/me/录音/二.flac");
        library.add(first.clone(), true, &mut FakeTranscriber::default());
        library.add(second, true, &mut FakeTranscriber::default());
        library.select(&first);

        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");

        assert_eq!(restored.recordings(), library.recordings());
        assert_eq!(
            restored.selected().map(|recording| recording.path.clone()),
            Some(first)
        );
    }

    #[test]
    fn a_broken_snapshot_is_not_a_library() {
        assert!(Library::from_json("{").is_err());
    }

    #[derive(Default)]
    struct FakeDownloader {
        starts: usize,
        models: Vec<Model>,
    }

    impl Downloader for FakeDownloader {
        fn start(&mut self, model: Model) {
            self.starts += 1;
            self.models.push(model);
        }
    }

    #[test]
    fn a_missing_weight_starts_downloading_without_asking() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();

        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());

        assert_eq!(downloader.starts, 1);
        assert_eq!(downloader.models, vec![Model::LargeV3]);
        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3,
                received: 0,
                total: 0
            }
        );
    }

    #[test]
    fn large_v3_is_named_and_sized_for_its_weight() {
        assert_eq!(Model::LargeV3.full_name(), "Large v3");
        assert_eq!(Model::LargeV3.short_name(), "Large");
        assert_eq!(Model::LargeV3.weight_file(), "ggml-large-v3.bin");
        assert_eq!(Model::LargeV3.bytes(), 3_095_033_483);
    }

    #[test]
    fn large_v3_turbo_is_named_and_sized_for_its_weight() {
        assert_eq!(Model::LargeV3Turbo.full_name(), "Large v3 Turbo");
        assert_eq!(Model::LargeV3Turbo.short_name(), "Turbo");
        assert_eq!(Model::LargeV3Turbo.weight_file(), "ggml-large-v3-turbo.bin");
        assert_eq!(Model::LargeV3Turbo.bytes(), 1_624_555_275);
        assert_eq!(Model::ALL, [Model::LargeV3, Model::LargeV3Turbo]);
    }

    #[test]
    fn a_library_from_before_the_choice_comes_up_as_large_v3() {
        let mut library = Library::from_json(r#"{"recordings":[],"selected":null}"#).expect("old");
        let mut downloader = FakeDownloader::default();

        library.prepare(
            &[Model::LargeV3],
            &mut downloader,
            &mut FakeTranscriber::default(),
        );

        assert_eq!(library.model(), Some(Model::LargeV3));
        assert!(library.is_ready());
        assert_eq!(downloader.starts, 0);
    }

    #[test]
    fn the_current_model_is_saved_and_restored() {
        let mut library = Library::new();
        library.prepare(
            &[Model::LargeV3, Model::LargeV3Turbo],
            &mut FakeDownloader::default(),
            &mut FakeTranscriber::default(),
        );
        library.choose(
            Model::LargeV3Turbo,
            &mut FakeDownloader::default(),
            &mut FakeTranscriber::default(),
        );

        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");

        assert_eq!(restored.model(), Some(Model::LargeV3Turbo));
    }

    /// Both weights on disk, Large v3 current, as after switching once.
    fn library_with_both_weights() -> (Library, FakeTranscriber) {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(
            &[Model::LargeV3, Model::LargeV3Turbo],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );
        (library, transcriber)
    }

    #[test]
    fn choosing_a_model_that_is_not_on_disk_downloads_it_and_queued_recordings_wait() {
        let (mut library, mut transcriber) = ready_library();
        let mut downloader = FakeDownloader::default();
        let waiting = path("/Users/me/录音/三.m4a");

        library.choose(Model::LargeV3Turbo, &mut downloader, &mut transcriber);
        library.add(waiting.clone(), true, &mut transcriber);

        assert_eq!(library.model(), Some(Model::LargeV3Turbo));
        assert_eq!(downloader.models, vec![Model::LargeV3Turbo]);
        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3Turbo,
                received: 0,
                total: 0
            }
        );
        assert!(!library.is_ready());
        assert_eq!(library.recording(&waiting).unwrap().status, Status::Queued);
        assert!(transcriber.started.is_empty());

        library.download_finished(Model::LargeV3Turbo, &mut transcriber);

        assert!(library.is_ready());
        assert_eq!(library.download(), &Download::Idle);
        assert_eq!(
            library.recording(&waiting).unwrap().status,
            Status::Transcribing
        );
        assert_eq!(transcriber.models.last(), Some(&Model::LargeV3Turbo));
        assert_eq!(downloader.starts, 1);
    }

    #[test]
    fn switching_during_a_transcription_lets_it_finish_and_the_next_one_uses_the_new_model() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let first = path("/Users/me/录音/一.m4a");
        let second = path("/Users/me/录音/二.m4a");
        library.add(first.clone(), true, &mut transcriber);
        library.add(second.clone(), true, &mut transcriber);

        library.choose(
            Model::LargeV3Turbo,
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        assert_eq!(transcriber.cancels, 0);
        assert_eq!(
            library.recording(&first).unwrap().status,
            Status::Transcribing
        );
        assert_eq!(transcriber.models, vec![Model::LargeV3]);

        library.transcription_finished(&first, 1_000, &mut transcriber);

        assert_eq!(
            library.recording(&second).unwrap().status,
            Status::Transcribing
        );
        assert_eq!(
            transcriber.models,
            vec![Model::LargeV3, Model::LargeV3Turbo]
        );
    }

    #[test]
    fn choosing_the_current_model_again_does_nothing() {
        let (mut library, mut transcriber) = ready_library();
        let mut downloader = FakeDownloader::default();
        let before = library.clone();

        library.choose(Model::LargeV3, &mut downloader, &mut transcriber);

        assert_eq!(library, before);
        assert_eq!(downloader.starts, 0);
        assert!(transcriber.started.is_empty());
    }

    #[test]
    fn choosing_a_model_on_disk_needs_no_download_and_the_next_recording_uses_it() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let mut downloader = FakeDownloader::default();
        library.choose(Model::LargeV3Turbo, &mut downloader, &mut transcriber);
        let waiting = path("/Users/me/录音/一.m4a");

        library.add(waiting.clone(), true, &mut transcriber);

        assert_eq!(transcriber.models, vec![Model::LargeV3Turbo]);
        assert_eq!(downloader.starts, 0);

        library.transcription_finished(&waiting, 1_000, &mut transcriber);
        let next = path("/Users/me/录音/二.m4a");
        library.choose(Model::LargeV3, &mut downloader, &mut transcriber);
        library.add(next, true, &mut transcriber);

        assert_eq!(
            transcriber.models,
            vec![Model::LargeV3Turbo, Model::LargeV3]
        );
    }

    #[test]
    fn re_transcribing_starts_with_the_current_model() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.transcription_finished(&recording, 1_000, &mut transcriber);
        library.choose(
            Model::LargeV3Turbo,
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        library.rerun(&recording, true, &mut transcriber);

        assert_eq!(
            transcriber.models,
            vec![Model::LargeV3, Model::LargeV3Turbo]
        );
    }

    #[test]
    fn choosing_a_second_missing_model_mid_download_downloads_that_one_and_keeps_the_first_weight()
    {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(&[], &mut downloader, &mut transcriber);

        library.choose(Model::LargeV3Turbo, &mut downloader, &mut transcriber);
        library.download_progressed(Model::LargeV3, 900, 3_000);

        assert_eq!(downloader.models, vec![Model::LargeV3, Model::LargeV3Turbo]);
        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3Turbo,
                received: 0,
                total: 0
            }
        );

        library.download_finished(Model::LargeV3, &mut transcriber);

        assert!(!library.is_ready());
        library.choose(Model::LargeV3, &mut downloader, &mut transcriber);
        assert!(library.is_ready());
        assert_eq!(downloader.starts, 2);
    }

    #[test]
    fn a_stored_weight_is_ready_without_downloading_again() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();

        library.prepare(
            &[Model::LargeV3],
            &mut downloader,
            &mut FakeTranscriber::default(),
        );

        assert_eq!(downloader.starts, 0);
        assert!(library.is_ready());
        assert_eq!(library.model(), Some(Model::LargeV3));
    }

    #[test]
    fn download_progress_shows_how_much_of_the_weight_has_arrived() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());

        library.download_progressed(Model::LargeV3, 1_800_000_000, 3_095_033_483);

        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3,
                received: 1_800_000_000,
                total: 3_095_033_483
            }
        );
    }

    #[test]
    fn a_failed_download_stays_failed_and_does_not_retry_by_itself() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());
        library.download_progressed(Model::LargeV3, 500, 3_000);

        library.download_failed(Model::LargeV3, "网络中断".into());
        library.download_progressed(Model::LargeV3, 900, 3_000);

        assert_eq!(
            library.download(),
            &Download::Failed {
                model: Model::LargeV3,
                error: "网络中断".into()
            }
        );
        assert_eq!(downloader.starts, 1);
    }

    #[test]
    fn retry_starts_the_download_again_from_nothing() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());
        library.download_progressed(Model::LargeV3, 500, 3_000);
        library.download_failed(Model::LargeV3, "网络中断".into());

        library.retry_download(&mut downloader);

        assert_eq!(downloader.starts, 2);
        assert_eq!(downloader.models, vec![Model::LargeV3, Model::LargeV3]);
        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3,
                received: 0,
                total: 0
            }
        );
    }

    #[test]
    fn a_finished_download_makes_the_weight_ready() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());
        library.download_progressed(Model::LargeV3, 3_000, 3_000);

        library.download_finished(Model::LargeV3, &mut FakeTranscriber::default());

        assert!(library.is_ready());
        assert_eq!(library.model(), Some(Model::LargeV3));
    }

    #[test]
    fn recordings_added_while_the_weight_downloads_or_failed_stay_queued() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());
        library.add(
            path("/Users/me/录音/一.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );
        library.download_progressed(Model::LargeV3, 1_000, 3_000);
        library.download_failed(Model::LargeV3, "网络中断".into());
        library.add(
            path("/Users/me/录音/二.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );

        let statuses: Vec<_> = library
            .recordings()
            .iter()
            .map(|recording| recording.status.clone())
            .collect();
        assert_eq!(statuses, vec![Status::Queued, Status::Queued]);
    }

    #[test]
    fn retry_does_nothing_while_the_download_is_running() {
        let mut library = Library::new();
        let mut downloader = FakeDownloader::default();
        library.prepare(&[], &mut downloader, &mut FakeTranscriber::default());
        library.download_progressed(Model::LargeV3, 1_000, 3_000);

        library.retry_download(&mut downloader);

        assert_eq!(downloader.starts, 1);
        assert_eq!(
            library.download(),
            &Download::Running {
                model: Model::LargeV3,
                received: 1_000,
                total: 3_000
            }
        );
    }

    #[derive(Default)]
    struct FakeTranscriber {
        started: Vec<(PathBuf, Language)>,
        models: Vec<Model>,
        cancels: usize,
    }

    impl Transcriber for FakeTranscriber {
        fn start(&mut self, path: &Path, language: Language, model: Model) {
            self.started.push((path.to_path_buf(), language));
            self.models.push(model);
        }

        fn cancel(&mut self) {
            self.cancels += 1;
        }
    }

    fn ready_library() -> (Library, FakeTranscriber) {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );
        (library, transcriber)
    }

    #[test]
    fn with_the_weight_ready_an_added_recording_starts_transcribing_in_auto() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");

        library.add(recording.clone(), true, &mut transcriber);

        assert_eq!(library.recordings()[0].status, Status::Transcribing);
        assert_eq!(library.recordings()[0].status.label(), "转写中");
        assert_eq!(transcriber.started, vec![(recording, Language::Auto)]);
        assert_eq!(transcriber.models, vec![Model::LargeV3]);
    }

    #[test]
    fn decoded_duration_is_kept_after_completion_and_restart() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.transcription_progressed(&recording, 738_000, 1_085_000);
        library.transcription_finished(&recording, 60_000, &mut transcriber);
        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");
        let recording = restored.recording(&recording).expect("Recording");
        assert_eq!(recording.duration_ms(), Some(1_085_000));
        assert_eq!(recording.progress(), None);
        assert_eq!(recording.status, Status::Completed);
    }

    #[test]
    fn the_time_a_transcription_took_is_kept_and_shown_only_while_completed() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        assert_eq!(library.recordings()[0].transcription_ms(), None);
        library.transcription_finished(&recording, 908_000, &mut transcriber);
        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");
        assert_eq!(restored.recordings()[0].transcription_ms(), Some(908_000));

        library.rerun(&recording, true, &mut transcriber);
        library.transcription_failed(&recording, "解码失败".into(), &mut transcriber);
        assert_eq!(library.recordings()[0].transcription_ms(), None);
    }

    #[test]
    fn a_stored_library_from_before_transcription_times_still_loads() {
        let json = r#"{"recordings":[{"path":"/Users/me/录音/周会.m4a","status":"Completed"}],"selected":null}"#;
        let library = Library::from_json(json).expect("old snapshot");
        assert_eq!(library.recordings()[0].transcription_ms(), None);
    }

    #[test]
    fn remaining_time_waits_for_a_steady_pace_then_follows_it() {
        let progress = Progress {
            processed_ms: 1_100_000,
            duration_ms: 4_000_000,
        };
        assert_eq!(progress.speed(20_000), None);
        assert_eq!(progress.remaining_ms(20_000), None);
        assert_eq!(progress.speed(250_000), Some(4.4));
        assert_eq!(progress.remaining_ms(250_000), Some(659_091));
        let barely_started = Progress {
            processed_ms: 60_000,
            duration_ms: 4_000_000,
        };
        assert_eq!(barely_started.remaining_ms(60_000), None);
        let done = Progress {
            processed_ms: 4_000_000,
            duration_ms: 4_000_000,
        };
        assert_eq!(done.remaining_ms(900_000), Some(0));
    }

    fn statuses(library: &Library) -> Vec<Status> {
        library
            .recordings()
            .iter()
            .map(|recording| recording.status.clone())
            .collect()
    }

    #[test]
    fn only_one_recording_transcribes_at_a_time() {
        let (mut library, mut transcriber) = ready_library();

        library.add(path("/Users/me/录音/一.m4a"), true, &mut transcriber);
        library.add(path("/Users/me/录音/二.m4a"), true, &mut transcriber);

        assert_eq!(
            statuses(&library),
            vec![Status::Transcribing, Status::Queued]
        );
        assert_eq!(transcriber.started.len(), 1);
    }

    #[test]
    fn the_earliest_recording_queued_during_the_download_starts_when_it_finishes() {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(&[], &mut FakeDownloader::default(), &mut transcriber);
        let first = path("/Users/me/录音/一.m4a");
        library.add(first.clone(), true, &mut transcriber);
        library.add(path("/Users/me/录音/二.m4a"), true, &mut transcriber);
        assert!(transcriber.started.is_empty());

        library.download_finished(Model::LargeV3, &mut transcriber);

        assert_eq!(
            statuses(&library),
            vec![Status::Transcribing, Status::Queued]
        );
        assert_eq!(transcriber.started, vec![(first, Language::Auto)]);
    }

    #[test]
    fn a_restored_queue_starts_once_the_stored_weight_is_found() {
        let mut queued = Library::new();
        let first = path("/Users/me/录音/一.m4a");
        queued.add(first.clone(), true, &mut FakeTranscriber::default());
        queued.add(
            path("/Users/me/录音/二.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );
        let mut library =
            Library::from_json(&queued.to_json().expect("snapshot")).expect("snapshot");
        let mut transcriber = FakeTranscriber::default();

        library.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        assert_eq!(transcriber.started, vec![(first, Language::Auto)]);
    }

    fn segment(start_ms: u64, end_ms: u64, text: &str) -> Segment {
        Segment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    fn visible_texts(library: &Library, index: usize) -> Vec<String> {
        library.recordings()[index]
            .visible_segments()
            .iter()
            .map(|segment| segment.text.clone())
            .collect()
    }

    #[test]
    fn live_segments_show_simplified_chinese_and_leave_english_alone() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/产品评审.m4a");
        library.add(recording.clone(), true, &mut transcriber);

        library.segment_transcribed(&recording, segment(0, 4_000, "今天討論 Visper 的範圍。"));
        library.segment_transcribed(
            &recording,
            segment(4_000, 9_000, "We should keep the Vulkan path."),
        );
        library.segment_transcribed(&recording, segment(9_000, 12_000, "API 這個詞保持英文。"));

        assert_eq!(
            visible_texts(&library, 0),
            vec![
                "今天讨论 Visper 的范围。",
                "We should keep the Vulkan path.",
                "API 这个词保持英文。",
            ]
        );
        assert_eq!(
            library.recordings()[0].visible_segments()[1].start_ms,
            4_000
        );
    }

    #[test]
    fn a_segment_shows_its_start_as_hours_minutes_seconds() {
        assert_eq!(segment(62_400, 70_000, "今天").start_label(), "00:01:02");
        assert_eq!(
            segment(3_723_000, 3_725_000, "later").start_label(),
            "01:02:03"
        );
    }

    #[test]
    fn transcribing_shows_processed_time_over_duration() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);

        library.transcription_progressed(&recording, 1_200_000, 2_400_000);

        let progress = library.recordings()[0].progress().expect("progress");
        assert_eq!(progress.label(), "00:20:00 / 00:40:00 · 50%");
        assert_eq!(progress.fraction(), 0.5);
    }

    #[test]
    fn a_successful_transcription_stores_the_segments_and_starts_the_next() {
        let (mut library, mut transcriber) = ready_library();
        let first = path("/Users/me/录音/一.m4a");
        let second = path("/Users/me/录音/二.m4a");
        library.add(first.clone(), true, &mut transcriber);
        library.add(second.clone(), true, &mut transcriber);
        library.segment_transcribed(&first, segment(0, 3_000, "導出只寫 Markdown。"));

        library.transcription_finished(&first, 60_000, &mut transcriber);

        let done = &library.recordings()[0];
        assert_eq!(done.status, Status::Completed);
        assert_eq!(done.status.label(), "完成");
        assert_eq!(
            done.stored_segments(),
            &[segment(0, 3_000, "导出只写 Markdown。")]
        );
        assert_eq!(visible_texts(&library, 0), vec!["导出只写 Markdown。"]);
        assert_eq!(library.recordings()[1].status, Status::Transcribing);
        assert_eq!(transcriber.started.last(), Some(&(second, Language::Auto)));
    }

    #[test]
    fn a_recording_without_words_completes_with_an_empty_transcript() {
        let (mut library, mut transcriber) = ready_library();
        let silent = path("/Users/me/录音/静音.wav");
        library.add(silent.clone(), true, &mut transcriber);

        library.transcription_finished(&silent, 60_000, &mut transcriber);

        assert_eq!(library.recordings()[0].status, Status::Completed);
        assert!(library.recordings()[0].stored_segments().is_empty());
    }

    #[test]
    fn a_completed_transcript_survives_a_restart() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.segment_transcribed(&recording, segment(1_000, 2_000, "第一句。"));
        library.transcription_finished(&recording, 60_000, &mut transcriber);

        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");

        assert_eq!(restored.recordings()[0].status, Status::Completed);
        assert_eq!(visible_texts(&restored, 0), vec!["第一句。"]);
    }

    #[test]
    fn a_transcription_cut_off_by_a_crash_is_queued_again_and_starts_first() {
        let (mut library, mut transcriber) = ready_library();
        let interrupted = path("/Users/me/录音/一.m4a");
        library.add(interrupted.clone(), true, &mut transcriber);
        library.add(path("/Users/me/录音/二.m4a"), true, &mut transcriber);
        let mut restored =
            Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");
        let mut transcriber = FakeTranscriber::default();

        restored.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        assert_eq!(
            statuses(&restored),
            vec![Status::Transcribing, Status::Queued]
        );
        assert_eq!(transcriber.started, vec![(interrupted, Language::Auto)]);
    }

    #[test]
    fn a_language_chosen_before_the_start_is_the_one_the_transcription_uses() {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(&[], &mut FakeDownloader::default(), &mut transcriber);
        let recording = path("/Users/me/录音/路演彩排.flac");
        library.add(recording.clone(), true, &mut transcriber);

        library.set_language(&recording, Language::Chinese);
        library.download_finished(Model::LargeV3, &mut transcriber);

        assert_eq!(library.recordings()[0].language, Language::Chinese);
        assert_eq!(transcriber.started, vec![(recording, Language::Chinese)]);
    }

    #[test]
    fn the_language_is_locked_once_the_transcription_has_started() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);

        library.set_language(&recording, Language::English);

        assert_eq!(library.recordings()[0].language, Language::Auto);
    }

    #[test]
    fn cancel_drops_the_live_segments_keeps_the_recording_and_starts_the_next() {
        let (mut library, mut transcriber) = ready_library();
        let first = path("/Users/me/录音/产品评审.m4a");
        let second = path("/Users/me/录音/路演彩排.flac");
        library.add(first.clone(), true, &mut transcriber);
        library.add(second.clone(), true, &mut transcriber);
        library.segment_transcribed(&first, segment(0, 3_000, "今天讨论范围。"));

        library.cancel(&first, &mut transcriber);
        library.segment_transcribed(&first, segment(3_000, 6_000, "迟到的分段。"));

        let cancelled = &library.recordings()[0];
        assert_eq!(cancelled.status, Status::Cancelled);
        assert_eq!(cancelled.status.label(), "已取消");
        assert!(cancelled.visible_segments().is_empty());
        assert!(cancelled.stored_segments().is_empty());
        assert_eq!(transcriber.cancels, 1);
        assert_eq!(library.recordings()[1].status, Status::Transcribing);
        assert_eq!(transcriber.started.last(), Some(&(second, Language::Auto)));
    }

    #[test]
    fn a_failure_drops_the_live_segments_keeps_the_error_and_starts_the_next() {
        let (mut library, mut transcriber) = ready_library();
        let broken = path("/Users/me/录音/损坏片段.wav");
        let next = path("/Users/me/录音/英文访谈.mp3");
        library.add(broken.clone(), true, &mut transcriber);
        library.add(next.clone(), true, &mut transcriber);
        library.segment_transcribed(&broken, segment(0, 3_000, "半成品。"));

        library.transcription_failed(
            &broken,
            "音频在 00:06:12 处解码失败。".into(),
            &mut transcriber,
        );

        let failed = &library.recordings()[0];
        assert_eq!(failed.status, Status::Failed);
        assert_eq!(failed.status.label(), "失败");
        assert_eq!(failed.error(), Some("音频在 00:06:12 处解码失败。"));
        assert!(failed.visible_segments().is_empty());
        assert_eq!(transcriber.started.last(), Some(&(next, Language::Auto)));
    }

    fn rerun_with_a_stored_transcript(recording: &Path) -> (Library, FakeTranscriber) {
        let snapshot = format!(
            r#"{{"recordings":[{{"path":"{}","status":"Queued","segments":[{{"start_ms":4000,"end_ms":9000,"text":"上一版已经保存的稿。"}}]}}],"selected":null}}"#,
            recording.display()
        );
        let mut library = Library::from_json(&snapshot).expect("snapshot");
        let mut transcriber = FakeTranscriber::default();
        library.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );
        (library, transcriber)
    }

    #[test]
    fn a_failed_rerun_shows_the_stored_transcript_again() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (mut library, mut transcriber) = rerun_with_a_stored_transcript(&recording);
        library.segment_transcribed(&recording, segment(0, 2_000, "这次重跑的新分段。"));
        assert_eq!(visible_texts(&library, 0), vec!["这次重跑的新分段。"]);

        library.transcription_failed(&recording, "转写中断。".into(), &mut transcriber);

        assert_eq!(visible_texts(&library, 0), vec!["上一版已经保存的稿。"]);
    }

    #[test]
    fn a_cancelled_rerun_keeps_the_stored_transcript() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (mut library, mut transcriber) = rerun_with_a_stored_transcript(&recording);
        library.segment_transcribed(&recording, segment(0, 2_000, "这次重跑的新分段。"));

        library.cancel(&recording, &mut transcriber);

        assert_eq!(library.recordings()[0].status, Status::Cancelled);
        assert_eq!(visible_texts(&library, 0), vec!["上一版已经保存的稿。"]);
    }

    #[test]
    fn the_list_shows_transcribing_then_queued_in_add_order_then_newest_finished() {
        let (mut library, mut transcriber) = ready_library();
        let names = ["一.m4a", "二.m4a", "三.m4a", "四.m4a", "五.m4a"];
        for name in names {
            library.add(
                path(&format!("/Users/me/录音/{name}")),
                true,
                &mut transcriber,
            );
        }
        library.transcription_finished(&path("/Users/me/录音/一.m4a"), 60_000, &mut transcriber);
        library.cancel(&path("/Users/me/录音/二.m4a"), &mut transcriber);

        let groups: Vec<(&str, Vec<String>)> = library
            .groups()
            .into_iter()
            .map(|(group, recordings)| {
                (
                    group.label(),
                    recordings
                        .iter()
                        .map(|recording| recording.file_name())
                        .collect(),
                )
            })
            .collect();

        assert_eq!(
            groups,
            vec![
                ("转写中", vec!["三.m4a".to_string()]),
                ("等待中", vec!["四.m4a".to_string(), "五.m4a".to_string()]),
                ("已结束", vec!["二.m4a".to_string(), "一.m4a".to_string()]),
            ]
        );
    }

    #[test]
    fn empty_groups_are_left_out() {
        let (mut library, mut transcriber) = ready_library();
        library.add(path("/Users/me/录音/一.m4a"), true, &mut transcriber);

        let labels: Vec<_> = library
            .groups()
            .into_iter()
            .map(|(group, _)| group.label())
            .collect();

        assert_eq!(labels, vec!["转写中"]);
    }

    #[test]
    fn reading_another_recording_leaves_the_transcription_running() {
        let (mut library, mut transcriber) = ready_library();
        let first = path("/Users/me/录音/客户访谈.m4a");
        let second = path("/Users/me/录音/周会.m4a");
        library.add(first.clone(), true, &mut transcriber);
        library.add(second.clone(), true, &mut transcriber);
        library.select(&first);

        library.select(&second);

        assert_eq!(library.recordings()[0].status, Status::Transcribing);
        assert_eq!(transcriber.cancels, 0);
    }

    #[derive(Default)]
    struct FakeWriter {
        written: Vec<(PathBuf, String)>,
    }

    impl MarkdownWriter for FakeWriter {
        fn write(&mut self, file: &Path, body: &str) -> Result<(), String> {
            self.written.push((file.to_path_buf(), body.to_owned()));
            Ok(())
        }
    }

    fn completed(recording: &Path, segments: &[Segment]) -> Library {
        let (mut library, mut transcriber) = ready_library();
        library.add(recording.to_path_buf(), true, &mut transcriber);
        for segment in segments {
            library.segment_transcribed(recording, segment.clone());
        }
        library.transcription_finished(recording, 60_000, &mut transcriber);
        library
    }

    #[test]
    fn export_writes_the_stored_segments_as_markdown_named_after_the_recording() {
        let recording = path("/Users/me/录音/周会.m4a");
        let library = completed(
            &recording,
            &[
                segment(62_000, 70_000, "今天讨论 Rust 的 API。"),
                segment(75_000, 80_000, "We should keep the Vulkan path."),
            ],
        );
        let mut writer = FakeWriter::default();

        let outcome = library.export(&recording, &path("/Users/me/导出"), &mut writer);

        assert_eq!(
            outcome,
            ExportOutcome::Written(path("/Users/me/导出/周会.md"))
        );
        assert_eq!(
            writer.written,
            vec![(
                path("/Users/me/导出/周会.md"),
                "[00:01:02] 今天讨论 Rust 的 API。\n\n[00:01:15] We should keep the Vulkan path.\n"
                    .to_string()
            )]
        );
    }

    #[test]
    fn an_empty_completed_transcript_cannot_be_exported() {
        let recording = path("/Users/me/录音/静音.wav");
        let library = completed(&recording, &[]);
        let mut writer = FakeWriter::default();

        let outcome = library.export(&recording, &path("/Users/me/导出"), &mut writer);

        assert!(!library.recordings()[0].can_export());
        assert_eq!(outcome, ExportOutcome::NothingStored);
        assert!(writer.written.is_empty());
    }

    #[test]
    fn a_first_transcription_still_running_cannot_be_exported() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.segment_transcribed(&recording, segment(0, 2_000, "还没保存。"));
        let mut writer = FakeWriter::default();

        let outcome = library.export(&recording, &path("/Users/me/导出"), &mut writer);

        assert!(!library.recordings()[0].can_export());
        assert_eq!(outcome, ExportOutcome::NothingStored);
        assert!(writer.written.is_empty());
    }

    #[test]
    fn export_during_a_rerun_writes_the_stored_transcript_not_the_live_one() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (mut library, _) = rerun_with_a_stored_transcript(&recording);
        library.segment_transcribed(&recording, segment(0, 2_000, "这次重跑的新分段。"));
        let mut writer = FakeWriter::default();

        library.export(&recording, &path("/Users/me/导出"), &mut writer);

        assert!(library.recordings()[0].can_export());
        assert_eq!(writer.written[0].1, "[00:00:04] 上一版已经保存的稿。\n");
    }

    #[test]
    fn a_completed_recording_can_play_even_with_an_empty_transcript() {
        let silent = path("/Users/me/录音/静音.wav");
        let library = completed(&silent, &[]);

        assert!(library.recordings()[0].can_play());
    }

    #[test]
    fn a_queued_recording_without_a_transcript_cannot_play() {
        let mut library = Library::new();
        library.add(
            path("/Users/me/录音/周会.m4a"),
            true,
            &mut FakeTranscriber::default(),
        );

        assert!(!library.recordings()[0].can_play());
    }

    #[test]
    fn a_transcription_can_play_once_segments_are_visible() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/客户访谈.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        assert!(!library.recordings()[0].can_play());

        library.segment_transcribed(&recording, segment(0, 2_000, "第一句。"));

        assert!(library.recordings()[0].can_play());
    }

    #[test]
    fn a_queued_rerun_with_a_stored_transcript_can_play() {
        let snapshot = r#"{"recordings":[{"path":"/Users/me/录音/设计讨论.m4a","status":"Queued","segments":[{"start_ms":0,"end_ms":1000,"text":"旧稿。"}]}],"selected":null}"#;
        let library = Library::from_json(snapshot).expect("snapshot");

        assert!(library.recordings()[0].can_play());
    }

    #[test]
    fn the_playing_segment_is_the_one_the_playhead_is_inside() {
        let recording = path("/Users/me/录音/周会.m4a");
        let library = completed(
            &recording,
            &[
                segment(2_000, 5_000, "一。"),
                segment(5_000, 9_000, "二。"),
                segment(12_000, 15_000, "三。"),
            ],
        );
        let done = &library.recordings()[0];

        assert_eq!(done.segment_at(1_000), None);
        assert_eq!(done.segment_at(2_000), Some(0));
        assert_eq!(done.segment_at(5_000), Some(1));
        assert_eq!(done.segment_at(8_999), Some(1));
        assert_eq!(done.segment_at(10_000), None);
        assert_eq!(done.segment_at(14_000), Some(2));
        assert_eq!(done.segment_at(60_000), None);
    }

    #[test]
    fn a_rerun_queues_a_finished_recording_again_and_keeps_its_transcript_until_success() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let mut library = completed(&recording, &[segment(0, 2_000, "上一版。")]);
        let mut transcriber = FakeTranscriber::default();

        let outcome = library.rerun(&recording, true, &mut transcriber);

        let rerun = &library.recordings()[0];
        assert_eq!(outcome, RerunOutcome::Queued);
        assert_eq!(rerun.status, Status::Transcribing);
        assert_eq!(rerun.stored_segments(), &[segment(0, 2_000, "上一版。")]);
        assert!(rerun.visible_segments().is_empty());
        assert!(rerun.exports_previous_transcript());
        assert_eq!(
            transcriber.started,
            vec![(recording.clone(), Language::Auto)]
        );

        library.segment_transcribed(&recording, segment(0, 3_000, "新的一版。"));
        library.transcription_finished(&recording, 60_000, &mut transcriber);

        assert_eq!(
            library.recordings()[0].stored_segments(),
            &[segment(0, 3_000, "新的一版。")]
        );
    }

    #[test]
    fn a_queued_or_transcribing_recording_cannot_be_queued_twice() {
        let (mut library, mut transcriber) = ready_library();
        let running = path("/Users/me/录音/一.m4a");
        let waiting = path("/Users/me/录音/二.m4a");
        library.add(running.clone(), true, &mut transcriber);
        library.add(waiting.clone(), true, &mut transcriber);

        assert_eq!(
            library.rerun(&running, true, &mut transcriber),
            RerunOutcome::NotFinished
        );
        assert_eq!(
            library.rerun(&waiting, true, &mut transcriber),
            RerunOutcome::NotFinished
        );
        assert_eq!(library.recordings().len(), 2);
        assert_eq!(transcriber.started.len(), 1);
    }

    #[test]
    fn a_recording_whose_file_no_longer_opens_cannot_run_again() {
        let recording = path("/Users/me/录音/归档会议.m4a");
        let mut library = completed(&recording, &[segment(0, 1_000, "旧稿。")]);

        let outcome = library.rerun(&recording, false, &mut FakeTranscriber::default());

        assert_eq!(outcome, RerunOutcome::CannotOpen);
        assert_eq!(library.recordings()[0].status, Status::Completed);
    }

    #[test]
    fn a_rerun_waits_behind_recordings_that_were_already_queued() {
        let (mut library, mut transcriber) = ready_library();
        let done = path("/Users/me/录音/一.m4a");
        let running = path("/Users/me/录音/二.m4a");
        let waiting = path("/Users/me/录音/三.m4a");
        for recording in [&done, &running, &waiting] {
            library.add(recording.clone(), true, &mut transcriber);
        }
        library.transcription_finished(&done, 60_000, &mut transcriber);

        library.rerun(&done, true, &mut transcriber);
        library.transcription_finished(&running, 60_000, &mut transcriber);

        assert_eq!(transcriber.started.last(), Some(&(waiting, Language::Auto)));
        let queued: Vec<_> = library.groups()[1]
            .1
            .iter()
            .map(|r| r.file_name())
            .collect();
        assert_eq!(queued, vec!["一.m4a"]);
    }

    #[test]
    fn adding_a_finished_path_again_queues_it_and_keeps_the_old_transcript() {
        let recording = path("/Users/me/录音/周会.m4a");
        let mut library = completed(&recording, &[segment(0, 1_000, "旧稿。")]);
        let mut transcriber = FakeTranscriber::default();

        let outcome = library.add(recording.clone(), true, &mut transcriber);

        assert_eq!(outcome, AddOutcome::Queued);
        assert_eq!(library.recordings().len(), 1);
        assert_eq!(library.recordings()[0].status, Status::Transcribing);
        assert_eq!(
            library.recordings()[0].stored_segments(),
            &[segment(0, 1_000, "旧稿。")]
        );
    }

    #[test]
    fn adding_a_transcribing_path_again_leaves_it_alone() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/周会.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.segment_transcribed(&recording, segment(0, 1_000, "进行中。"));

        let outcome = library.add(recording.clone(), true, &mut transcriber);

        assert_eq!(outcome, AddOutcome::AlreadyTranscribing);
        assert_eq!(visible_texts(&library, 0), vec!["进行中。"]);
        assert_eq!(transcriber.started.len(), 1);
    }

    #[test]
    fn removal_asks_first_only_when_there_is_a_stored_transcript() {
        let empty_completed = completed(&path("/Users/me/录音/静音.wav"), &[]);
        assert!(empty_completed.recordings()[0].needs_removal_confirmation());

        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (mut rerun, mut transcriber) = rerun_with_a_stored_transcript(&recording);
        rerun.transcription_failed(&recording, "转写中断。".into(), &mut transcriber);
        assert!(rerun.recordings()[0].needs_removal_confirmation());

        let (mut fresh, mut transcriber) = ready_library();
        let failed = path("/Users/me/录音/损坏片段.wav");
        let cancelled = path("/Users/me/录音/临时备忘.m4a");
        let queued = path("/Users/me/录音/路演彩排.flac");
        for recording in [&failed, &cancelled, &queued] {
            fresh.add(recording.clone(), true, &mut transcriber);
        }
        fresh.transcription_failed(&failed, "解码失败。".into(), &mut transcriber);
        fresh.cancel(&cancelled, &mut transcriber);
        assert!(
            fresh
                .recordings()
                .iter()
                .all(|r| !r.needs_removal_confirmation())
        );
    }

    #[test]
    fn removing_a_queued_recording_takes_it_out_of_the_library() {
        let mut library = Library::new();
        let recording = path("/Users/me/录音/周会.m4a");
        let mut transcriber = FakeTranscriber::default();
        library.add(recording.clone(), true, &mut transcriber);

        library.remove(&recording, &mut transcriber);

        assert!(library.is_empty());
        assert!(library.selected().is_none());
        assert_eq!(transcriber.cancels, 0);
    }

    #[test]
    fn removing_the_transcribing_recording_cancels_it_first_and_the_queue_moves_on() {
        let (mut library, mut transcriber) = ready_library();
        let running = path("/Users/me/录音/客户访谈.m4a");
        let next = path("/Users/me/录音/路演彩排.flac");
        library.add(running.clone(), true, &mut transcriber);
        library.add(next.clone(), true, &mut transcriber);
        library.segment_transcribed(&running, segment(0, 1_000, "半成品。"));

        library.remove(&running, &mut transcriber);

        assert_eq!(transcriber.cancels, 1);
        assert_eq!(library.recordings().len(), 1);
        assert_eq!(library.recordings()[0].path, next);
        assert_eq!(library.recordings()[0].status, Status::Transcribing);
    }

    #[test]
    fn a_path_that_no_longer_opens_is_missing_but_keeps_its_status_and_transcript() {
        let recording = path("/Users/me/录音/归档会议.m4a");
        let mut library = completed(&recording, &[segment(0, 2_000, "档案里的旧会议。")]);
        let mut transcriber = FakeTranscriber::default();

        library.file_checked(&recording, false, &mut FakeTranscriber::default());

        let missing = &library.recordings()[0];
        assert!(missing.is_missing());
        assert_eq!(missing.status, Status::Completed);
        assert_eq!(visible_texts(&library, 0), vec!["档案里的旧会议。"]);
        assert!(!missing.can_play());
        assert!(!missing.can_rerun());
        assert!(missing.can_export());
        assert_eq!(
            library.rerun(&recording, false, &mut transcriber),
            RerunOutcome::CannotOpen
        );
        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");
        assert!(restored.recordings()[0].is_missing());
    }

    #[test]
    fn a_missing_path_that_opens_again_is_no_longer_missing() {
        let recording = path("/Users/me/录音/归档会议.m4a");
        let mut library = completed(&recording, &[segment(0, 2_000, "旧稿。")]);
        library.file_checked(&recording, false, &mut FakeTranscriber::default());

        library.file_checked(&recording, true, &mut FakeTranscriber::default());

        assert!(!library.recordings()[0].is_missing());
        assert!(library.recordings()[0].can_play());
    }

    #[test]
    fn a_missing_queued_recording_waits_and_starts_once_its_file_opens_again() {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        let moved = path("/Users/me/录音/移走了.m4a");
        let present = path("/Users/me/录音/还在.m4a");
        library.add(moved.clone(), true, &mut transcriber);
        library.add(present.clone(), true, &mut transcriber);
        library.file_checked(&moved, false, &mut transcriber);

        library.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );
        assert_eq!(library.recordings()[0].status, Status::Queued);
        assert_eq!(transcriber.started, vec![(present.clone(), Language::Auto)]);
        library.transcription_finished(&present, 60_000, &mut transcriber);
        assert_eq!(library.recordings()[0].status, Status::Queued);

        library.file_checked(&moved, true, &mut transcriber);

        assert_eq!(library.recordings()[0].status, Status::Transcribing);
        assert_eq!(transcriber.started.last(), Some(&(moved, Language::Auto)));
    }

    #[test]
    fn an_empty_stored_transcript_still_counts_after_its_rerun_fails() {
        let silent = path("/Users/me/录音/静音.wav");
        let mut library = completed(&silent, &[]);
        let mut transcriber = FakeTranscriber::default();
        library.rerun(&silent, true, &mut transcriber);
        assert!(library.recordings()[0].needs_removal_confirmation());

        library.transcription_failed(&silent, "转写中断。".into(), &mut transcriber);

        let failed = &library.recordings()[0];
        assert!(failed.has_stored_transcript());
        assert!(failed.needs_removal_confirmation());
        assert!(failed.can_play());
        assert!(!failed.can_export());
    }

    #[test]
    fn removing_a_rerun_in_progress_asks_first_when_a_transcript_is_stored() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (library, _) = rerun_with_a_stored_transcript(&recording);

        assert_eq!(library.recordings()[0].status, Status::Transcribing);
        assert!(library.recordings()[0].needs_removal_confirmation());
    }

    #[test]
    fn quit_cancels_the_transcription_keeps_the_queue_and_starts_nothing_new() {
        let (mut library, mut transcriber) = ready_library();
        let running = path("/Users/me/录音/客户访谈.m4a");
        let waiting = path("/Users/me/录音/路演彩排.flac");
        library.add(running.clone(), true, &mut transcriber);
        library.add(waiting.clone(), true, &mut transcriber);
        library.segment_transcribed(&running, segment(0, 1_000, "半成品。"));

        library.quit(&mut transcriber);

        assert_eq!(statuses(&library), vec![Status::Cancelled, Status::Queued]);
        assert!(library.recordings()[0].visible_segments().is_empty());
        assert_eq!(transcriber.cancels, 1);
        assert_eq!(transcriber.started, vec![(running, Language::Auto)]);
    }

    #[test]
    fn after_quit_the_queue_starts_again_on_the_next_launch_in_add_order() {
        let (mut library, mut transcriber) = ready_library();
        let running = path("/Users/me/录音/一.m4a");
        let second = path("/Users/me/录音/二.m4a");
        let waiting = path("/Users/me/录音/三.m4a");
        for recording in [&running, &second, &waiting] {
            library.add(recording.clone(), true, &mut transcriber);
        }
        library.quit(&mut transcriber);
        let mut next_launch =
            Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");
        let mut transcriber = FakeTranscriber::default();

        next_launch.prepare(
            &[Model::LargeV3],
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        assert_eq!(
            statuses(&next_launch),
            vec![Status::Cancelled, Status::Transcribing, Status::Queued]
        );
        assert_eq!(transcriber.started, vec![(second, Language::Auto)]);
    }

    #[test]
    fn quit_keeps_a_stored_transcript_of_a_rerun() {
        let recording = path("/Users/me/录音/设计讨论.m4a");
        let (mut library, mut transcriber) = rerun_with_a_stored_transcript(&recording);
        library.segment_transcribed(&recording, segment(0, 2_000, "这次重跑的新分段。"));

        library.quit(&mut transcriber);

        assert_eq!(library.recordings()[0].status, Status::Cancelled);
        assert_eq!(visible_texts(&library, 0), vec!["上一版已经保存的稿。"]);
    }

    #[test]
    fn nothing_starts_after_quit_even_when_late_events_arrive() {
        let mut library = Library::new();
        let mut transcriber = FakeTranscriber::default();
        library.prepare(&[], &mut FakeDownloader::default(), &mut transcriber);
        let moved = path("/Users/me/录音/移走了.m4a");
        library.add(moved.clone(), true, &mut transcriber);
        library.add(path("/Users/me/录音/二.m4a"), true, &mut transcriber);
        library.file_checked(&moved, false, &mut transcriber);

        library.quit(&mut transcriber);
        library.download_finished(Model::LargeV3, &mut transcriber);
        library.file_checked(&moved, true, &mut transcriber);

        assert!(transcriber.started.is_empty());
        assert_eq!(statuses(&library), vec![Status::Queued, Status::Queued]);
    }

    fn choose_turbo(library: &mut Library, transcriber: &mut FakeTranscriber) {
        library.choose(
            Model::LargeV3Turbo,
            &mut FakeDownloader::default(),
            transcriber,
        );
    }

    #[test]
    fn a_transcription_that_succeeds_with_turbo_records_turbo() {
        let (mut library, mut transcriber) = library_with_both_weights();
        choose_turbo(&mut library, &mut transcriber);
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            None
        );

        library.transcription_finished(&recording, 1_000, &mut transcriber);

        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            Some(Model::LargeV3Turbo)
        );
    }

    #[test]
    fn the_model_recorded_is_the_one_the_transcription_started_with() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        choose_turbo(&mut library, &mut transcriber);

        library.transcription_finished(&recording, 1_000, &mut transcriber);

        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            Some(Model::LargeV3)
        );
    }

    #[test]
    fn a_failure_or_cancel_after_a_turbo_success_keeps_the_earlier_model() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let failing = path("/Users/me/录音/一.m4a");
        let cancelled = path("/Users/me/录音/二.m4a");
        choose_turbo(&mut library, &mut transcriber);
        library.add(failing.clone(), true, &mut transcriber);
        library.transcription_finished(&failing, 1_000, &mut transcriber);
        library.add(cancelled.clone(), true, &mut transcriber);
        library.transcription_finished(&cancelled, 1_000, &mut transcriber);
        library.choose(
            Model::LargeV3,
            &mut FakeDownloader::default(),
            &mut transcriber,
        );

        library.rerun(&failing, true, &mut transcriber);
        library.transcription_failed(&failing, "坏了".into(), &mut transcriber);
        library.rerun(&cancelled, true, &mut transcriber);
        library.cancel(&cancelled, &mut transcriber);

        for path in [&failing, &cancelled] {
            let recording = library.recording(path).unwrap();
            assert_ne!(recording.status, Status::Completed);
            assert_eq!(recording.transcript_model(), Some(Model::LargeV3Turbo));
        }
    }

    #[test]
    fn a_failure_without_an_earlier_transcript_records_no_model() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);

        library.transcription_failed(&recording, "坏了".into(), &mut transcriber);

        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            None
        );
    }

    #[test]
    fn re_transcribing_a_large_v3_transcript_while_turbo_is_current_records_turbo() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        library.transcription_finished(&recording, 1_000, &mut transcriber);
        choose_turbo(&mut library, &mut transcriber);

        library.rerun(&recording, true, &mut transcriber);
        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            Some(Model::LargeV3)
        );
        library.transcription_finished(&recording, 1_000, &mut transcriber);

        assert_eq!(
            library.recording(&recording).unwrap().transcript_model(),
            Some(Model::LargeV3Turbo)
        );
    }

    #[test]
    fn the_transcript_model_survives_a_restart() {
        let (mut library, mut transcriber) = library_with_both_weights();
        let recording = path("/Users/me/录音/一.m4a");
        choose_turbo(&mut library, &mut transcriber);
        library.add(recording.clone(), true, &mut transcriber);
        library.transcription_finished(&recording, 1_000, &mut transcriber);

        let restored = Library::from_json(&library.to_json().expect("snapshot")).expect("snapshot");

        assert_eq!(
            restored.recording(&recording).unwrap().transcript_model(),
            Some(Model::LargeV3Turbo)
        );
    }

    #[test]
    fn a_stored_transcript_without_a_recorded_model_reads_as_large_v3() {
        let json = r#"{"recordings":[
            {"path":"/a/完成.m4a","status":"Completed","segments":[{"start_ms":0,"end_ms":1,"text":"好"}]},
            {"path":"/a/失败.m4a","status":"Failed","transcript_stored":true},
            {"path":"/a/没有.m4a","status":"Failed"},
            {"path":"/a/等待.m4a","status":"Queued"}
        ],"selected":null}"#;

        let library = Library::from_json(json).expect("old snapshot");

        let models: Vec<_> = library
            .recordings()
            .iter()
            .map(Recording::transcript_model)
            .collect();
        assert_eq!(
            models,
            vec![Some(Model::LargeV3), Some(Model::LargeV3), None, None]
        );
    }

    #[test]
    fn the_model_is_shown_only_for_a_finished_recording_with_a_stored_transcript() {
        let (mut library, mut transcriber) = ready_library();
        let recording = path("/Users/me/录音/一.m4a");
        library.add(recording.clone(), true, &mut transcriber);
        let shown = |library: &Library| library.recording(&recording).unwrap().shown_model();
        assert_eq!(shown(&library), None);

        library.transcription_finished(&recording, 1_000, &mut transcriber);
        assert_eq!(shown(&library), Some(Model::LargeV3));

        library.rerun(&recording, true, &mut transcriber);
        assert_eq!(shown(&library), None);

        library.cancel(&recording, &mut transcriber);
        assert_eq!(shown(&library), Some(Model::LargeV3));
    }
}
