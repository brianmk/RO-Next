//! Data-driven paper-doll slot table.
//!
//! The equipment window draws the 11 equipment slots around the composited
//! character sprite. The slot geometry, the canvas size and the character
//! fitting constants used to be hardcoded in
//! [`super::paper_doll`]; they now live in
//! `archive/data/client/paper_doll_slots.ron`, embedded with `include_str!` as
//! the compiled fallback and loaded through [`GameFileLoader`] so a server can
//! override the layout in the client archive without rebuilding the client.
//!
//! The localized slot names/short labels stay in
//! [`crate::state::localization`] (`PaperDollTexts`, the P0 localization keys);
//! the table only selects which slot kind is drawn where. The slot kind maps to
//! the protocol [`EquipPosition`] used to find the equipped item.
//!
//! A malformed archive override is reported and the compiled table is used
//! instead, so a bad override can never leave the equipment window without
//! slots. [`validate`] (exposed as `korangar --check-paper-doll [PATH]`) is the
//! schema/drift guard.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::EquipPosition;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\paper_doll_slots.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/paper_doll_slots.ron");

/// Number of equipment slots of the paper doll.
///
/// This is a protocol shape, not a layout constant: a character wears at most
/// one item per `EquipPosition` of the classic equipment window (head top,
/// ammo, head middle, head lower, armor, garment, right/left hand, two
/// accessories and shoes). The data file has to define exactly these 11 slots.
pub const SLOT_COUNT: usize = 11;

/// Semantic kind of an equipment slot.
///
/// The variants map 1:1 to the protocol [`EquipPosition`] values that the
/// equipment window shows; the localized names/short labels are looked up from
/// [`crate::state::localization::PaperDollTexts`] by that position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum SlotKind {
    HeadTop,
    Ammo,
    HeadMiddle,
    HeadLower,
    Armor,
    Garment,
    RightHand,
    LeftHand,
    LeftAccessory,
    RightAccessory,
    Shoes,
}

impl SlotKind {
    /// The protocol equipment position of this slot.
    pub fn equip_position(self) -> EquipPosition {
        match self {
            SlotKind::HeadTop => EquipPosition::HEAD_TOP,
            SlotKind::Ammo => EquipPosition::AMMO,
            SlotKind::HeadMiddle => EquipPosition::HEAD_MIDDLE,
            SlotKind::HeadLower => EquipPosition::HEAD_LOWER,
            SlotKind::Armor => EquipPosition::ARMOR,
            SlotKind::Garment => EquipPosition::GARMENT,
            SlotKind::RightHand => EquipPosition::RIGHT_HAND,
            SlotKind::LeftHand => EquipPosition::LEFT_HAND,
            SlotKind::LeftAccessory => EquipPosition::LEFT_ACCESSORY,
            // The protocol enum spells the right accessory `RIGTH_ACCESSORY`;
            // keep the wire name and only use the descriptive data-file name.
            SlotKind::RightAccessory => EquipPosition::RIGTH_ACCESSORY,
            SlotKind::Shoes => EquipPosition::SHOES,
        }
    }
}

/// One equipment slot of the paper doll (data-file record).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotFile {
    kind: SlotKind,
    left: f32,
    top: f32,
}

/// Data file schema; the field names are the public data-file contract.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaperDollSlotsFile {
    #[allow(dead_code)]
    provenance: String,
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    canvas_width: f32,
    canvas_height: f32,
    slot_size: f32,
    character_max_scale: f32,
    character_top_margin: f32,
    slots: Vec<SlotFile>,
}

/// One validated equipment slot.
#[derive(Debug, Clone, Copy)]
pub struct SlotDefinition {
    pub kind: SlotKind,
    /// Offset of the slot rectangle inside the paper-doll canvas.
    pub left: f32,
    pub top: f32,
}

impl SlotDefinition {
    /// The protocol equipment position of this slot.
    pub fn equip_position(&self) -> EquipPosition {
        self.kind.equip_position()
    }
}

/// The validated paper-doll slot table.
#[derive(Debug)]
pub struct PaperDollSlots {
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub slot_size: f32,
    pub character_max_scale: f32,
    pub character_top_margin: f32,
    pub slots: Vec<SlotDefinition>,
}

impl PaperDollSlots {
    fn from_file(file: PaperDollSlotsFile) -> Self {
        Self {
            canvas_width: file.canvas_width,
            canvas_height: file.canvas_height,
            slot_size: file.slot_size,
            character_max_scale: file.character_max_scale,
            character_top_margin: file.character_top_margin,
            slots: file
                .slots
                .into_iter()
                .map(|slot| SlotDefinition {
                    kind: slot.kind,
                    left: slot.left,
                    top: slot.top,
                })
                .collect(),
        }
    }

