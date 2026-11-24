//! State of the item service windows: weapon refine, item repair, identify
//! and card composition.
//!
//! rAthena 20190605 drives all four services by *pushing* a list packet to the
//! client; the client answers with a selection packet:
//!
//! * refine: `ZC_NOTIFY_WEAPONITEMLIST` (0x0221) -> `CZ_REQ_WEAPONREFINE`
//!   (0x0222), result `ZC_ACK_WEAPONREFINE` (0x0223) / `ZC_ACK_ITEMREFINING`
//!   (0x0188),
//! * repair: `ZC_REPAIRITEMLIST` (0x01FC) -> `CZ_REQ_ITEMREPAIR1` (0x01FD),
//!   result `ZC_ACK_ITEMREPAIR` (0x01FE),
//! * identify: `ZC_ITEMIDENTIFY_LIST` (0x0177), sent by the merchant Item
//!   Appraisal skill, -> `CZ_REQ_ITEMIDENTIFY` (0x0178), result
//!   `ZC_ACK_ITEMIDENTIFY` (0x0179); the one-click request
//!   `CZ_REQ_ONECLICK_ITEMIDENTIFY` (0x0A35) skips both the skill and the list,
//! * card composition: `CZ_REQ_ITEMCOMPOSITION_LIST` (0x017A) ->
//!   `ZC_ITEMCOMPOSITION_LIST` (0x017B) -> `CZ_REQ_ITEMCOMPOSITION` (0x017C),
//!   result `ZC_ACK_ITEMCOMPOSITION` (0x017D).
//!
//! The four services share one selection window
//! ([`ItemServiceWindow`](crate::interface::windows::ItemServiceWindow)); this
//! state holds the list that the window renders and the last result line. The
//! state is filled by the network event handlers in `lib.rs` and is also what
//! the inventory context menu checks to decide which entries are available.
//!
//! In addition, the module holds the state of the separate NPC/atcommand refine
//! UI (`UIRefiningWnd`, [`RefiningUi`]), which is opened by
//! `ZC_OPEN_REFINING_UI` (0x0AA0) from the `refineui` script command and uses
//! its own window
//! ([`RefiningWindow`](crate::interface::windows::RefiningWindow)) because the
//! flow (pick item -> materials -> attempt) differs from the Blacksmith skill
//! refine list.

use ro_next_interface::element::StateElement;
use ragnarok_packets::{InventoryIndex, ItemId};
use rust_state::RustState;

/// Which item service the shared window currently shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, RustState, StateElement)]
pub enum ItemServiceKind {
    /// Weapons that the Blacksmith skill can refine.
    WeaponRefine,
    /// Damaged items that the Repair Weapon skill can repair.
    ItemRepair,
    /// Unidentified items that a Magnifier can appraise.
    ItemIdentify,
    /// Equipment that the selected card can be inserted into.
    CardComposition,
}

impl ItemServiceKind {
    /// Window title of the service.
    pub fn title(self) -> &'static str {
        match self {
            Self::WeaponRefine => "Weapon Refine",
            Self::ItemRepair => "Repair Item",
            Self::ItemIdentify => "Identify Item",
            Self::CardComposition => "Insert Card",
        }
    }

    /// Label of the per-row action button.
    pub fn action_label(self) -> &'static str {
        match self {
            Self::WeaponRefine => "Refine",
            Self::ItemRepair => "Repair",
            Self::ItemIdentify => "Identify",
            Self::CardComposition => "Insert",
        }
    }

    /// One line that explains where the list comes from. This is important
    /// because rAthena only accepts the answer while the matching menu skill is
    /// pending: the refine/repair/identify lists are sent by the server after
    /// the player used the skill (or, for identify, a Magnifier).
    pub fn hint(self) -> &'static str {
        match self {
            Self::WeaponRefine => "Weapons the server accepts for the blacksmith refine skill. The refine level shown is the current one.",
            Self::ItemRepair => "Damaged items that can be repaired with the Repair Weapon skill (a smith material is consumed).",
            Self::ItemIdentify => "Unidentified items. Identifying consumes one Magnifier per item.",
            Self::CardComposition => "Equipment the selected card fits into. Both items must be unequipped and identified.",
        }
    }
}

