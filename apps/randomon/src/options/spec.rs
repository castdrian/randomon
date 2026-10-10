use randomon_settings::Settings;

use crate::bundle;
use crate::capabilities::Capabilities;
use crate::section::Section;

pub type Visibility = fn(&Capabilities) -> bool;
pub type Enablement = fn(&Settings, &Capabilities) -> bool;
pub type Reset = fn(&mut Settings, &Settings);
pub type FlagGetter = fn(&Settings) -> bool;
pub type FlagSetter = fn(&mut Settings, bool);
pub type NumberGetter = fn(&Settings) -> i32;
pub type NumberSetter = fn(&mut Settings, i32);
pub type Selector = fn(&mut Settings);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Bundle(&'static str),
    Plain(&'static str),
}

impl Text {
    pub fn label(self) -> &'static str {
        match self {
            Text::Bundle(key) => bundle::label(key),
            Text::Plain(value) => value,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Text::Bundle(key) => bundle::title(key),
            Text::Plain(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberStyle {
    Slider,
    Stepper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberFormat {
    Plain,
    Percent,
    SignedPercent,
    Level,
}

#[derive(Debug, Clone, Copy)]
pub struct Number {
    pub get: NumberGetter,
    pub set: NumberSetter,
    pub min: i32,
    pub max: i32,
    pub step: i32,
    pub style: NumberStyle,
    pub format: NumberFormat,
    pub help: Option<&'static str>,
}

impl Number {
    pub fn displayed(&self, settings: &Settings) -> i32 {
        (self.get)(settings).clamp(self.min, self.max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    EditGenerationLimits,
}

#[derive(Debug, Clone, Copy)]
pub enum Attachment {
    Number(Number),
    Pick(Pick),
    Action(Action),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    ExpCurve,
    BaseStatGeneration,
    MoveGeneration,
    Starter(usize),
    TrainerMode,
}

impl Pick {
    pub fn reset(self, settings: &mut Settings, defaults: &Settings) {
        match self {
            Pick::ExpCurve => settings.selected_exp_curve = defaults.selected_exp_curve,
            Pick::BaseStatGeneration => {
                settings.update_base_stats_to_generation = defaults.update_base_stats_to_generation;
            }
            Pick::MoveGeneration => {
                settings.update_moves_to_generation = defaults.update_moves_to_generation;
            }
            Pick::Starter(slot) => settings.custom_starters[slot] = defaults.custom_starters[slot],
            Pick::TrainerMode => settings.trainers_mod = defaults.trainers_mod,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Control {
    Toggle {
        get: FlagGetter,
        set: FlagSetter,
        attachment: Option<Attachment>,
    },
    Choice {
        get: FlagGetter,
        select: Selector,
    },
    Pick(Pick),
}

#[derive(Debug, Clone, Copy)]
pub struct Item {
    pub label: Text,
    pub help: Option<&'static str>,
    pub control: Control,
    pub visible: Visibility,
    pub enabled: Enablement,
    pub reset: Reset,
    pub nested: bool,
}

pub fn always(_: &Capabilities) -> bool {
    true
}

pub fn always_enabled(_: &Settings, _: &Capabilities) -> bool {
    true
}

pub fn never_reset(_: &mut Settings, _: &Settings) {}

impl Item {
    pub const fn new(
        label: Text,
        help: Option<&'static str>,
        control: Control,
        reset: Reset,
    ) -> Self {
        Self {
            label,
            help,
            control,
            visible: always,
            enabled: always_enabled,
            reset,
            nested: false,
        }
    }

    pub const fn shown_when(mut self, visible: Visibility) -> Self {
        self.visible = visible;
        self
    }

    pub const fn enabled_when(mut self, enabled: Enablement) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn nested(mut self) -> Self {
        self.nested = true;
        self
    }

    pub const fn with(mut self, attachment: Attachment) -> Self {
        if let Control::Toggle { get, set, .. } = self.control {
            self.control = Control::Toggle {
                get,
                set,
                attachment: Some(attachment),
            };
        }
        self
    }

    pub fn is_visible(&self, caps: &Capabilities) -> bool {
        (self.visible)(caps)
    }

    pub fn is_enabled(&self, settings: &Settings, caps: &Capabilities) -> bool {
        (self.enabled)(settings, caps)
    }

    pub fn is_selected(&self, settings: &Settings) -> bool {
        match self.control {
            Control::Toggle { get, .. } | Control::Choice { get, .. } => get(settings),
            Control::Pick(_) => false,
        }
    }

    pub fn help_text(&self) -> Option<&'static str> {
        self.help.and_then(bundle::help)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Radio {
    pub title: Option<Text>,
    pub choices: &'static [Item],
    pub reset: Reset,
    pub nested: bool,
}

impl Radio {
    pub const fn new(choices: &'static [Item], reset: Reset) -> Self {
        Self {
            title: None,
            choices,
            reset,
            nested: false,
        }
    }

    pub const fn titled(mut self, title: Text) -> Self {
        self.title = Some(title);
        self
    }

    pub const fn nested(mut self) -> Self {
        self.nested = true;
        self
    }

    pub fn is_visible(&self, caps: &Capabilities) -> bool {
        self.choices.iter().any(|choice| choice.is_visible(caps))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Row {
    Item(Item),
    Radio(Radio),
    Caption { text: Text, visible: Visibility },
}

impl Row {
    pub const fn caption(key: &'static str) -> Self {
        Row::Caption {
            text: Text::Bundle(key),
            visible: always,
        }
    }

    pub const fn caption_when(key: &'static str, visible: Visibility) -> Self {
        Row::Caption {
            text: Text::Bundle(key),
            visible,
        }
    }

    pub fn is_visible(&self, caps: &Capabilities) -> bool {
        match self {
            Row::Item(item) => item.is_visible(caps),
            Row::Radio(radio) => radio.is_visible(caps),
            Row::Caption { visible, .. } => visible(caps),
        }
    }

    pub fn reset(&self, settings: &mut Settings, defaults: &Settings) {
        match self {
            Row::Item(item) => {
                (item.reset)(settings, defaults);
                match item.control {
                    Control::Toggle {
                        attachment: Some(Attachment::Number(number)),
                        ..
                    } => (number.set)(settings, (number.get)(defaults)),
                    Control::Toggle {
                        attachment: Some(Attachment::Pick(pick)),
                        ..
                    }
                    | Control::Pick(pick) => pick.reset(settings, defaults),
                    _ => {}
                }
            }
            Row::Radio(radio) => (radio.reset)(settings, defaults),
            Row::Caption { .. } => {}
        }
    }

    pub fn hidden_labels(&self, caps: &Capabilities) -> Vec<&'static str> {
        match self {
            Row::Item(item) if !item.is_visible(caps) => vec![item.label.label()],
            Row::Radio(radio) => radio
                .choices
                .iter()
                .filter(|choice| !choice.is_visible(caps))
                .map(|choice| choice.label.label())
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardBody {
    Rows,
    MiscTweaks,
}

#[derive(Debug, Clone, Copy)]
pub struct Card {
    pub id: &'static str,
    pub section: Section,
    pub title: Text,
    pub summary: &'static str,
    pub visible: Visibility,
    pub unavailable_note: Option<&'static str>,
    pub rows: &'static [Row],
    pub body: CardBody,
}

impl Card {
    pub const fn new(
        id: &'static str,
        section: Section,
        title: Text,
        summary: &'static str,
        rows: &'static [Row],
    ) -> Self {
        Self {
            id,
            section,
            title,
            summary,
            visible: always,
            unavailable_note: None,
            rows,
            body: CardBody::Rows,
        }
    }

    pub const fn shown_when(mut self, visible: Visibility) -> Self {
        self.visible = visible;
        self
    }

    pub const fn unavailable_note(mut self, key: &'static str) -> Self {
        self.unavailable_note = Some(key);
        self
    }

    pub const fn misc_tweaks(mut self) -> Self {
        self.body = CardBody::MiscTweaks;
        self
    }

    pub fn is_visible(&self, caps: &Capabilities) -> bool {
        (self.visible)(caps)
    }
}
