//! commerce packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// Sent by the map server after the Vending skill (`MC_VENDING`) when the
/// player may open the shop creation UI (`ZC_OPENSTORE`, 0x012D).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x012D)]
#[packet_size(4)]
pub struct OpenStorePacket {
    /// Number of item slots the shop may hold (`2 + skill level`).
    pub slots: u16,
}

/// One item entry of the shop creation request.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct StoreItemEntry {
    pub inventory_index: u16,
    pub amount: u16,
    pub price: u32,
}

/// Sent by the client to create a shop (`CZ_REQ_OPENSTORE`, 0x012F).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x012F)]
#[variable_length]
pub struct RequestOpenStorePacket {
    #[length(80)]
    pub shop_name: String,
    #[repeating_remaining]
    pub items: Vec<StoreItemEntry>,
}

/// Sent by the client to cancel the shop creation UI (`CZ_REQ_OPENSTORE2`,
/// 0x01B2, result 0). rAthena clears its `prevend`/`workinprogress` state from
/// this packet, so the character cannot stay frozen after Vending.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01B2)]
#[variable_length]
pub struct CancelOpenStorePacket {
    #[length(80)]
    pub shop_name: String,
    pub result: u8,
    #[repeating_remaining]
    pub items: Vec<StoreItemEntry>,
}

/// Sent by the client to request a trade with another player
/// (`CZ_REQ_EXCHANGE_ITEM`, 0x00E4).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00E4)]
#[packet_size(6)]
pub struct RequestExchangeItemPacket {
    pub target_account_id: AccountId,
}

/// Sent by the client to accept/reject a trade request
/// (`CZ_ACK_EXCHANGE_ITEM`, 0x00E6). 3 = accept, 4 = reject.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00E6)]
#[packet_size(3)]
pub struct AnswerExchangeItemPacket {
    pub result: u8,
}

/// Sent by the client to offer an inventory item or zeny in a trade
/// (`CZ_ADD_EXCHANGE_ITEM`, 0x00E8). `inventory_index == 0` means zeny.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00E8)]
#[packet_size(8)]
pub struct AddExchangeItemPacket {
    pub inventory_index: u16,
    pub amount: u32,
}

/// Sent by the client to lock its side of the trade
/// (`CZ_CONCLUDE_EXCHANGE_ITEM`, 0x00EB).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00EB)]
#[packet_size(2)]
pub struct ConfirmExchangeItemPacket {}

/// Sent by the client to cancel the trade (`CZ_CANCEL_EXCHANGE_ITEM`, 0x00ED).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00ED)]
#[packet_size(2)]
pub struct CancelExchangeItemPacket {}

/// Sent by the client to commit the trade (`CZ_EXEC_EXCHANGE_ITEM`, 0x00EF).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00EF)]
#[packet_size(2)]
pub struct CommitExchangeItemPacket {}

/// `ZC_ADD_EXCHANGE_ITEM` (0x0A96 for PACKETVER 2019): the partner offered an
/// item or zeny in the current trade. `item_id == 0` means the amount is zeny.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A96)]
#[packet_size(61)]
pub struct ExchangeOfferPacket2019 {
    pub item_id: u32,
    pub item_type: u8,
    pub amount: u32,
    #[new_default]
    pub identified: u8,
    #[new_default]
    pub damaged: u8,
    #[new_default]
    pub refine: u8,
    #[new_default]
    pub cards: [u32; 4],
    #[new_default]
    pub options: [u8; 25],
    #[new_default]
    pub location: u32,
    #[new_default]
    pub look: u16,
}

/// 2019 era `ZC_ACK_ADD_EXCHANGE_ITEM` (0x00EA): trade window item ack.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00EA)]
#[packet_size(5)]
pub struct ExchangeAddItemPacket2019 {
    pub inventory_index: u16,
    pub result: u8,
}

/// 2019 era `ZC_CONCLUDE_EXCHANGE_ITEM` (0x00EC): trade was accepted by a side.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00EC)]
#[packet_size(3)]
pub struct ExchangeConcludePacket2019 {
    pub who: u8,
}

/// 2019 era `ZC_CANCEL_EXCHANGE_ITEM` (0x00EE): trade window was closed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00EE)]
#[packet_size(2)]
pub struct ExchangeCancelPacket2019 {}

/// 2019 era `ZC_EXEC_EXCHANGE_ITEM` (0x00F0): trade was executed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F0)]
#[packet_size(3)]
pub struct ExchangeExecPacket2019 {
    pub result: u8,
}

/// 2019 era `ZC_EXCHANGEITEM_UNDO` (0x00F1): the last trade item was undone.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F1)]
#[packet_size(2)]
pub struct ExchangeUndoPacket2019 {}

/// 2019 era `ZC_REQ_EXCHANGE_ITEM` (0x01F4): another player requests a trade.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01F4)]
#[packet_size(32)]
pub struct ExchangeRequestPacket2019 {
    #[length(24)]
    pub requester_name: String,
    pub target_account_id: AccountId,
    pub target_level: u16,
}

/// 2019 era `ZC_ACK_EXCHANGE_ITEM` (0x01F5): answer to a trade request.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01F5)]
#[packet_size(9)]
pub struct ExchangeAckPacket2019 {
    pub result: u8,
    pub target_account_id: AccountId,
    pub target_level: u16,
}

/// One entry of the item list of an NPC shop.
///
/// Layout for PACKETVER 2019 (MAIN < 20210203), i.e. the layout used by the
/// CondaRO server: `{ price u32, discount price u32, item type u8, item id u32
/// }`.
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct ShopItemInformation {
    pub price: Price,
    pub discount_price: Price,
    pub item_type: u8,
    pub item_id: ItemId,
}

