//! Isolated debug-only scenarios for visual review. No model download or user Library.
use super::{PlaybackState, Store, VisperWindow, render};
use gpui::{App, WindowAppearance, WindowHandle, point, px, size};
use serde_json::json;
use std::path::Path;

pub(super) struct Noop;
impl visper::Downloader for Noop {
    fn start(&mut self, _: visper::Model) {}
    fn abandon(&mut self, _: visper::Model) {}
}
impl visper::Transcriber for Noop {
    fn start(&mut self, _: &Path, _: visper::Language, _: visper::Model) {}
    fn cancel(&mut self) {}
}

/// The selected Recording in every scenario.
const RECORDING: &str = "/Users/lin/录音/产品讨论.m4a";

fn segments() -> Vec<serde_json::Value> {
    [
        (4_000, 18_000, "我们先看一下这次产品讨论的目标。"),
        (18_000, 36_000, "界面需要更清晰，让主要操作自然出现。"),
        (
            36_000,
            52_000,
            "We should keep the workflow simple and predictable.",
        ),
        (52_000, 76_000, "语言自动判断，中文和英文都保留原文。"),
        (76_000, 102_000, "接下来讨论导入和转写的反馈。"),
        (
            102_000,
            120_000,
            "完成之后，可以按分段回听，再导出 Markdown。",
        ),
    ]
    .into_iter()
    .map(|(start, end, text)| {
        json!({
            "start_ms": start, "end_ms": end, "text": text
        })
    })
    .collect()
}

fn fixture(scenario: &str) -> visper::Library {
    let path = RECORDING;
    let mut rows = Vec::new();
    let blank = scenario == "empty" || scenario.starts_with("first-launch");
    if !blank {
        let status = if scenario == "transcribing" || scenario.starts_with("model-download") {
            "Queued"
        } else if scenario == "failure" {
            "Failed"
        } else {
            "Completed"
        };
        let mut transcript = segments();
        if scenario == "long" {
            transcript = (0..1000).map(|index| json!({
                "start_ms": index * 18_000, "end_ms": (index + 1) * 18_000,
                "text": format!("第 {} 个分段。我们使用真实的中英文混排检查长内容的换行与阅读布局。The layout should remain stable while the transcript grows. 一个很长的连续单词用于检验换行：abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz。", index + 1),
            })).collect();
        }
        rows.push(json!({
            "path": path, "status": status, "language": "Auto", "duration_ms": 1_085_000,
            "segments": if status == "Queued" { Vec::new() } else { transcript },
            "transcript_stored": status != "Queued", "finished_order": 10,
            "missing": scenario == "missing", "transcription_ms": 248_000,
            "transcript_model": "LargeV3Turbo",
            "error": if scenario == "failure" { Some("无法解码录音。上次成功保存的转写稿仍然可用。") } else { None },
        }));
        for (index, name) in [
            "访谈记录.wav",
            "课程录音.mp3",
            "团队周会.m4a",
            "项目回顾.flac",
        ]
        .iter()
        .enumerate()
        {
            rows.push(json!({
                "path": format!("/Users/lin/录音/{name}"),
                "status": if scenario == "transcribing" && index < 2 { "Queued" } else { "Completed" },
                "segments": segments(), "duration_ms": 2_400_000,
                "language": "Auto", "finished_order": 4 - index, "transcription_ms": 577_000,
                "transcript_stored": true,
                // The first one has no recorded Model: an old Transcript.
                "transcript_model": if index == 1 { Some("LargeV3Turbo") } else { None },
            }));
        }
        if scenario == "long" {
            for index in 5..100 {
                rows.push(json!({
                    "path": format!("/Users/lin/录音/第{index}份录音_包含较长名称的产品讨论与技术评审记录.m4a"),
                    "status": "Completed", "segments": segments(), "language": "Auto",
                    "finished_order": index, "transcript_stored": true
                }));
            }
            rows[0]["path"] = json!(
                "/Users/lin/文稿/年度计划/项目资料/产品研究/转写测试/这是一份包含很长文件名称与多层路径的产品讨论记录用于检查布局是否发生重叠_2026-10-14.m4a"
            );
        }
    }
    let selected = rows.first().map(|row| row["path"].clone());
    let mut library = visper::Library::from_json(
        &json!({
            "recordings": rows, "selected": selected, "finished_count": 200
        })
        .to_string(),
    )
    .expect("valid preview Library");
    let on_disk = if scenario.starts_with("first-launch") {
        vec![]
    } else {
        vec![visper::Model::LargeV3]
    };
    if scenario == "model-download" || scenario == "model-download-failed" {
        // Chosen before the queue starts, so the Queued Recording waits.
        library.choose(visper::Model::LargeV3Turbo, &mut Noop, &mut Noop);
    }
    library.prepare(&on_disk, &mut Noop, &mut Noop);
    if scenario == "first-launch-download" || scenario == "first-launch-failed" {
        // The click on a card, without the network behind it.
        library.choose(visper::Model::LargeV3, &mut Noop, &mut Noop);
    }
    if scenario == "first-launch-download" {
        library.download_progressed(visper::Model::LargeV3, 1_795_000_000, 3_095_033_483);
    } else if scenario == "first-launch-failed" {
        library.download_failed(visper::Model::LargeV3, "连接下载地址失败：网络中断".into());
    } else if scenario == "model-download-failed" {
        library.download_failed(
            visper::Model::LargeV3Turbo,
            "连接下载地址失败：网络中断".into(),
        );
    } else if scenario == "model-download" {
        library.download_progressed(visper::Model::LargeV3Turbo, 640_000_000, 1_624_555_275);
    } else if scenario == "transcribing" {
        for value in segments() {
            let segment: visper::Segment = serde_json::from_value(value).expect("preview Segment");
            library.segment_transcribed(Path::new(path), segment);
        }
        library.transcription_progressed(Path::new(path), 738_000, 1_085_000);
    }
    library
}

