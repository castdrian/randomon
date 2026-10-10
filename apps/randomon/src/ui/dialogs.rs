use iced::widget::{
    Column, button, center, column, container, mouse_area, opaque, row, scrollable, space, text,
    toggler,
};
use iced::{Alignment, Element, Fill};
use randomon_settings::{CustomNames, GenRestrictions};

use crate::bundle;
use crate::capabilities::Capabilities;
use crate::ui::options_view::SEMIBOLD;
use crate::ui::style::{self, SPACE_1, SPACE_2, SPACE_3};

const HIGHEST_RESTRICTABLE_GENERATION: i32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationLimitDraft {
    pub restrictions: GenRestrictions,
    pub highest_generation: i32,
    pub is_xy: bool,
}

impl GenerationLimitDraft {
    pub fn open(current: Option<GenRestrictions>, caps: &Capabilities) -> Self {
        let highest_generation = caps
            .generation
            .unwrap_or(HIGHEST_RESTRICTABLE_GENERATION)
            .min(HIGHEST_RESTRICTABLE_GENERATION);
        let mut restrictions = current.unwrap_or_default();
        restrictions.limit_to_gen(highest_generation);
        let mut draft = Self {
            restrictions,
            highest_generation,
            is_xy: caps.force_swap_static_mega_evos,
        };
        draft.enforce_rules();
        draft
    }

    pub fn set_generation(&mut self, generation: i32, allowed: bool) {
        if let Some(flag) = generation_flag(&mut self.restrictions, generation) {
            *flag = allowed;
        }
        self.enforce_rules();
    }

    pub fn set_relatives(&mut self, allowed: bool) {
        self.restrictions.allow_evolutionary_relatives = allowed && self.relatives_enabled();
    }

    pub fn relatives_enabled(&self) -> bool {
        !self.restrictions.nothing_selected()
    }

    pub fn can_confirm(&self) -> bool {
        let restrictions = self.restrictions;
        !self.is_xy
            || restrictions.allow_gen1
            || restrictions.allow_gen2
            || restrictions.allow_gen3
            || restrictions.allow_gen4
    }

    fn enforce_rules(&mut self) {
        if !self.relatives_enabled() {
            self.restrictions.allow_evolutionary_relatives = false;
        }
    }

    fn is_allowed(&self, generation: i32) -> bool {
        let mut restrictions = self.restrictions;
        generation_flag(&mut restrictions, generation).is_some_and(|flag| *flag)
    }
}

fn generation_flag(restrictions: &mut GenRestrictions, generation: i32) -> Option<&mut bool> {
    match generation {
        1 => Some(&mut restrictions.allow_gen1),
        2 => Some(&mut restrictions.allow_gen2),
        3 => Some(&mut restrictions.allow_gen3),
        4 => Some(&mut restrictions.allow_gen4),
        5 => Some(&mut restrictions.allow_gen5),
        6 => Some(&mut restrictions.allow_gen6),
        7 => Some(&mut restrictions.allow_gen7),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamesTab {
    TrainerNames,
    TrainerClasses,
    DoublesTrainerNames,
    DoublesTrainerClasses,
    PokemonNicknames,
}

impl NamesTab {
    const ALL: [NamesTab; 5] = [
        NamesTab::TrainerNames,
        NamesTab::TrainerClasses,
        NamesTab::DoublesTrainerNames,
        NamesTab::DoublesTrainerClasses,
        NamesTab::PokemonNicknames,
    ];

    fn title(self) -> &'static str {
        bundle::label(match self {
            NamesTab::TrainerNames => {
                "CustomNamesEditorDialog.trainerNamesSP.TabConstraints.tabTitle"
            }
            NamesTab::TrainerClasses => {
                "CustomNamesEditorDialog.trainerClassesSP.TabConstraints.tabTitle"
            }
            NamesTab::DoublesTrainerNames => {
                "CustomNamesEditorDialog.doublesTrainerNamesSP.TabConstraints.tabTitle"
            }
            NamesTab::DoublesTrainerClasses => {
                "CustomNamesEditorDialog.doublesTrainerClassesSP.TabConstraints.tabTitle"
            }
            NamesTab::PokemonNicknames => {
                "CustomNamesEditorDialog.nicknamesSP.TabConstraints.tabTitle"
            }
        })
    }

    fn names(self, names: &CustomNames) -> &[String] {
        match self {
            NamesTab::TrainerNames => &names.trainer_names,
            NamesTab::TrainerClasses => &names.trainer_classes,
            NamesTab::DoublesTrainerNames => &names.doubles_trainer_names,
            NamesTab::DoublesTrainerClasses => &names.doubles_trainer_classes,
            NamesTab::PokemonNicknames => &names.pokemon_nicknames,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    GenerationLimits(GenerationLimitDraft),
    CustomNames(NamesTab),
}

#[derive(Debug, Clone, Copy)]
pub enum DialogMessage {
    SetGeneration(i32, bool),
    SetRelatives(bool),
    ConfirmGenerationLimits,
    ShowNames(NamesTab),
    Close,
}

pub fn modal<'a, Message: Clone + 'a>(
    dialog: Element<'a, Message>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    opaque(
        mouse_area(
            container(center(opaque(dialog)))
                .width(Fill)
                .height(Fill)
                .style(style::backdrop),
        )
        .on_press(on_dismiss),
    )
}

pub fn view<'a>(dialog: &Dialog, custom_names: &'a CustomNames) -> Element<'a, DialogMessage> {
    match dialog {
        Dialog::GenerationLimits(draft) => generation_limits(draft),
        Dialog::CustomNames(tab) => custom_names_preview(*tab, custom_names),
    }
}