/// One selectable row of the item service window.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct ItemServiceEntry {
    /// Inventory slot of the item the action applies to.
    ///
    /// The repair list sends raw 0-based server slots and the other lists send
    /// the `slot + 2` client index; both decode to the same server slot here
    /// (see the packet documentation in `ragnarok-packets`).
    pub index: InventoryIndex,
    /// Item id, used for the repair request packet (rAthena ignores it, but the
    /// 25 byte request carries it).
    pub item_id: ItemId,
    /// Display name, resolved from the item database when the list arrived.
    pub name: String,
    /// Refine level of the item (`0` for items that cannot be refined).
    pub refine: u8,
}

impl ItemServiceEntry {
    pub fn new(index: InventoryIndex, item_id: ItemId, name: String, refine: u8) -> Self {
        Self {
            index,
            item_id,
            name,
            refine,
        }
    }

    /// Label of the row: the item name plus its refine level when it has one.
    pub fn label(&self) -> String {
        match self.refine {
            0 => self.name.clone(),
            refine => format!("{} +{}", self.name, refine),
        }
    }
}

/// One selectable material of the NPC refine UI
/// (`ZC_REFINING_MATERIAL_LIST` row, 0x0AA2).
#[derive(Debug, Clone, RustState, StateElement)]
pub struct RefiningMaterial {
    /// Item id that the refine request has to send.
    pub item_id: ItemId,
    /// Resolved display name of the material.
    pub name: String,
    /// Success chance in percent (`cost->chance / 100`).
    pub chance: u8,
    /// Zeny cost of the attempt.
    pub zeny: i32,
}

/// State of the NPC/atcommand refine UI (`UIRefiningWnd`).
///
/// Flow (rAthena `feature.refineui`, `clif.cpp:22475`-`22599`):
/// the server opens the UI with `ZC_OPEN_REFINING_UI` (0x0AA0), the client
/// sends the item with `CZ_REFINING_SELECT_ITEM` (0x0AA1), the server answers
/// with the material list `ZC_REFINING_MATERIAL_LIST` (0x0AA2) and the client
/// attempts the refine with `CZ_REQ_REFINING` (0x0AA3). The server re-sends the
/// material list after every attempt; `ZC_ACK_ITEMREFINING` (0x0188) carries
/// the new refine level. Closing sends `CZ_CLOSE_REFINING_UI` (0x0AA4).
#[derive(RustState, StateElement)]
pub struct RefiningUi {
    /// Whether the server opened the UI.
    pub open: bool,
    /// Items the player can pick. The server does not send a candidate list, so
    /// this is built from the inventory when the UI opens (identified,
    /// undamaged, unequipped weapons/armor).
    pub candidates: Vec<ItemServiceEntry>,
    /// Item that the material list belongs to.
    pub item: Option<ItemServiceEntry>,
    /// Materials the server offers for the selected item.
    pub materials: Vec<RefiningMaterial>,
    /// Number of Blacksmith Blessings the server requires (`0` = not usable).
    pub blacksmith_blessing: u8,
    /// Whether the player wants to protect the item with Blacksmith Blessings
    /// (only offered when the server requires some for the selected level).
    pub use_blacksmith_blessing: bool,
    /// Result line shown at the top of the window.
    pub status_message: String,
}

impl Default for RefiningUi {
    fn default() -> Self {
        Self {
            open: false,
            candidates: Vec::new(),
            item: None,
            materials: Vec::new(),
            blacksmith_blessing: 0,
            use_blacksmith_blessing: false,
            status_message: String::new(),
        }
    }
}

impl RefiningUi {
    /// The server opened the refine UI.
    pub fn start(&mut self, candidates: Vec<ItemServiceEntry>) {
        self.open = true;
        self.candidates = candidates;
        self.item = None;
        self.materials.clear();
        self.blacksmith_blessing = 0;
        self.use_blacksmith_blessing = false;
        self.status_message.clear();
    }

    /// The server sent the material list for the selected item.
    pub fn show_materials(&mut self, item: ItemServiceEntry, blacksmith_blessing: u8, materials: Vec<RefiningMaterial>) {
        self.open = true;
        self.item = Some(item);
        self.blacksmith_blessing = blacksmith_blessing;
        // Protection is on by default when the server offers it; the player can
        // switch it off in the window.
        self.use_blacksmith_blessing = blacksmith_blessing > 0;
        self.materials = materials;
        self.status_message.clear();
    }

    /// Close and forget the refine UI.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the result line of the window.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }
}

