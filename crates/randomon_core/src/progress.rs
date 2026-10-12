use std::fmt;
use std::sync::{Arc, Mutex};
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

type Callback = dyn Fn(Progress) + Send + Sync;

#[derive(Clone)]
pub struct ProgressReporter(Arc<Callback>);

impl ProgressReporter {
    pub fn new(callback: impl Fn(Progress) + Send + Sync + 'static) -> Self {
        Self(Arc::new(callback))
    }

    pub fn report(&self, progress: Progress) {
        (self.0)(progress);
    }
}

impl fmt::Debug for ProgressReporter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProgressReporter")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Moves,
    MiscTweaks,
    Types,
    Evolutions,
    BaseStats,
    Abilities,
    Starters,
    Movesets,
    TmsAndTutors,
    Trainers,
    Statics,
    WildEncounters,
    Trades,
    Items,
    Saving,
    WritingLog,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Moves => "Updating move data",
            Stage::MiscTweaks => "Applying tweaks",
            Stage::Types => "Randomizing types",
            Stage::Evolutions => "Randomizing evolutions",
            Stage::BaseStats => "Randomizing base stats",
            Stage::Abilities => "Randomizing abilities",
            Stage::Starters => "Picking starters",
            Stage::Movesets => "Randomizing movesets",
            Stage::TmsAndTutors => "Randomizing TMs and tutors",
            Stage::Trainers => "Randomizing trainers",
            Stage::Statics => "Randomizing static Pokémon",
            Stage::WildEncounters => "Randomizing wild Pokémon",
            Stage::Trades => "Randomizing trades",
            Stage::Items => "Randomizing items",
            Stage::Saving => "Saving the game",
            Stage::WritingLog => "Writing the spoiler log",
        }
    }
}

pub const FINISHED_LABEL: &str = "Done";

#[derive(Debug)]
struct Position {
    step: usize,
    fraction: f32,
    label: &'static str,
    last_emitted: Option<Instant>,
    stage_has_detail: bool,
}

#[derive(Debug)]
pub struct ProgressPlan {
    stages: Vec<Stage>,
    reporter: Option<ProgressReporter>,
    position: Mutex<Position>,
}

impl ProgressPlan {
    pub fn new(stages: Vec<Stage>, reporter: Option<ProgressReporter>) -> Self {
        Self {
            stages,
            reporter,
            position: Mutex::new(Position {
                step: 0,
                fraction: 0.0,
                label: "",
                last_emitted: None,
                stage_has_detail: false,
            }),
        }
    }

    pub fn silent() -> Self {
        Self::new(Vec::new(), None)
    }

    pub fn total_steps(&self) -> usize {
        self.stages.len()
    }

    fn step_of(&self, stage: Stage) -> Option<usize> {
        self.stages.iter().position(|&planned| planned == stage)
    }

    fn emit(&self, position: &mut Position, detail: Option<String>) {
        let Some(reporter) = &self.reporter else {
            return;
        };
        position.last_emitted = Some(Instant::now());
        if detail.is_some() {
            position.stage_has_detail = true;
        }
        reporter.report(Progress {
            completed_steps: position.step,
            total_steps: self.total_steps(),
            stage_fraction: position.fraction,
            label: position.label,
            detail,
        });
    }

    fn is_due(position: &Position) -> bool {
        !position.stage_has_detail
            || position
                .last_emitted
                .is_none_or(|emitted| emitted.elapsed() >= DETAIL_INTERVAL)
    }

    pub fn begin(&self, stage: Stage) {
        let (Some(_), Some(step)) = (&self.reporter, self.step_of(stage)) else {
            return;
        };
        let Ok(mut position) = self.position.lock() else {
            return;
        };
        if step < position.step || (step == position.step && position.label == stage.label()) {
            return;
        }
        position.step = step;
        position.fraction = 0.0;
        position.label = stage.label();
        position.stage_has_detail = false;
        self.emit(&mut position, None);
    }

