//! Item menus, item info windows, item/card artwork and cutin/book loading.
//!
//! Free functions shared by the packet and input dispatchers; they take the
//! interface and state explicitly so they stay testable without a `Client`.

use crate::*;

/// Open the character context menu for the given entity if its name is already
/// known. Returns `true` if the name still has to be requested from the map
/// server; the menu can then be opened once the name arrives.
pub(crate) fn open_character_context_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    entity_id: EntityId,
    position: ScreenPosition,
) -> bool {
    let character = state
        .follow(client_state().entities())
        .iter()
        .find(|entity| entity.get_entity_id() == entity_id)
        .and_then(|entity| {
            entity.get_details().map(|details| {
                (
                    details.split('#').next().unwrap_or_default().to_owned(),
                    entity.get_base_level().unwrap_or(0) as u16,
                    entity.get_guild_name().map(ToOwned::to_owned),
                    entity.get_vending_shop_name().map(ToOwned::to_owned),
                    match entity.get_entity_type() {
                        EntityType::Pet => Some(CompanionKind::Pet),
                        EntityType::Homunculus => Some(CompanionKind::Homunculus),
                        EntityType::Mercenary => Some(CompanionKind::Mercenary),
                        _ => None,
                    },
                )
            })
        })
        .filter(|(name, ..)| !name.is_empty());

    match character {
        Some((character_name, level, guild_name, vending_shop_name, companion_kind)) => {
            interface.close_window_with_class(WindowClass::ContextMenu);
            interface.open_window_at(
                CharacterContextMenuWindow::new(
                    character_name,
                    entity_id.0,
                    level,
                    guild_name,
                    vending_shop_name,
                    companion_kind,
                ),
                position,
            );

            false
        }
        None => true,
    }
}

/// Returns the localized label of an item category.
pub(crate) fn item_category_label(state: &State<ClientState>, category: ItemCategory) -> String {
    let localization = client_state().localization();

    match category {
        ItemCategory::Consumables => state.follow(localization.inventory_consumables_tab_text()).clone(),
        ItemCategory::Equipments => state.follow(localization.inventory_equipments_tab_text()).clone(),
        ItemCategory::Misc => state.follow(localization.inventory_misc_tab_text()).clone(),
    }
}

/// Open the context menu of the inventory item with the given inventory index
/// at the given screen position.
pub(crate) fn open_inventory_item_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    inventory_index: InventoryIndex,
    position: ScreenPosition,
) {
    let items = state.follow(client_state().inventory().items());
    let Some(item) = items.iter().find(|item| item.index == inventory_index) else {
        return;
    };

    let action = item_use_action(item);

    // While a Kafra storage window is open the same menu can store the item.
    let storage_action = match interface.is_window_with_class_open(WindowClass::Storage) {
        true => Some(ItemStorageAction::Deposit {
            inventory_index,
            amount: item_amount(item),
        }),
        false => None,
    };

    // Item service entries: identify needs an unidentified item, refine an
    // identified unequipped weapon below the maximum refine level, repair a
    // damaged (broken) item and card insertion a card. rAthena still validates
    // all of them server side, and refine/repair additionally require their
    // menu skill to be pending.
    let identified = item.is_identified();
    let refinement_level = match &item.details {
        ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
        ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
    };
    let broken = matches!(
        &item.details,
        ro_next_networking::InventoryItemDetails::Equippable { flags, .. } if flags.contains(EquippableItemFlags::IS_BROKEN)
    );
    let identify = !identified;
    let refine = item.item_type == item_type::WEAPON && identified && !item.is_equipped() && refinement_level < 10;
    let repair = broken;
    let insert_card = item.item_type == item_type::CARD;

    interface.close_window_with_class(WindowClass::ContextMenu);
    interface.open_window_at(
        ItemContextMenuWindow::new(inventory_index, item.item_id, action)
            .with_storage_action(storage_action)
            .with_service_actions(identify, refine, repair, insert_card),
        position,
    );
}

