use std::ffi::c_void;
use std::fs::File;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use rubato::{FftFixedInOut, Resampler};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use visper::{Language, Segment};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

const SAMPLE_RATE: usize = 16_000;
/// Loudness is measured in 20 ms frames.
const FRAME: usize = SAMPLE_RATE / 50;
/// One Whisper window; nothing longer is decoded in one go.
const WINDOW_FRAMES: usize = 30 * 50;
/// A pause this long separates two stretches of speech.
const PAUSE_FRAMES: usize = 20;
/// Where a stretch runs past one window, it is cut at a breath this long.
const BREATH_FRAMES: usize = 5;
/// A stretch with less voiced time than this is treated as silence.
const VOICED_FRAMES: usize = 15;
/// Whisper drops the first words of speech that starts right at the edge of
/// its window, so each window is fed after this much silence.
const LEAD_IN_SAMPLES: usize = SAMPLE_RATE / 2;

pub enum TranscriptionEvent {
    Segment {
        path: PathBuf,
        segment: Segment,
    },
    Progressed {
        path: PathBuf,
        processed_ms: u64,
        duration_ms: u64,
    },
    Finished {
        path: PathBuf,
    },
    Failed {
        path: PathBuf,
        error: String,
    },
}

struct Request {
    path: PathBuf,
    language: Language,
    cancel: Arc<Cancel>,
}

/// Cancellation of one Request. Events go out under `gate`, and `raise`
/// takes the same lock, so once `cancel` returns nothing more from that
/// Request reaches the window; a rerun of the same Recording cannot be fed
/// a stale Segment or Finished.
#[derive(Default)]
struct Cancel {
    raised: AtomicBool,
    gate: Mutex<()>,
}

impl Cancel {
    fn raise(&self) {
        let _gate = self.gate.lock();
        self.raised.store(true, Ordering::SeqCst);
    }

    fn is_raised(&self) -> bool {
        self.raised.load(Ordering::SeqCst)
    }

    fn send(&self, events: &Sender<TranscriptionEvent>, event: TranscriptionEvent) {
        let _gate = self.gate.lock();
        if !self.is_raised() {
            let _ = events.send(event);
        }
    }
}

/// Owns one worker thread that keeps the large-v3 weight loaded between
/// Recordings and runs one Transcription at a time.
pub struct WhisperTranscriber {
    requests: Sender<Request>,
    /// Cancels the Transcription that was started last.
    cancel: Arc<Cancel>,
}

impl WhisperTranscriber {
    pub fn new(weight: PathBuf, events: Sender<TranscriptionEvent>) -> Self {
        let (requests, pending) = channel();
        std::thread::spawn(move || work(weight, pending, events));
        Self {
            requests,
            cancel: Arc::default(),
        }
    }
}

impl visper::Transcriber for WhisperTranscriber {
    fn start(&mut self, path: &Path, language: Language) {
        self.cancel = Arc::default();
        let _ = self.requests.send(Request {
            path: path.to_path_buf(),
            language,
            cancel: self.cancel.clone(),
        });
    }

    fn cancel(&mut self) {
        self.cancel.raise();
    }
}

fn work(weight: PathBuf, pending: Receiver<Request>, events: Sender<TranscriptionEvent>) {
    whisper_rs::install_logging_hooks();
    let mut gpu = None;
    for request in pending {
        let outcome = transcribe(&weight, &mut gpu, &request, &events);
        let path = request.path.clone();
        request.cancel.send(
            &events,
            match outcome {
                Ok(()) => TranscriptionEvent::Finished { path },
                Err(error) => TranscriptionEvent::Failed { path, error },
            },
        );
    }
}

