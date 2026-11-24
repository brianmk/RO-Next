//! Per-domain registration of the map-server packet handlers.
//!
//! `register_map_server_packets` creates the shared list buffers (the
//! inventory/storage/cart list packets arrive as start/item/end triples, so
//! the handlers share transient `Rc<RefCell<...>>` state) and calls one
//! registration function per domain.

use super::*;

mod commerce;
mod inventory;
mod misc;
mod session;
mod skills;
mod social;
mod ui;
mod world;

pub(crate) use commerce::register_commerce;
pub(crate) use inventory::register_inventory;
pub(crate) use misc::register_misc;
pub(crate) use session::register_session;
pub(crate) use skills::register_skills;
pub(crate) use social::register_social;
pub(crate) use ui::register_ui;
pub(crate) use world::register_world;

/// Inventory type byte of the shared list packets that belongs to the
/// normal player inventory.
pub(super) const INVENTORY_TYPE_INVENTORY: u8 = 0;
/// Inventory type byte of the Kafra storage list.
pub(super) const INVENTORY_TYPE_STORAGE: u8 = 2;
/// Inventory type byte of the merchant pushcart list (`INVTYPE_CART` in
/// rAthena `clif_cartlist`, `clif.cpp:3226`).
pub(super) const INVENTORY_TYPE_CART: u8 = 1;

/// Transient state shared by the list-packet handlers.
pub(super) struct ListBuffers {
    pub(super) inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>>,
    pub(super) storage_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>>,
    pub(super) storage_name: Rc<RefCell<String>>,
    pub(super) cart_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>>,
}

impl ListBuffers {
    fn new() -> Self {
        Self {
            inventory_items: Rc::new(RefCell::new(None)),
            storage_items: Rc::new(RefCell::new(None)),
            storage_name: Rc::new(RefCell::new(String::new())),
            cart_items: Rc::new(RefCell::new(None)),
        }
    }
}

/// Registers every map-server packet handler of this packet version.
pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    let buffers = ListBuffers::new();

    register_session(packet_handler)?;
    register_world(packet_handler)?;
    register_inventory(packet_handler, &buffers)?;
    register_skills(packet_handler)?;
    register_social(packet_handler)?;
    register_commerce(packet_handler)?;
    register_ui(packet_handler)?;
    register_misc(packet_handler, &buffers)?;
    Ok(())
}