/// State of the shared item service window.
#[derive(RustState, StateElement)]
pub struct ItemService {
    /// Service whose list is currently shown, `None` while the window is
    /// closed.
    pub kind: Option<ItemServiceKind>,
    /// Rows of the current service list.
    pub entries: Vec<ItemServiceEntry>,
    /// Card that the player selected for composition (`CZ_REQ_ITEMCOMPOSITION`
    /// sends it together with the equipment the player picks).
    pub card: Option<ItemServiceEntry>,
    /// Last result of the server, shown at the top of the window.
    pub status_message: String,
    /// State of the NPC/atcommand refine UI (`UIRefiningWnd`).
    #[hidden_element]
    pub refining: RefiningUi,
}

impl Default for ItemService {
    fn default() -> Self {
        Self {
            kind: None,
            entries: Vec::new(),
            card: None,
            status_message: String::new(),
            refining: RefiningUi::default(),
        }
    }
}

impl ItemService {
    /// Replace the shown list with a new service list (opened by the server).
    pub fn show(&mut self, kind: ItemServiceKind, entries: Vec<ItemServiceEntry>) {
        self.kind = Some(kind);
        self.entries = entries;
        self.card = None;
        self.status_message.clear();
    }

    /// Show the equipment list for a card that the player wants to insert.
    pub fn show_card_list(&mut self, card: ItemServiceEntry, entries: Vec<ItemServiceEntry>) {
        self.kind = Some(ItemServiceKind::CardComposition);
        self.entries = entries;
        self.card = Some(card);
        self.status_message.clear();
    }

    /// Close the window state.
    pub fn clear(&mut self) {
        self.kind = None;
        self.entries.clear();
        self.card = None;
        self.status_message.clear();
    }

    /// Set the result line of the window.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: u16, refine: u8) -> ItemServiceEntry {
        ItemServiceEntry::new(InventoryIndex(index), ItemId(1500), format!("Item {index}"), refine)
    }

    #[test]
    fn label_appends_the_refine_level() {
        assert_eq!(entry(1, 0).label(), "Item 1");
        assert_eq!(entry(1, 7).label(), "Item 1 +7");
    }

    #[test]
    fn showing_a_list_replaces_the_previous_one() {
        let mut service = ItemService::default();
        service.show(ItemServiceKind::WeaponRefine, vec![entry(1, 0), entry(2, 3)]);
        assert_eq!(service.kind, Some(ItemServiceKind::WeaponRefine));
        assert_eq!(service.entries.len(), 2);
        assert_eq!(
            service
                .entries
                .iter()
                .find(|entry| entry.index == InventoryIndex(2))
                .unwrap()
                .refine,
            3
        );

        service.show_card_list(entry(9, 0), vec![entry(3, 0)]);
        assert_eq!(service.kind, Some(ItemServiceKind::CardComposition));
        assert!(service.card.is_some());
        assert!(!service.entries.iter().any(|entry| entry.index == InventoryIndex(1)));
    }

    fn material(item_id: u32, chance: u8) -> RefiningMaterial {
        RefiningMaterial {
            item_id: ItemId(item_id),
            name: format!("Material {item_id}"),
            chance,
            zeny: 1000,
        }
    }

    #[test]
    fn refining_ui_switches_from_candidates_to_materials() {
        let mut ui = RefiningUi::default();
        ui.start(vec![entry(1, 4), entry(2, 0)]);
        assert!(ui.open);
        assert_eq!(ui.candidates.len(), 2);
        assert!(ui.item.is_none());

        ui.show_materials(entry(1, 4), 2, vec![material(984, 75)]);
        assert_eq!(ui.item.as_ref().unwrap().index, InventoryIndex(1));
        assert_eq!(ui.blacksmith_blessing, 2);
        assert_eq!(ui.materials[0].chance, 75);

        ui.set_status("refined");
        ui.clear();
        assert!(!ui.open);
        assert!(ui.candidates.is_empty());
        assert!(ui.materials.is_empty());
        assert!(ui.status_message.is_empty());
    }

    #[test]
    fn clearing_removes_the_list_and_the_card() {
        let mut service = ItemService::default();
        service.show_card_list(entry(9, 0), vec![entry(3, 0)]);
        service.set_status("done");
        service.clear();

        assert!(service.kind.is_none());
        assert!(service.entries.is_empty());
        assert!(service.card.is_none());
        assert!(service.status_message.is_empty());
    }
}
