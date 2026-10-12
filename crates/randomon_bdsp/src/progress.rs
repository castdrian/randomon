use std::cell::Cell;
use std::time::{Duration, Instant};

pub const DETAIL_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub completed_steps: usize,
    pub total_steps: usize,
    pub stage_fraction: f32,
    pub label: &'static str,
    pub detail: Option<String>,
}

impl Progress {
    pub fn fraction(&self) -> f32 {
        if self.total_steps == 0 {
            return 1.0;
        }
        let done = self.completed_steps as f32 + self.stage_fraction.clamp(0.0, 1.0);
        (done / self.total_steps as f32).clamp(0.0, 1.0)
    }

    pub fn percent(&self) -> u8 {
        (self.fraction() * 100.0).floor() as u8
    }

    pub fn is_finished(&self) -> bool {
        self.completed_steps >= self.total_steps
    }
}

pub const LOADING_LABEL: &str = "Reading the game files";
pub const PACKING_LABEL: &str = "Packing changed bundles";
pub const WRITING_LABEL: &str = "Writing the mod files";
pub const FINISHED_LABEL: &str = "Done";

pub struct Tracker<'a> {
    total_steps: usize,
    completed_steps: Cell<usize>,
    label: Cell<&'static str>,
    last_emitted: Cell<Option<Instant>>,
    on_progress: &'a dyn Fn(Progress),
}

impl<'a> Tracker<'a> {
    pub fn new(total_steps: usize, on_progress: &'a dyn Fn(Progress)) -> Self {
        Self {
            total_steps,
            completed_steps: Cell::new(0),
            label: Cell::new(""),
            last_emitted: Cell::new(None),
            on_progress,
        }
    }

    fn emit(&self, stage_fraction: f32, detail: Option<String>) {
        self.last_emitted.set(Some(Instant::now()));
        (self.on_progress)(Progress {
            completed_steps: self.completed_steps.get(),
            total_steps: self.total_steps,
            stage_fraction,
            label: self.label.get(),
            detail,
        });
    }

    pub fn begin(&self, label: &'static str) {
        self.label.set(label);
        self.emit(0.0, None);
    }

    pub fn detail(&self, done: usize, total: usize, describe: impl FnOnce() -> String) {
        let due = self
            .last_emitted
            .get()
            .is_none_or(|emitted| emitted.elapsed() >= DETAIL_INTERVAL);
        if !due && done + 1 < total {
            return;
        }
        let fraction = if total == 0 {
            0.0
        } else {
            done as f32 / total as f32
        };
        self.emit(fraction, Some(describe()));
    }

    pub fn complete(&self) {
        let next = (self.completed_steps.get() + 1).min(self.total_steps);
        self.completed_steps.set(next);
    }

    pub fn finish(&self) {
        self.completed_steps.set(self.total_steps);
        self.begin(FINISHED_LABEL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn details_are_throttled_but_the_last_item_always_shows() {
        let seen = RefCell::new(Vec::new());
        let record = |progress: Progress| seen.borrow_mut().push(progress);
        let tracker = Tracker::new(2, &record);
        tracker.begin(WRITING_LABEL);
        for file in 0..500 {
            tracker.detail(file, 500, || format!("file {file}"));
        }
        tracker.complete();
        tracker.finish();
        let seen = seen.into_inner();
        assert!(seen.len() < 20);
        let last_detail = seen
            .iter()
            .rev()
            .find_map(|progress| progress.detail.clone());
        assert_eq!(last_detail.as_deref(), Some("file 499"));
        assert!(seen.last().expect("finished").is_finished());
    }
}