fn transcribe(
    weight: &Path,
    gpu: &mut Option<WhisperContext>,
    request: &Request,
    events: &Sender<TranscriptionEvent>,
) -> Result<(), String> {
    let samples = decode(&request.path, &request.cancel)?;
    let mut pass = Pass {
        state: whisper_state(weight, gpu)?,
        threads: std::thread::available_parallelism()
            .map(|count| count.get().min(8))
            .unwrap_or(4),
        path: &request.path,
        events,
        duration_ms: ms(samples.len()),
        cancel: &request.cancel,
    };
    pass.progressed(0);

    // Each stretch gets its own language; neighbours in the same language
    // share a window so a long Recording does not pay one pass per sentence.
    let mut open: Option<(Range<usize>, &'static str)> = None;
    for stretch in stretches(&samples) {
        if pass.cancel.is_raised() {
            return Err("已取消。".into());
        }
        let code = match request.language {
            Language::Chinese => "zh",
            Language::English => "en",
            Language::Auto => pass.chinese_or_english(&samples[stretch.clone()])?,
        };
        open = match open.take() {
            Some((window, open_code))
                if open_code == code && stretch.end - window.start <= WINDOW_FRAMES * FRAME =>
            {
                Some((window.start..stretch.end, code))
            }
            Some((window, open_code)) => {
                pass.transcribe_window(&samples, window, open_code)?;
                Some((stretch, code))
            }
            None => Some((stretch, code)),
        };
    }
    if let Some((window, code)) = open {
        pass.transcribe_window(&samples, window, code)?;
    }
    pass.progressed(pass.duration_ms);
    Ok(())
}

/// One Transcription in progress on the worker thread.
struct Pass<'a> {
    state: WhisperState,
    threads: usize,
    path: &'a Path,
    events: &'a Sender<TranscriptionEvent>,
    duration_ms: u64,
    cancel: &'a Cancel,
}

impl Pass<'_> {
    fn report(&self, event: TranscriptionEvent) {
        self.cancel.send(self.events, event);
    }

    fn progressed(&self, processed_ms: u64) {
        self.report(TranscriptionEvent::Progressed {
            path: self.path.to_path_buf(),
            processed_ms,
            duration_ms: self.duration_ms,
        });
    }

    /// ADR 0002: a stretch is judged as only Chinese or English.
    fn chinese_or_english(&mut self, audio: &[f32]) -> Result<&'static str, String> {
        let failed = |error| format!("语言判断失败：{error}");
        self.state.pcm_to_mel(audio, self.threads).map_err(failed)?;
        let (_, probabilities) = self.state.lang_detect(0, self.threads).map_err(failed)?;
        let probability = |code| {
            whisper_rs::get_lang_id(code)
                .and_then(|id| probabilities.get(id as usize).copied())
                .unwrap_or(0.0)
        };
        // Cantonese has no mode of its own; it goes down the Chinese path.
        Ok(
            if probability("en") > probability("zh") + probability("yue") {
                "en"
            } else {
                "zh"
            },
        )
    }

    fn transcribe_window(
        &mut self,
        samples: &[f32],
        window: Range<usize>,
        code: &str,
    ) -> Result<(), String> {
        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_n_threads(self.threads as i32);
        params.set_language(Some(code));
        params.set_translate(false);
        params.set_no_context(true);
        params.set_suppress_nst(true);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // whisper-rs 0.16's `set_abort_callback_safe` casts its boxed closure
        // to the wrong type, so the flag is handed to whisper.cpp directly.
        // SAFETY: `self.cancel` outlives this `full` call, and
        // `cancel_requested` only reads the AtomicBool behind the pointer.
        unsafe {
            params.set_abort_callback(Some(cancel_requested));
            params.set_abort_callback_user_data(
                &self.cancel.raised as *const AtomicBool as *mut c_void,
            );
        }
        let mut audio = vec![0.0; LEAD_IN_SAMPLES];
        audio.extend_from_slice(&samples[window.clone()]);
        self.state
            .full(params, &audio)
            .map_err(|error| format!("转写中断：{error}"))?;

        let at = |timestamp| {
            (ms(window.start) + centiseconds(timestamp)).saturating_sub(ms(LEAD_IN_SAMPLES))
        };
        for segment in self.state.as_iter() {
            let text = segment
                .to_str_lossy()
                .map_err(|error| format!("读取分段失败：{error}"))?
                .trim()
                .to_owned();
            if text.is_empty() {
                continue;
            }
            self.report(TranscriptionEvent::Segment {
                path: self.path.to_path_buf(),
                segment: Segment {
                    start_ms: at(segment.start_timestamp()),
                    end_ms: at(segment.end_timestamp()),
                    text,
                },
            });
        }
        self.progressed(ms(window.end));
        Ok(())
    }
}