    /// Number of slot rows (used by the `--check-paper-doll` output).
    fn count(&self) -> usize {
        self.slots.len()
    }

    /// Rejects data that parses but cannot render the classic 11-slot paper
    /// doll: wrong slot count, duplicates, invalid geometry.
    fn validate(&self) -> Result<(), String> {
        if !self.canvas_width.is_finite() || self.canvas_width <= 0.0 {
            return Err(format!("`canvas_width` must be positive and finite, got {}", self.canvas_width));
        }
        if !self.canvas_height.is_finite() || self.canvas_height <= 0.0 {
            return Err(format!(
                "`canvas_height` must be positive and finite, got {}",
                self.canvas_height
            ));
        }
        if !self.slot_size.is_finite() || self.slot_size <= 0.0 {
            return Err(format!("`slot_size` must be positive and finite, got {}", self.slot_size));
        }
        if !self.character_max_scale.is_finite() || self.character_max_scale <= 0.0 {
            return Err(format!(
                "`character_max_scale` must be positive and finite, got {}",
                self.character_max_scale
            ));
        }
        if !self.character_top_margin.is_finite() || self.character_top_margin < 0.0 || self.character_top_margin >= self.canvas_height {
            return Err(format!(
                "`character_top_margin` must be finite and inside the canvas, got {}",
                self.character_top_margin
            ));
        }

        if self.slots.len() != SLOT_COUNT {
            return Err(format!(
                "expected exactly {SLOT_COUNT} equipment slots (one per classic equipment position), got {}",
                self.slots.len()
            ));
        }

        for (index, slot) in self.slots.iter().enumerate() {
            if self.slots[..index].iter().any(|other| other.kind == slot.kind) {
                return Err(format!("slot kind {:?} is defined twice", slot.kind));
            }
            if !slot.left.is_finite() || !slot.top.is_finite() || slot.left < 0.0 || slot.top < 0.0 {
                return Err(format!(
                    "slot {:?} has an invalid position ({}, {}), positions must be finite and non-negative",
                    slot.kind, slot.left, slot.top
                ));
            }
            if slot.left + self.slot_size > self.canvas_width + f32::EPSILON {
                return Err(format!(
                    "slot {:?} does not fit into the canvas horizontally (left {} + slot size {} > canvas width {})",
                    slot.kind, slot.left, self.slot_size, self.canvas_width
                ));
            }
            if slot.top + self.slot_size > self.canvas_height + f32::EPSILON {
                return Err(format!(
                    "slot {:?} does not fit into the canvas vertically (top {} + slot size {} > canvas height {})",
                    slot.kind, slot.top, self.slot_size, self.canvas_height
                ));
            }
        }

        Ok(())
    }
}

fn parse(content: &str) -> Result<PaperDollSlots, String> {
    let file: PaperDollSlotsFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    let slots = PaperDollSlots::from_file(file);
    slots.validate()?;
    Ok(slots)
}

static COMPILED: OnceLock<PaperDollSlots> = OnceLock::new();
static ARCHIVE: OnceLock<PaperDollSlots> = OnceLock::new();

fn compiled() -> &'static PaperDollSlots {
    COMPILED.get_or_init(|| parse(FALLBACK).expect("the compiled paper-doll slot data has to be valid"))
}

/// The active paper-doll slot table.
pub fn table() -> &'static PaperDollSlots {
    ARCHIVE.get().unwrap_or_else(|| compiled())
}

/// Validates paper-doll slot data and returns the number of slots.
pub(crate) fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|slots| slots.count())
}

/// Validates the compiled fallback table.
pub(crate) fn validate_compiled() -> Result<usize, String> {
    parse(FALLBACK).map(|slots| slots.count())
}