    pub fn within(&self, stage: Stage, fraction: f32) {
        self.within_detailed(stage, fraction, || None);
    }

    pub fn within_detailed(
        &self,
        stage: Stage,
        fraction: f32,
        describe: impl FnOnce() -> Option<String>,
    ) {
        if self.reporter.is_none() || self.step_of(stage).is_none() {
            return;
        }
        self.begin(stage);
        let Ok(mut position) = self.position.lock() else {
            return;
        };
        if position.label != stage.label() {
            return;
        }
        position.fraction = position.fraction.max(fraction.clamp(0.0, 1.0));
        if Self::is_due(&position) || fraction >= 1.0 {
            self.emit(&mut position, describe());
        }
    }

    pub fn detail(&self, done: usize, total: usize, describe: impl FnOnce() -> String) {
        if self.reporter.is_none() {
            return;
        }
        let Ok(mut position) = self.position.lock() else {
            return;
        };
        if position.label.is_empty() || !Self::is_due(&position) {
            return;
        }
        if total > 0 {
            position.fraction = position.fraction.max(done as f32 / total as f32).min(1.0);
        }
        self.emit(&mut position, Some(describe()));
    }

    pub fn finish(&self) {
        if self.reporter.is_none() {
            return;
        }
        let Ok(mut position) = self.position.lock() else {
            return;
        };
        position.step = self.total_steps();
        position.fraction = 0.0;
        position.label = FINISHED_LABEL;
        self.emit(&mut position, None);
    }

    pub fn stage_reporter(&self, stage: Stage) -> impl Fn(f32, &str) + '_ {
        move |fraction, detail| self.within_detailed(stage, fraction, || Some(detail.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording_plan(stages: Vec<Stage>) -> (ProgressPlan, Arc<Mutex<Vec<Progress>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let reporter = ProgressReporter::new(move |progress| {
            sink.lock().expect("progress lock").push(progress);
        });
        (ProgressPlan::new(stages, Some(reporter)), seen)
    }

    #[test]
    fn only_planned_stages_report_and_never_move_backwards() {
        let (plan, seen) = recording_plan(vec![Stage::Types, Stage::Saving, Stage::WritingLog]);
        plan.begin(Stage::Moves);
        plan.begin(Stage::Types);
        plan.begin(Stage::Saving);
        std::thread::sleep(DETAIL_INTERVAL);
        plan.within(Stage::Saving, 0.5);
        plan.begin(Stage::Types);
        plan.begin(Stage::WritingLog);
        plan.finish();
        let seen = seen.lock().expect("progress lock");
        let steps: Vec<(usize, &str)> = seen
            .iter()
            .map(|progress| (progress.completed_steps, progress.label))
            .collect();
        assert_eq!(
            steps,
            vec![
                (0, Stage::Types.label()),
                (1, Stage::Saving.label()),
                (1, Stage::Saving.label()),
                (2, Stage::WritingLog.label()),
                (3, FINISHED_LABEL),
            ]
        );
        assert!((seen[2].fraction() - 0.5).abs() < f32::EPSILON);
        assert!(seen.last().expect("finished").is_finished());
        assert_eq!(seen.last().expect("finished").percent(), 100);
    }

    #[test]
    fn details_are_throttled_and_advance_the_stage() {
        let (plan, seen) = recording_plan(vec![Stage::Trainers, Stage::Saving]);
        plan.begin(Stage::Trainers);
        for trainer in 0..1000 {
            plan.detail(trainer, 1000, || format!("Trainer {} of 1000", trainer + 1));
        }
        std::thread::sleep(DETAIL_INTERVAL);
        plan.detail(500, 1000, || "Trainer 501 of 1000".to_owned());
        let seen = seen.lock().expect("progress lock");
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[1].detail.as_deref(), Some("Trainer 1 of 1000"));
        let last = seen.last().expect("detail");
        assert_eq!(last.detail.as_deref(), Some("Trainer 501 of 1000"));
        assert!((last.fraction() - 0.25).abs() < f32::EPSILON);
    }
}