/// ADR 0003: the GPU (Metal on macOS, Vulkan elsewhere) when it comes up,
/// and that context is kept for later Recordings. When it does not, this one
/// Transcription runs on the CPU and the next one tries the GPU again.
fn whisper_state(weight: &Path, gpu: &mut Option<WhisperContext>) -> Result<WhisperState, String> {
    if gpu.is_none() && gpu_present() {
        *gpu = load(weight, true).ok();
    }
    if let Some(state) = gpu.as_ref().and_then(|context| context.create_state().ok()) {
        return Ok(state);
    }
    *gpu = None;
    load(weight, false)?
        .create_state()
        .map_err(|error| format!("模型无法开始转写：{error}"))
}

fn load(weight: &Path, gpu: bool) -> Result<WhisperContext, String> {
    let mut params = WhisperContextParameters::default();
    params.use_gpu(gpu);
    // Faster on Metal, and on Vulkan the attention path without it drops
    // words; see docs/whisper-vulkan-vs-metal-bench.md.
    params.flash_attn(gpu);
    WhisperContext::new_with_params(weight, params)
        .map_err(|error| format!("模型无法加载：{error}"))
}

/// Every Mac Visper runs on has Metal; whisper.cpp itself falls back to the
/// CPU if its Metal device cannot be set up.
#[cfg(target_os = "macos")]
fn gpu_present() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
fn gpu_present() -> bool {
    !whisper_rs::vulkan::list_devices().is_empty()
}

/// Speech between pauses, each at most one window long, without the
/// stretches that hold no speech.
fn stretches(samples: &[f32]) -> Vec<Range<usize>> {
    let loudness: Vec<f32> = samples
        .chunks(FRAME)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .collect();
    let mut sorted = loudness.clone();
    sorted.sort_by(f32::total_cmp);
    let floor = sorted.get(sorted.len() / 10).copied().unwrap_or(0.0);
    let threshold = (floor * 4.0).clamp(0.003, 0.03);
    let quiet: Vec<bool> = loudness.iter().map(|level| *level < threshold).collect();

    let mut cuts = vec![0];
    cuts.extend(
        pauses(&quiet, 0, quiet.len(), PAUSE_FRAMES).map(|pause| pause.start + pause.len() / 2),
    );
    cuts.push(quiet.len());

    let mut ranges = Vec::new();
    for pair in cuts.windows(2) {
        let (mut start, end) = (pair[0], pair[1]);
        while start < end {
            let stop = if end - start <= WINDOW_FRAMES {
                end
            } else {
                pauses(
                    &quiet,
                    start + WINDOW_FRAMES / 3,
                    start + WINDOW_FRAMES,
                    BREATH_FRAMES,
                )
                .max_by_key(|pause| pause.len())
                .map(|pause| pause.start + pause.len() / 2)
                .unwrap_or(start + WINDOW_FRAMES)
            };
            let voiced = quiet[start..stop].iter().filter(|quiet| !**quiet).count();
            if voiced >= VOICED_FRAMES {
                ranges.push(start * FRAME..(stop * FRAME).min(samples.len()));
            }
            start = stop;
        }
    }
    ranges
}

/// Runs of at least `shortest` quiet frames inside `from..to`.
fn pauses(
    quiet: &[bool],
    from: usize,
    to: usize,
    shortest: usize,
) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut run = None;
    quiet[from..to]
        .iter()
        .chain([&false])
        .enumerate()
        .filter_map(move |(offset, is_quiet)| {
            let index = from + offset;
            match (*is_quiet, run) {
                (true, None) => {
                    run = Some(index);
                    None
                }
                (false, Some(start)) => {
                    run = None;
                    (index - start >= shortest).then_some(start..index)
                }
                _ => None,
            }
        })
}