/// Item list of an NPC shop (ZC_PC_PURCHASE_ITEMLIST, 2019 layout).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C6)]
#[variable_length]
pub struct ShopItemListPacket {
    #[repeating_remaining]
    pub items: Vec<ShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C4)]
#[packet_size(6)]
pub struct BuyOrSellPacket {
    pub shop_id: ShopId,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum BuyOrSellOption {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C5)]
pub struct SelectBuyOrSellPacket {
    pub shop_id: ShopId,
    pub option: BuyOrSellOption,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum BuyItemResult {
    #[numeric_value(0)]
    Successful,
    #[numeric_value(1)]
    NotEoughZeny,
    #[numeric_value(2)]
    WeightLimitExceeded,
    #[numeric_value(3)]
    TooManyItems,
    #[numeric_value(9)]
    TooManyOfThisItem,
    #[numeric_value(10)]
    PropsOpenAir,
    #[numeric_value(11)]
    ExchangeFailed,
    #[numeric_value(12)]
    ExchangeWellDone,
    #[numeric_value(13)]
    ItemSoldOut,
    #[numeric_value(14)]
    NotEnoughGoods,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct BuyItemInformation {
    pub amount: u16,
    pub item_id: ItemId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C8)]
#[variable_length]
pub struct BuyItemsPacket {
    #[repeating_remaining]
    pub items: Vec<BuyItemInformation>,
}

/// Result of a purchase at an NPC shop (ZC_PC_PURCHASE_RESULT, 2019 layout).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00CA)]
#[packet_size(3)]
pub struct BuyItemsResultPacket {
    pub result: BuyItemResult,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09D4)]
pub struct CloseShopPacket {}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct SellItemInformation {
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C7)]
#[variable_length]
pub struct SellListPacket {
    #[repeating_remaining]
    pub items: Vec<SellItemInformation>,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C9)]
#[variable_length]
pub struct SellItemsPacket {
    #[repeating_remaining]
    pub items: Vec<SoldItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum SellItemsResult {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00CB)]
#[packet_size(3)]
pub struct SellItemsResultPacket {
    pub result: SellItemsResult,
}

/// `CZ_REQ_BUY_FROMMC` (0x0130): ask the map server for another player's
/// vending shop list. `clif_parse_VendingListReq` (`clif.cpp:14128`) reads the
/// account id and answers with `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0130)]
#[packet_size(6)]
pub struct RequestVendingListPacket {
    pub owner_account_id: AccountId,
}

/// 2019 `ZC_STORE_ENTRY` (0x0131): a vending shop entered the player's view.
/// Sent by `clif_showvendingboard` (`clif.cpp:7558`) with the owner account id
/// and the 80 byte shop title.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0131)]
#[packet_size(86)]
pub struct StoreEntryPacket2019 {
    pub owner_account_id: AccountId,
    #[length(80)]
    pub store_name: String,
}

/// 2019 `ZC_DISAPPEAR_ENTRY` (0x0132): the vending shop of the player left the
/// view or was closed (`clif_closevendingboard`, `clif.cpp:7576`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0132)]
#[packet_size(6)]
pub struct StoreEntryRemovedPacket2019 {
    pub owner_account_id: AccountId,
}

/// One row of `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800) for PACKETVER
/// 20190605.
///
/// Layout from rAthena `packets_struct.hpp`
/// `PACKET_ZC_PC_PURCHASE_ITEMLIST_FROMMC_sub` (the `PACKETVER >= 20100105`
/// variant) as written by `clif_vendinglist` (`clif.cpp:7592`): 63 bytes per
/// row. `cart_index` is the seller's cart slot in the usual `slot + 2`
/// encoding, which is exactly what the purchase request has to echo back.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct VendingItemInformation2019 {
    pub price: u32,
    pub amount: u16,
    pub cart_index: InventoryIndex,
    pub item_type: u8,
    pub item_id: ItemId,
    pub identified: u8,
    pub damaged: u8,
    pub refinement_level: u8,
    pub card: [u32; 4],
    pub option_data: [ItemOptions; 5],
    pub location: u32,
    pub view_sprite: u16,
}

/// 2019 `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800): the item list of another
/// player's vending shop. rAthena sends `AID`, the shop's `vender_id` and one
/// [VendingItemInformation2019] per row (`clif.cpp:7592`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0800)]
#[variable_length]
pub struct VendingItemListPacket2019 {
    pub owner_account_id: AccountId,
    pub shop_id: u32,
    #[repeating_remaining]
    pub items: Vec<VendingItemInformation2019>,
}

/// One `{ <amount>.W <index>.W }` entry of a
/// `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134) request.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct PurchaseFromShopEntry {
    pub amount: u16,
    pub cart_index: InventoryIndex,
}

/// `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134): buy items from another player's
/// vending shop. The server reads `{ amount, index }` rows and matches the cart
/// slot index against the shop it last sent (`clif.cpp:14138`,
/// `vending.cpp:129`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0134)]
#[variable_length]
pub struct PurchaseFromShopPacket {
    pub owner_account_id: AccountId,
    #[repeating_remaining]
    pub items: Vec<PurchaseFromShopEntry>,
}

/// 2019 `ZC_PC_PURCHASE_RESULT_FROMMC` (0x0135): the vending purchase failed
/// (the server does not send a success ack; the item arrives through the
/// regular inventory packets). Result codes: 1 not enough zeny, 2 overweight,
/// 4 out of stock, 5 trading, 6 stale shop, 7 no sales info
/// (`clif.cpp:7642`, `clif.hpp:818`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0135)]
#[packet_size(7)]
pub struct VendingPurchaseResultPacket2019 {
    pub cart_index: InventoryIndex,
    pub amount: u16,
    pub result: u8,
}
