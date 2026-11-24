//! Minimal pet / homunculus / mercenary information windows.
//!
//! The server only sends the status sheet of the *local player's* companion
//! (`ZC_PROPERTY_PET` 0x01A2, `ZC_PROPERTY_HOMUN` 0x09F7, `ZC_MER_INIT`
//! 0x029B). The window is opened from the entity context menu and matches the
//! received sheet to the clicked entity:
//!
//! * pet: same monster class id (`ZC_PROPERTY_PET` carries `job`),
//! * homunculus: same name (`ZC_PROPERTY_HOMUN` carries the name),
//! * mercenary: same block id (`ZC_MER_INIT` starts with the block id).
//!
//! Full pet/homunculus/mercenary AI, feeding, commands and skill trees are out
//! of scope; this is an information window only.

use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::EntityId;
use rust_state::{PathExt, Selector};

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Which companion sheet the window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionKind {
    Pet,
    Homunculus,
    Mercenary,
}

impl CompanionKind {
    fn title(self) -> &'static str {
        match self {
            Self::Pet => "Pet",
            Self::Homunculus => "Homunculus",
            Self::Mercenary => "Mercenary",
        }
    }
}

fn body_selector(entity_id: EntityId, kind: CompanionKind) -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(move |state: &ClientState| CompanionInfoWindow { entity_id, kind }.body_text(state))
}

pub struct CompanionInfoWindow {
    entity_id: EntityId,
    kind: CompanionKind,
}

impl CompanionInfoWindow {
    pub fn new(entity_id: EntityId, kind: CompanionKind) -> Self {
        Self { entity_id, kind }
    }

    fn body_text(&self, state: &ClientState) -> String {
        let companion = client_state().companion().follow_safe(state);
        let entity = client_state()
            .entities()
            .follow_safe(state)
            .iter()
            .find(|entity| entity.get_entity_id() == self.entity_id);

        match self.kind {
            CompanionKind::Pet => match &companion.pet {
                Some(pet) if entity.is_none_or(|entity| entity.get_job_id() == pet.job_id) => format!(
                    "Name: {}\nLevel: {}\nHP: not sent by the 2019 server (ZC_PROPERTY_PET 0x01A2 has no HP field)\nHunger: {}\nIntimacy: \
                     {}\nAccessory view id: {}",
                    pet.name, pet.level, pet.hunger, pet.intimacy, pet.accessory
                ),
                Some(_) => "The pet status packet describes the local player's own pet; this pet belongs to another player, so no status \
                            data is available."
                    .to_owned(),
                None => {
                    "No pet information received. The server only sends ZC_PROPERTY_PET (0x01A2) for the local player's own pet.".to_owned()
                }
            },
            CompanionKind::Homunculus => match &companion.homunculus {
                Some(homunculus)
                    if entity.is_none_or(|entity| entity.get_details().map(String::as_str) == Some(homunculus.name.as_str())) =>
                {
                    format!(
                        "Name: {}\nLevel: {}\nHP: {}\nSP: {}\nHunger: {}\nIntimacy: {}\nATK: {}  MATK: {}  HIT: {}  CRIT: {}\nDEF: {}  \
                         MDEF: {}  FLEE: {}  ASPD: {}\nEXP: {} / {}\nSkill points: {}\nStatus: {}{}",
                        homunculus.name,
                        homunculus.level,
                        homunculus.health_text(),
                        homunculus.spell_text(),
                        homunculus.hunger,
                        homunculus.intimacy,
                        homunculus.attack,
                        homunculus.magic_attack,
                        homunculus.hit,
                        homunculus.critical,
                        homunculus.defense,
                        homunculus.magic_defense,
                        homunculus.flee,
                        homunculus.attack_motion,
                        homunculus.experience,
                        homunculus.maximum_experience,
                        homunculus.skill_points,
                        if homunculus.dead { "dead" } else { "alive" },
                        if homunculus.vaporized { ", vaporized" } else { "" },
                    )
                }
                Some(_) => "The homunculus status packet describes the local player's own homunculus; this unit belongs to another \
                            player, so no status data is available."
                    .to_owned(),
                None => "No homunculus information received. The server only sends ZC_PROPERTY_HOMUN (0x09F7) for the local player's own \
                         homunculus."
                    .to_owned(),
            },
            CompanionKind::Mercenary => match &companion.mercenary {
                Some(mercenary) if mercenary.entity_id == self.entity_id => format!(
                    "Name: {}\nLevel: {}\nHP: {} / {}\nSP: {} / {}\nATK: {}  MATK: {}  HIT: {}  CRIT: {}\nDEF: {}  MDEF: {}  FLEE: {}  \
                     ASPD: {}\nFaith: {}\nCalls: {}  Kills: {}\nAttack range: {}",
                    mercenary.name,
                    mercenary.level,
                    mercenary.health_points,
                    mercenary.maximum_health_points,
                    mercenary.spell_points,
                    mercenary.maximum_spell_points,
                    mercenary.attack,
                    mercenary.magic_attack,
                    mercenary.hit,
                    mercenary.critical,
                    mercenary.defense,
                    mercenary.magic_defense,
                    mercenary.flee,
                    mercenary.attack_motion,
                    mercenary.faith,
                    mercenary.calls,
                    mercenary.kills,
                    mercenary.attack_range,
                ),
                Some(_) => "The mercenary status packet describes the local player's own mercenary; this unit belongs to another player, \
                            so no status data is available."
                    .to_owned(),
                None => "No mercenary information received. The server only sends ZC_MER_INIT (0x029B) for the local player's own \
                         mercenary."
                    .to_owned(),
            },
        }
    }
}

impl CustomWindow<ClientState> for CompanionInfoWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CompanionInfo)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let entity_id = self.entity_id;
        let kind = self.kind;
        let footer = "AI, commands, feeding and skill trees are out of scope for this client.";

        window! {
            title: kind.title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 360.0,
            maximum_width: 520.0,
            maximum_height: 560.0,
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                    text! {
                        text: body_selector(entity_id, kind),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: footer,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                ),
            },
        }
    }
}