/// Loads the paper-doll slot table from the client archive (or keeps the
/// compiled fallback).
///
/// Returns the number of loaded slots. A malformed data file is reported and
/// the compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!(
            "paper doll slots: loaded {} slots from the compiled fallback",
            compiled().count()
        );
        return compiled().count();
    };

    match parse(content) {
        Ok(slots) => {
            let count = slots.count();
            let _ = ARCHIVE.set(slots);
            crate::logging::log_debug!("paper doll slots: loaded {count} slots from {DATA_PATH}");
            count
        }
        Err(error) => {
            crate::logging::log_warn!("paper doll slots: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SLOT_COUNT, SlotKind, table, validate, validate_compiled};

    #[test]
    fn compiled_table_is_valid() {
        let count = validate_compiled().expect("the committed paper-doll slot data has to be valid");
        assert_eq!(count, SLOT_COUNT);
        assert_eq!(table().slots.len(), SLOT_COUNT);
    }

    #[test]
    fn defaults_match_the_former_hardcoded_layout() {
        let slots = table();
        assert_eq!(slots.canvas_width, 220.0);
        assert_eq!(slots.canvas_height, 342.0);
        assert_eq!(slots.slot_size, 42.0);
        assert_eq!(slots.character_max_scale, 3.2);
        assert_eq!(slots.character_top_margin, 6.0);

        // The slot kinds and their positions are the layout that used to be
        // hardcoded in `paper_doll.rs`; this pins the default/classic visuals.
        let expected = [
            (SlotKind::HeadTop, 89.0, 4.0),
            (SlotKind::Ammo, 166.0, 4.0),
            (SlotKind::HeadMiddle, 12.0, 52.0),
            (SlotKind::HeadLower, 166.0, 52.0),
            (SlotKind::Armor, 12.0, 126.0),
            (SlotKind::Garment, 166.0, 126.0),
            (SlotKind::RightHand, 12.0, 200.0),
            (SlotKind::LeftHand, 166.0, 200.0),
            (SlotKind::LeftAccessory, 12.0, 274.0),
            (SlotKind::RightAccessory, 166.0, 274.0),
            (SlotKind::Shoes, 89.0, 296.0),
        ];

        assert_eq!(slots.slots.len(), expected.len());
        for (slot, (kind, left, top)) in slots.slots.iter().zip(expected) {
            assert_eq!(slot.kind, kind);
            assert_eq!(slot.left, left);
            assert_eq!(slot.top, top);
        }
    }

    #[test]
    fn slot_kinds_map_to_the_protocol_equipment_positions() {
        use ragnarok_packets::EquipPosition;

        assert_eq!(SlotKind::HeadTop.equip_position(), EquipPosition::HEAD_TOP);
        assert_eq!(SlotKind::Ammo.equip_position(), EquipPosition::AMMO);
        assert_eq!(SlotKind::HeadMiddle.equip_position(), EquipPosition::HEAD_MIDDLE);
        assert_eq!(SlotKind::HeadLower.equip_position(), EquipPosition::HEAD_LOWER);
        assert_eq!(SlotKind::Armor.equip_position(), EquipPosition::ARMOR);
        assert_eq!(SlotKind::Garment.equip_position(), EquipPosition::GARMENT);
        assert_eq!(SlotKind::RightHand.equip_position(), EquipPosition::RIGHT_HAND);
        assert_eq!(SlotKind::LeftHand.equip_position(), EquipPosition::LEFT_HAND);
        assert_eq!(SlotKind::LeftAccessory.equip_position(), EquipPosition::LEFT_ACCESSORY);
        assert_eq!(SlotKind::RightAccessory.equip_position(), EquipPosition::RIGTH_ACCESSORY);
        assert_eq!(SlotKind::Shoes.equip_position(), EquipPosition::SHOES);
    }

    #[test]
    fn malformed_data_is_rejected() {
        let data = super::FALLBACK;

        assert!(validate("(slots: [)").is_err(), "broken RON must be rejected");
        assert!(
            validate(&data.replace(
                "(kind: HeadTop, left: 89.0, top: 4.0)",
                "(kind: HeadTop, left: 89.0, top: 4.0, typo: 1)"
            ))
            .is_err(),
            "unknown fields must be rejected"
        );
        assert!(
            validate(&data.replace("kind: HeadTop", "kind: NotASlot")).is_err(),
            "unknown slot kinds must be rejected"
        );
        assert!(
            validate(&data.replace(
                "(kind: HeadTop, left: 89.0, top: 4.0)",
                "(kind: HeadTop, left: 89.0, top: 4.0), (kind: HeadTop, left: 0.0, top: 0.0)"
            ))
            .is_err(),
            "duplicate slot kinds must be rejected"
        );
        assert!(
            validate(&data.replace("canvas_width: 220.0", "canvas_width: 0.0")).is_err(),
            "a zero canvas must be rejected"
        );
        assert!(
            validate(&data.replace(
                "(kind: HeadTop, left: 89.0, top: 4.0)",
                "(kind: HeadTop, left: 1000.0, top: 4.0)"
            ))
            .is_err(),
            "slots outside the canvas must be rejected"
        );
        assert!(
            validate(&data.replace("(kind: Shoes, left: 89.0, top: 296.0),", "")).is_err(),
            "a missing equipment slot must be rejected"
        );
    }

    #[test]
    fn archive_override_can_move_a_slot() {
        // An override only changes the table when it is valid; the moved slot
        // is used instead of the compiled one.
        let data = super::FALLBACK.replace("(kind: HeadTop, left: 89.0, top: 4.0)", "(kind: HeadTop, left: 88.0, top: 6.0)");
        let slots = super::parse(&data).expect("a valid override must parse");
        assert_eq!(slots.slots[0].left, 88.0);
        assert_eq!(slots.slots[0].top, 6.0);
        assert_eq!(slots.count(), SLOT_COUNT);
    }
}
