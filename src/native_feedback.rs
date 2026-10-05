use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::sync::OnceLock;

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeFeedbackKind {
    Copy,
    Paste,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FeedbackDispatch {
    Disabled,
    Queued,
    Busy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeFeedbackPlayback {
    pub backend: &'static str,
    pub used_default_fallback: bool,
}

#[derive(Clone, Debug)]
struct FeedbackConfig {
    kind: String,
    custom_path: PathBuf,
}

#[derive(Clone, Copy, Debug)]
enum SoundSource<'a> {
    BuiltIn(&'static str, &'static [u8]),
    Custom(&'a Path),
}

static DEFAULT_SOUND: &[u8] = include_bytes!("../assets/sounds/paste_default.wav");
static SOFT_SOUND: &[u8] = include_bytes!("../assets/sounds/paste_soft.wav");
static BRIGHT_SOUND: &[u8] = include_bytes!("../assets/sounds/paste_bright.wav");

impl FeedbackConfig {
    fn from_json(settings: &Value) -> Self {
        Self {
            kind: settings
                .get("paste_success_sound_kind")
                .and_then(Value::as_str)
                .unwrap_or("default")
                .trim()
                .to_string(),
            custom_path: settings
                .get("paste_success_sound_path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .into(),
        }
    }
}

fn enabled(kind: NativeFeedbackKind, settings: &Value) -> bool {
    let key = match kind {
        NativeFeedbackKind::Copy => "copy_success_sound_enabled",
        NativeFeedbackKind::Paste => "paste_success_sound_enabled",
    };
    settings.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Call once at the owning operation's successful completion boundary.
/// Queueing is not evidence that a desktop audio device produced audible sound.
pub(crate) fn notify_success(kind: NativeFeedbackKind, settings: &Value) -> FeedbackDispatch {
    if !enabled(kind, settings) {
        return FeedbackDispatch::Disabled;
    }
    static WORKER: OnceLock<SyncSender<(NativeFeedbackKind, FeedbackConfig)>> = OnceLock::new();
    let sender = WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<(NativeFeedbackKind, FeedbackConfig)>(16);
        std::thread::spawn(move || {
            for (kind, config) in receiver {
                match play_with_backend(&config, play_source) {
                    Ok(result) => eprintln!("ZSClip native sound kind={kind:?} backend={} completed=true default_fallback={}", result.backend, result.used_default_fallback),
                    Err(error) => eprintln!("ZSClip native sound kind={kind:?} completed=false error={error}"),
                }
            }
        });
        sender
    });
    match sender.try_send((kind, FeedbackConfig::from_json(settings))) {
        Ok(()) => FeedbackDispatch::Queued,
        Err(_) => FeedbackDispatch::Busy,
    }
}

/// Explicit preview ignores the notification switches. Hosts run this off their UI thread.
pub(crate) fn preview(settings: &Value) -> Result<NativeFeedbackPlayback, String> {
    play_with_backend(&FeedbackConfig::from_json(settings), play_source)
}

fn play_with_backend(
    config: &FeedbackConfig,
    mut play: impl FnMut(SoundSource<'_>) -> Result<&'static str, String>,
) -> Result<NativeFeedbackPlayback, String> {
    let mut fallback = false;
    if config.kind == "custom" {
        if !config.custom_path.as_os_str().is_empty() {
            if let Ok(backend) = play(SoundSource::Custom(&config.custom_path)) {
                return Ok(NativeFeedbackPlayback {
                    backend,
                    used_default_fallback: false,
                });
            }
        }
        fallback = true;
    }
    let (name, bytes) = match config.kind.as_str() {
        "soft" => ("soft", SOFT_SOUND),
        "bright" => ("bright", BRIGHT_SOUND),
        _ => ("default", DEFAULT_SOUND),
    };
    if let Ok(backend) = play(SoundSource::BuiltIn(name, bytes)) {
        return Ok(NativeFeedbackPlayback {
            backend,
            used_default_fallback: fallback,
        });
    }
    if name != "default" {
        let backend = play(SoundSource::BuiltIn("default", DEFAULT_SOUND))?;
        return Ok(NativeFeedbackPlayback {
            backend,
            used_default_fallback: true,
        });
    }
    Err("No native audio player completed playback".into())
}

fn play_source(source: SoundSource<'_>) -> Result<&'static str, String> {
    match source {
        SoundSource::Custom(path) => {
            let path = path
                .canonicalize()
                .map_err(|_| "Custom sound file is unavailable")?;
            let metadata =
                std::fs::metadata(&path).map_err(|_| "Custom sound file is unavailable")?;
            if !metadata.is_file() || metadata.len() > 16 * 1024 * 1024 {
                return Err("Custom sound file is invalid or too large".into());
            }
            play_file(&path)
        }
        SoundSource::BuiltIn(_, bytes) => {
            use std::io::Write;
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
            struct TemporarySound(PathBuf);
            impl Drop for TemporarySound {
                fn drop(&mut self) {
                    let _ = std::fs::remove_file(&self.0);
                }
            }
            let serial = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "zsclip-feedback-{}-{stamp}-{serial}.wav",
                std::process::id()
            ));
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| error.to_string())?;
            let temporary = TemporarySound(path);
            file.write_all(bytes).map_err(|error| error.to_string())?;
            drop(file);
            play_file(&temporary.0)
        }
    }
}

fn play_file(path: &Path) -> Result<&'static str, String> {
    #[cfg(target_os = "macos")]
    let players: &[(&str, &[&str])] = &[("/usr/bin/afplay", &[])];
    #[cfg(target_os = "linux")]
    let players: &[(&str, &[&str])] = &[
        ("/usr/bin/paplay", &["--"]),
        ("/usr/bin/pw-play", &["--"]),
        ("/usr/bin/aplay", &["-q", "--"]),
    ];
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let players: &[(&str, &[&str])] = &[];
    for &(program, arguments) in players {
        let result = std::process::Command::new(program)
            .args(arguments)
            .arg(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if result.is_ok_and(|status| status.success()) {
            return Ok(program);
        }
    }
    Err("Native audio playback is unavailable or failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_feedback_copy_and_paste_switches_are_independent() {
        for copy in [false, true] {
            for paste in [false, true] {
                let settings = serde_json::json!({"copy_success_sound_enabled":copy,"paste_success_sound_enabled":paste});
                assert_eq!(enabled(NativeFeedbackKind::Copy, &settings), copy);
                assert_eq!(enabled(NativeFeedbackKind::Paste, &settings), paste);
            }
        }
        assert!(!enabled(NativeFeedbackKind::Copy, &serde_json::json!({})));
        assert!(!enabled(NativeFeedbackKind::Paste, &serde_json::json!({})));
    }

    #[test]
    fn successful_custom_native_sound_does_not_play_a_second_tone() {
        let config = FeedbackConfig::from_json(
            &serde_json::json!({"paste_success_sound_kind":"custom","paste_success_sound_path":"chosen.wav"}),
        );
        let mut calls = 0;
        let result = play_with_backend(&config, |source| {
            assert!(matches!(source, SoundSource::Custom(_)));
            calls += 1;
            Ok("test-player")
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert!(!result.used_default_fallback);
    }

    #[test]
    fn failed_custom_native_sound_falls_back_once_and_reports_failure_honestly() {
        let config = FeedbackConfig::from_json(
            &serde_json::json!({"paste_success_sound_kind":"custom","paste_success_sound_path":"missing.wav"}),
        );
        let mut calls = Vec::new();
        let result = play_with_backend(&config, |source| match source {
            SoundSource::Custom(_) => {
                calls.push("custom");
                Err("missing".into())
            }
            SoundSource::BuiltIn(name, bytes) => {
                calls.push(name);
                assert_eq!(bytes, DEFAULT_SOUND);
                Ok("test-player")
            }
        })
        .unwrap();
        assert_eq!(calls, ["custom", "default"]);
        assert!(result.used_default_fallback);
        assert!(play_with_backend(&config, |_| Err("no audio device".into())).is_err());
    }

    #[test]
    fn explicit_preview_plan_uses_selected_tone_even_when_notifications_are_off() {
        let config = FeedbackConfig::from_json(
            &serde_json::json!({"paste_success_sound_kind":"soft","copy_success_sound_enabled":false,"paste_success_sound_enabled":false}),
        );
        let mut calls = Vec::new();
        let result = play_with_backend(&config, |source| match source {
            SoundSource::BuiltIn(name, _) => {
                calls.push(name);
                if name == "soft" {
                    Err("unsupported".into())
                } else {
                    Ok("test-player")
                }
            }
            _ => unreachable!(),
        })
        .unwrap();
        assert_eq!(calls, ["soft", "default"]);
        assert!(result.used_default_fallback);
    }
}