fn decode(path: &Path, cancel: &Cancel) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|error| format!("打不开录音：{error}"))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| format!("无法识别录音格式：{error}"))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("录音里没有可解码的音频。")?;
    let track_id = track.id;
    let rate = track.codec_params.sample_rate.ok_or("录音没有采样率。")? as usize;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| format!("无法解码录音：{error}"))?;

    let mut audio = Resampled::new(rate)?;
    loop {
        if cancel.is_raised() {
            return Err("已取消。".into());
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(DecodeError::ResetRequired) => break,
            Err(error) => return Err(format!("读取录音失败：{error}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(DecodeError::DecodeError(_)) => continue,
            Err(error) => return Err(format!("解码录音失败：{error}")),
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        buffer.copy_interleaved_ref(decoded);
        audio.push(
            buffer
                .samples()
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        )?;
    }
    audio.finish()
}

/// Mono audio converted to 16 kHz as it is decoded, so a long Recording is
/// never held at its original rate.
struct Resampled {
    resampler: Option<FftFixedInOut<f32>>,
    rate: usize,
    pending: Vec<f32>,
    output: Vec<f32>,
    received: usize,
}

impl Resampled {
    fn new(rate: usize) -> Result<Self, String> {
        let resampler = if rate == SAMPLE_RATE {
            None
        } else {
            Some(
                FftFixedInOut::<f32>::new(rate, SAMPLE_RATE, 1024, 1)
                    .map_err(|error| format!("重采样失败：{error}"))?,
            )
        };
        Ok(Self {
            resampler,
            rate,
            pending: Vec::new(),
            output: Vec::new(),
            received: 0,
        })
    }

    fn push(&mut self, samples: impl Iterator<Item = f32>) -> Result<(), String> {
        let Some(resampler) = self.resampler.as_mut() else {
            self.output.extend(samples);
            return Ok(());
        };
        let before = self.pending.len();
        self.pending.extend(samples);
        self.received += self.pending.len() - before;
        let mut used = 0;
        while self.pending.len() - used >= resampler.input_frames_next() {
            let next = resampler.input_frames_next();
            let block = resampler
                .process(&[&self.pending[used..used + next]], None)
                .map_err(|error| format!("重采样失败：{error}"))?;
            self.output.extend_from_slice(&block[0]);
            used += next;
        }
        self.pending.drain(..used);
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<f32>, String> {
        let Some(mut resampler) = self.resampler.take() else {
            return Ok(self.output);
        };
        let failed = |error: &dyn std::fmt::Display| format!("重采样失败：{error}");
        if self.received == 0 {
            return Ok(Vec::new());
        }
        let expected = self.received * SAMPLE_RATE / self.rate;
        let delay = resampler.output_delay();
        let tail = resampler
            .process_partial(Some(&[&self.pending[..]]), None)
            .map_err(|error| failed(&error))?;
        self.output.extend_from_slice(&tail[0]);
        while self.output.len() < expected + delay {
            let flushed = resampler
                .process_partial::<&[f32]>(None, None)
                .map_err(|error| failed(&error))?;
            self.output.extend_from_slice(&flushed[0]);
        }
        self.output.truncate(expected + delay);
        self.output.drain(..delay);
        Ok(self.output)
    }
}

unsafe extern "C" fn cancel_requested(flag: *mut c_void) -> bool {
    // SAFETY: the user data set next to this callback is an `AtomicBool`.
    unsafe { (*(flag as *const AtomicBool)).load(Ordering::SeqCst) }
}

fn ms(samples: usize) -> u64 {
    (samples as u64 * 1000) / SAMPLE_RATE as u64
}

fn centiseconds(value: i64) -> u64 {
    value.max(0) as u64 * 10
}
