//! commerce packet registrations.

use super::*;

pub(crate) fn register_commerce<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: OpenStorePacket| NetworkEvent::OpenStore { slots: packet.slots })?;

    packet_handler.register(|_: ExchangeCancelPacket2019| NetworkEvent::TradeCancelled)?;

    packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;

    packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;

    packet_handler.register(|packet: StoreEntryPacket2019| NetworkEvent::StoreEntry {
        owner_account_id: packet.owner_account_id,
        shop_name: packet.store_name.trim().to_owned(),
    })?;

    packet_handler.register(|packet: StoreEntryRemovedPacket2019| NetworkEvent::StoreEntryRemoved {
        owner_account_id: packet.owner_account_id,
    })?;

    packet_handler.register(|packet: VendingPurchaseResultPacket2019| NetworkEvent::VendingPurchaseResult { result: packet.result })?;

    packet_handler.register(|packet: MailZenyClaimResultPacket| NetworkEvent::MailAttachmentClaimed {
        mail_id: packet.mail_id,
        zeny: true,
        result: packet.result,
    })?;

    Ok(())
}