fn generation_limits<'a>(draft: &GenerationLimitDraft) -> Element<'a, DialogMessage> {
    let generations = Column::with_children((1..=draft.highest_generation).map(|generation| {
        toggler(draft.is_allowed(generation))
            .label(format!("Generation {generation}"))
            .text_size(14)
            .size(18)
            .spacing(10)
            .style(style::toggle_switch)
            .on_toggle(move |allowed| DialogMessage::SetGeneration(generation, allowed))
            .into()
    }))
    .spacing(SPACE_1);

    let relatives = toggler(draft.restrictions.allow_evolutionary_relatives)
        .label("Allow Evolutionary Relatives")
        .text_size(14)
        .size(18)
        .spacing(10)
        .style(style::toggle_switch)
        .on_toggle_maybe(
            draft
                .relatives_enabled()
                .then_some(DialogMessage::SetRelatives),
        );

    let mut content = column![
        text(bundle::label("GenerationLimitDialog.title"))
            .size(20)
            .font(SEMIBOLD),
        text(bundle::label(
            "GenerationLimitDialog.includePokemonHeader.text"
        ))
        .size(13)
        .style(style::muted_text),
        generations,
        relatives,
    ]
    .spacing(SPACE_2);

    if draft.is_xy {
        content = content.push(
            container(text(bundle::label("GenerationLimitDialog.warningXYLabel.text")).size(12))
                .padding(10)
                .width(Fill)
                .style(style::warning_note),
        );
    }

    content = content.push(dialog_buttons(
        button(text(bundle::label("GenerationLimitDialog.okButton.text")))
            .padding([8, 20])
            .style(style::primary_button)
            .on_press_maybe(
                draft
                    .can_confirm()
                    .then_some(DialogMessage::ConfirmGenerationLimits),
            )
            .into(),
        Some(bundle::label("GenerationLimitDialog.cancelButton.text")),
    ));

    dialog_frame(content.into(), 380.0)
}

fn custom_names_preview<'a>(
    tab: NamesTab,
    custom_names: &'a CustomNames,
) -> Element<'a, DialogMessage> {
    let tabs = iced::widget::Row::with_children(NamesTab::ALL.iter().map(|candidate| {
        button(text(candidate.title()).size(13))
            .padding([6, 12])
            .style(style::segment(*candidate == tab, true))
            .on_press(DialogMessage::ShowNames(*candidate))
            .into()
    }))
    .spacing(2);
    let names = tab.names(custom_names);
    let list = scrollable(
        container(text(names.join(", ")).size(13))
            .padding(SPACE_2)
            .width(Fill),
    )
    .height(260);

    let content = column![
        text(bundle::label("CustomNamesEditorDialog.title"))
            .size(20)
            .font(SEMIBOLD),
        text("The built-in names used when trainer names, classes or nicknames are randomized. Editing and importing your own lists is coming in a later update.")
            .size(13)
            .style(style::muted_text),
        container(tabs).padding(3).style(style::segmented_track),
        text(format!("{} entries", names.len()))
            .size(12)
            .style(style::muted_text),
        container(list).style(style::subtle_card),
        dialog_buttons(
            button(text(bundle::label("CustomNamesEditorDialog.closeBtn.text")))
                .padding([8, 20])
                .style(style::primary_button)
                .on_press(DialogMessage::Close)
                .into(),
            None,
        ),
    ]
    .spacing(SPACE_2);

    dialog_frame(content.into(), 720.0)
}

fn dialog_buttons<'a>(
    primary: Element<'a, DialogMessage>,
    cancel_label: Option<&'static str>,
) -> Element<'a, DialogMessage> {
    let mut buttons = row![space::horizontal()]
        .spacing(SPACE_1)
        .align_y(Alignment::Center);
    if let Some(label) = cancel_label {
        buttons = buttons.push(
            button(text(label))
                .padding([8, 16])
                .style(style::secondary_button)
                .on_press(DialogMessage::Close),
        );
    }
    buttons.push(primary).into()
}

fn dialog_frame<'a>(content: Element<'a, DialogMessage>, width: f32) -> Element<'a, DialogMessage> {
    container(content)
        .padding(SPACE_3)
        .width(width)
        .style(style::dialog)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relatives_require_a_generation() {
        let mut draft = GenerationLimitDraft::open(None, &Capabilities::for_generation(3));
        draft.set_relatives(true);
        assert!(!draft.restrictions.allow_evolutionary_relatives);
        draft.set_generation(2, true);
        draft.set_relatives(true);
        assert!(draft.restrictions.allow_evolutionary_relatives);
        draft.set_generation(2, false);
        assert!(!draft.restrictions.allow_evolutionary_relatives);
    }

    #[test]
    fn opening_limits_restrictions_to_the_game_generation() {
        let current = GenRestrictions::from_state(0b111_1111);
        let draft = GenerationLimitDraft::open(Some(current), &Capabilities::for_generation(2));
        assert_eq!(draft.highest_generation, 2);
        assert!(draft.restrictions.allow_gen2);
        assert!(!draft.restrictions.allow_gen3);
    }

    #[test]
    fn xy_needs_an_early_generation() {
        let caps = Capabilities {
            force_swap_static_mega_evos: true,
            ..Capabilities::for_generation(6)
        };
        let mut draft = GenerationLimitDraft::open(None, &caps);
        draft.set_generation(6, true);
        assert!(!draft.can_confirm());
        draft.set_generation(4, true);
        assert!(draft.can_confirm());
    }
}