pub fn open_preview(scenario: &str, cx: &mut App) -> WindowHandle<VisperWindow> {
    let dark = scenario.ends_with("-dark");
    let scenario = scenario.trim_end_matches("-dark").to_owned();
    let library = fixture(&scenario);
    let store = Store::new(
        std::env::temp_dir().join(format!("visper-preview-{}.json", std::process::id())),
    );
    let window = render::open_with(cx, move |cx| {
        let mut view = VisperWindow::build(
            store,
            std::env::temp_dir().join("visper-preview-no-model"),
            Some(library),
            cx,
        );
        view.preview_appearance = Some(if dark {
            WindowAppearance::Dark
        } else {
            WindowAppearance::Light
        });
        view.model_open = scenario == "model-popover";
        if scenario == "long" {
            // Both lists part-way down, with their bars on show; the
            // VISPER_PREVIEW_BAR=hot variant draws them as if hovered.
            view.transcript_scroll
                .set_offset(point(px(0.0), px(-6_000.0)));
            view.library_scroll.set_offset(point(px(0.0), px(-1_200.0)));
            let hot = std::env::var("VISPER_PREVIEW_BAR").is_ok_and(|bar| bar == "hot");
            view.transcript_bar.pin(hot);
            view.library_bar.pin(hot);
        }
        if scenario == "transcribing" {
            view.clock = Some(super::TranscriptionClock::new(
                RECORDING.into(),
                std::time::Instant::now() - std::time::Duration::from_secs(252),
            ));
        }
        if scenario == "completed" {
            view.preview_playback = Some(PlaybackState {
                playing: true,
                position_ms: 41_000,
                duration_ms: Some(1_085_000),
            });
        }
        view
    });
    if let Ok(dimensions) = std::env::var("VISPER_PREVIEW_SIZE")
        && let Some((width, height)) = dimensions.split_once('x')
        && let (Ok(width), Ok(height)) = (width.parse::<f32>(), height.parse::<f32>())
    {
        let _ = window.update(cx, |_, window, _| {
            window.resize(size(px(width), px(height)))
        });
    }
    window
}