/// Open the context menu of a Kafra storage item.
pub(crate) fn open_storage_item_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    storage_index: InventoryIndex,
    position: ScreenPosition,
) {
    let items = state.follow(client_state().storage().items());
    let Some(item) = items.iter().find(|item| item.index == storage_index) else {
        return;
    };

    interface.close_window_with_class(WindowClass::ContextMenu);
    interface.open_window_at(
        ItemContextMenuWindow::new(storage_index, item.item_id, None)
            .with_source(ItemSource::Storage)
            .with_storage_action(Some(ItemStorageAction::Withdraw {
                storage_index,
                amount: item_amount(item),
            })),
        position,
    );
}

/// The amount of an [InventoryItem]: the stack size for regular items, one for
/// equipment and other non-stackable items.
pub(crate) fn item_amount(item: &ro_next_networking::InventoryItem<ResourceMetadata>) -> u32 {
    match &item.details {
        ro_next_networking::InventoryItemDetails::Regular { amount, .. } => (*amount).into(),
        ro_next_networking::InventoryItemDetails::Equippable { .. } => 1,
    }
}

/// Open a `<URL>` chat/dialog link with the system browser.
///
/// There is no browser crate in the dependency tree, so this uses the platform
/// opener. Only `http(s)` URLs are accepted and the URL is passed as a single
/// argument (no shell), so a malicious chat line cannot inject a command.
pub(crate) fn open_system_url(url: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        crate::logging::log_warn!("url: refusing to open non-http(s) link '{}'", url);
        return;
    }

    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };

    match result {
        Ok(_) => crate::logging::log_info!("url: opened '{}' in the system browser", url),
        Err(error) => crate::logging::log_warn!("url: failed to open '{}': {:?}", url, error),
    }
}

/// Open the info window of the inventory or Kafra storage item with the given
/// index.
///
/// The item info window shows the same details for both sources: the item
/// description, the refinement level, the slotted cards and the random
/// options (the storage rows carry the same packet data as the inventory
/// rows).
pub(crate) fn open_item_info_window(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    inventory_index: InventoryIndex,
    source: ItemSource,
    library: &Library,
    texture_loader: &TextureLoader,
) {
    let items = match source {
        // Equipped items are part of the inventory item list as well.
        ItemSource::Inventory | ItemSource::Equipment { .. } => state.follow(client_state().inventory().items()),
        ItemSource::Cart => state.follow(client_state().cart().items()),
        ItemSource::Storage => state.follow(client_state().storage().items()),
    };

    let Some(item) = items.iter().find(|item| item.index == inventory_index) else {
        return;
    };

    // The comparison pane shows the item currently equipped in the same slot.
    let equipped = match source {
        ItemSource::Inventory | ItemSource::Equipment { .. } => {
            equipped_item_in_slot(state.follow(client_state().inventory().items()), item)
        }
        // Items in the Kafra storage or the merchant cart are compared against
        // the inventory as well; those lists never contain equipped items.
        ItemSource::Storage | ItemSource::Cart => equipped_item_in_slot(state.follow(client_state().inventory().items()), item),
    };

    open_item_info_window_for_item(state, interface, item, equipped, library, texture_loader);
}

/// Open the item info window for an item that is not necessarily part of the
/// inventory (for example a `<ITEM>` link in chat).
pub(crate) fn open_item_info_window_for_item(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    item: &ro_next_networking::InventoryItem<ResourceMetadata>,
    equipped: Option<&ro_next_networking::InventoryItem<ResourceMetadata>>,
    library: &Library,
    texture_loader: &TextureLoader,
) {
    let category = item_category(item);
    let category_label = item_category_label(state, category);

    let artwork = load_item_artwork(library, texture_loader, item);
    let card_icons = load_item_card_icons(library, texture_loader, item);

    interface.close_window_with_class(WindowClass::ItemInfo);
    interface.open_window(ItemInfoWindow::new(item, category_label, artwork, card_icons, equipped));
}

/// Loads the large artwork (collection image) of an item for the item display
/// window. Falls back to the small inventory icon when the collection image is
/// missing.
pub(crate) fn load_item_artwork(
    library: &Library,
    texture_loader: &TextureLoader,
    item: &ro_next_networking::InventoryItem<ResourceMetadata>,
) -> Option<Arc<Texture>> {
    let resource = library.get::<ItemResource>(ItemResourceKey {
        item_id: item.item_id,
        is_identified: item.is_identified(),
    });
    let path = format!("유저인터페이스\\collection\\{resource}.bmp");

    texture_loader
        .get_or_load(&path, ImageType::Color)
        .ok()
        .or_else(|| item.metadata.texture.clone())
}

