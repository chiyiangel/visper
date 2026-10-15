use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

/// Normal-speed playback of one Recording at a time. Position and play/pause
/// are view state; the Library only says whether a Recording can play.
#[derive(Default)]
pub struct Player {
    output: Option<OutputStream>,
    loaded: Option<Loaded>,
}

struct Loaded {
    path: PathBuf,
    sink: Sink,
    duration: Option<Duration>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackState {
    pub playing: bool,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
}

impl Player {
    /// The Recording whose audio is loaded, playing or paused.
    pub fn loaded_path(&self) -> Option<&Path> {
        self.started().map(|loaded| loaded.path.as_path())
    }

    /// Playback of `path`, unless it has not started or has run to the end.
    pub fn state(&self, path: &Path) -> Option<PlaybackState> {
        let loaded = self.started().filter(|loaded| loaded.path == path)?;
        Some(PlaybackState {
            playing: !loaded.sink.is_paused(),
            position_ms: loaded.sink.get_pos().as_millis() as u64,
            duration_ms: loaded.duration.map(|duration| duration.as_millis() as u64),
        })
    }

    /// Pauses or resumes; after the end, or for another Recording, it plays
    /// from the start.
    pub fn toggle(&mut self, path: &Path) -> Result<(), String> {
        match self.started().filter(|loaded| loaded.path == path) {
            Some(loaded) if loaded.sink.is_paused() => loaded.sink.play(),
            Some(loaded) => loaded.sink.pause(),
            None => self.load(path)?.sink.play(),
        }
        Ok(())
    }

    /// Plays `path` from `start_ms`, as a click on a Segment does.
    pub fn play_from(&mut self, path: &Path, start_ms: u64) -> Result<(), String> {
        let loaded = match self.loaded.take().filter(|loaded| loaded.path == path) {
            Some(loaded) if !loaded.sink.empty() => self.loaded.insert(loaded),
            _ => self.load(path)?,
        };
        loaded
            .sink
            .try_seek(Duration::from_millis(start_ms))
            .map_err(|error| format!("无法跳到这一段：{error}"))?;
        loaded.sink.play();
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(loaded) = self.loaded.take() {
            loaded.sink.stop();
        }
    }

    fn started(&self) -> Option<&Loaded> {
        self.loaded.as_ref().filter(|loaded| !loaded.sink.empty())
    }

    fn load(&mut self, path: &Path) -> Result<&Loaded, String> {
        self.stop();
        if self.output.is_none() {
            let mut output = OutputStreamBuilder::open_default_stream()
                .map_err(|error| format!("没有可用的音频输出：{error}"))?;
            output.log_on_drop(false);
            self.output = Some(output);
        }
        let Some(output) = self.output.as_ref() else {
            return Err("没有可用的音频输出。".into());
        };
        let file = File::open(path).map_err(|error| format!("打不开录音：{error}"))?;
        let decoder = Decoder::try_from(file).map_err(|error| format!("无法播放录音：{error}"))?;
        let duration = decoder.total_duration();
        let sink = Sink::connect_new(output.mixer());
        sink.pause();
        sink.append(decoder);
        Ok(self.loaded.insert(Loaded {
            path: path.to_path_buf(),
            sink,
            duration,
        }))
    }
}
