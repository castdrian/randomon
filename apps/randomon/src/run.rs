use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream, StreamExt};

#[derive(Debug, Clone, PartialEq)]
pub struct RunProgress {
    pub fraction: f32,
    pub label: &'static str,
    pub detail: Option<String>,
}

impl RunProgress {
    pub fn starting() -> Self {
        Self {
            fraction: 0.0,
            label: "Getting ready",
            detail: None,
        }
    }

    pub fn percent(&self) -> u8 {
        (self.fraction.clamp(0.0, 1.0) * 100.0).floor() as u8
    }
}

impl From<randomon_core::Progress> for RunProgress {
    fn from(progress: randomon_core::Progress) -> Self {
        Self {
            fraction: progress.fraction(),
            label: progress.label,
            detail: progress.detail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub headline: &'static str,
    pub output: PathBuf,
    pub log: Option<PathBuf>,
    pub race_code: Option<String>,
    pub install_hint: Option<String>,
}

impl Finished {
    pub fn output_name(&self) -> String {
        self.output
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.output.display().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum RunState {
    #[default]
    Idle,
    Running(RunProgress),
    Done(Finished),
    Failed(String),
}

impl RunState {
    pub fn is_running(&self) -> bool {
        matches!(self, RunState::Running(_))
    }

    pub fn advance(&mut self, mut progress: RunProgress) {
        let RunState::Running(current) = self else {
            return;
        };
        if progress.detail.is_none() && progress.label == current.label {
            progress.detail = current.detail.take();
        }
        progress.fraction = progress.fraction.max(current.fraction);
        *current = progress;
    }
}

#[derive(Debug, Clone)]
pub enum RunEvent<T> {
    Progress(RunProgress),
    Finished(Result<T, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardAction {
    Reveal(PathBuf),
    OpenLog(PathBuf),
    RandomizeAgain,
    Dismiss,
}

pub type ProgressSink = Arc<dyn Fn(RunProgress) + Send + Sync>;

pub fn in_background<T: Send + 'static>(
    work: impl FnOnce(ProgressSink) -> Result<T, String> + Send + 'static,
) -> impl Stream<Item = RunEvent<T>> {
    iced::stream::channel(32, async move |mut output: mpsc::Sender<RunEvent<T>>| {
        let (sender, mut updates) = mpsc::unbounded();
        let sink: ProgressSink = Arc::new(move |progress| {
            let _ = sender.unbounded_send(progress);
        });
        let job = tokio::task::spawn_blocking(move || {
            crate::engine::guarded(move || work(sink)).and_then(|result| result)
        });
        while let Some(progress) = updates.next().await {
            let _ = output.send(RunEvent::Progress(progress)).await;
        }
        let result = job.await.map_err(|error| error.to_string()).and_then(|r| r);
        let _ = output.send(RunEvent::Finished(result)).await;
    })
}

pub fn handle_card_side_effect(action: &CardAction) {
    match action {
        CardAction::Reveal(path) => reveal_in_file_manager(path),
        CardAction::OpenLog(path) => open_with_default_app(path),
        CardAction::RandomizeAgain | CardAction::Dismiss => {}
    }
}

pub const REVEAL_LABEL: &str = if cfg!(target_os = "macos") {
    "Show in Finder"
} else if cfg!(target_os = "windows") {
    "Show in Explorer"
} else {
    "Show in folder"
};

fn reveal_in_file_manager(path: &Path) {
    if let Err(error) = reveal_command(path).spawn() {
        log::warn!("failed to reveal {}: {error}", path.display());
    }
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    command.arg("-R").arg(path);
    command
}

#[cfg(target_os = "windows")]
fn reveal_command(path: &Path) -> std::process::Command {
    let mut select = std::ffi::OsString::from("/select,");
    select.push(path.as_os_str());
    let mut command = std::process::Command::new("explorer");
    command.arg(select);
    command
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn reveal_command(path: &Path) -> std::process::Command {
    let folder = path.parent().unwrap_or(path);
    let mut command = std::process::Command::new("xdg-open");
    command.arg(folder);
    command
}

fn open_with_default_app(path: &Path) {
    if let Err(error) = open::that_detached(path) {
        log::warn!("failed to open {}: {error}", path.display());
    }
}

pub fn race_code(check_value: i32) -> String {
    format!("{check_value:08X}")
}