/// Loads the small card item icons of the item's card slots for the item
/// display window.
///
/// Unidentified items hide their cards, so no icons are loaded for them.
/// Unknown card ids resolve to `None` and the window draws a generic filled
/// slot instead.
pub(crate) fn load_item_card_icons(
    library: &Library,
    texture_loader: &TextureLoader,
    item: &ro_next_networking::InventoryItem<ResourceMetadata>,
) -> [Option<Arc<Texture>>; CARD_SLOT_COUNT] {
    if !item.is_identified() {
        return std::array::from_fn(|_| None);
    }

    std::array::from_fn(|slot| {
        let card_id = item.slot[slot];

        if card_id == 0 {
            return None;
        }

        let resource = library.get::<ItemResource>(ItemResourceKey {
            item_id: ItemId(card_id),
            is_identified: true,
        });
        let path = format!("유저인터페이스\\item\\{resource}.bmp");

        texture_loader.get_or_load(&path, ImageType::Color).ok()
    })
}

/// Candidate archive paths for the `cutin` / `ZC_SHOW_IMAGE2` image name.
///
/// The 2019 server sends the bare script name (for example `kafra_01`); the
/// classic client stores cut-ins under `data\texture\유저인터페이스\illust`.
/// Servers may also send a path with an extension, and NPC/monster portraits
/// (`monster\...`, `npc\...`) are accepted as a fallback.
pub(crate) fn cutin_texture_candidates(image_name: &str) -> Vec<String> {
    let name = image_name.trim().trim_end_matches('\0').trim();
    let name = name.replace('/', "\\");

    if name.is_empty() {
        return Vec::new();
    }

    let has_extension = [".bmp", ".png", ".tga", ".jpg", ".jpeg"]
        .iter()
        .any(|extension| name.to_ascii_lowercase().ends_with(extension));
    let has_directory = name.contains('\\');

    let mut candidates = Vec::new();

    // A full (or already `data\texture` relative) path wins over the inferred
    // locations.
    if has_directory || has_extension {
        candidates.push(name.clone());
    }

    let stem = name.trim_end_matches(".bmp").trim_end_matches(".png").trim_end_matches(".tga");

    for extension in ["bmp", "png", "tga"] {
        candidates.push(format!("유저인터페이스\\illust\\{stem}.{extension}"));
    }

    for directory in ["monster", "npc"] {
        candidates.push(format!("유저인터페이스\\illust\\{directory}\\{stem}.bmp"));
        candidates.push(format!("{directory}\\{stem}.bmp"));
    }

    candidates.push(format!("{stem}.bmp"));

    candidates.dedup();
    candidates
}

/// Load the NPC illustration of `ZC_SHOW_IMAGE2` (0x01B3) from the archive.
pub(crate) fn load_cutin_texture(
    game_file_loader: &GameFileLoader,
    texture_loader: &TextureLoader,
    image_name: &str,
) -> Option<Arc<Texture>> {
    for candidate in cutin_texture_candidates(image_name) {
        if game_file_loader.get(&format!("data\\texture\\{candidate}")).is_ok() {
            return texture_loader.get_or_load(&candidate, ImageType::Color).ok();
        }
    }

    None
}

/// Load and paginate `data\book\<book id>.txt`.
///
/// The classic book files are EUC-KR encoded; [`book_pages`] drops the leading
/// `%RRGGBB` background command and splits the remaining text into fixed size
/// pages. A missing file yields an empty page list, which the book window
/// renders as a clear placeholder.
pub(crate) fn load_book_pages(game_file_loader: &GameFileLoader, book_id: u32) -> Vec<Vec<String>> {
    let Ok(bytes) = game_file_loader.get(&format!("data\\book\\{book_id}.txt")) else {
        return Vec::new();
    };

    let (text, ..) = encoding_rs::EUC_KR.decode(&bytes);
    book_pages(&text)
}
