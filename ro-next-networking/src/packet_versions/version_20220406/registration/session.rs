//! session packet registrations.

use super::*;

pub(crate) fn register_session<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;

    packet_handler.register(|packet: SoundPacket| NetworkEvent::PlaySound {
        name: packet.name,
        act: packet.act,
        term: packet.term,
        account_id: packet.account_id,
    })?;

    packet_handler.register(|packet: BroadcastMessagePacket| {
        vec![
            NetworkEvent::ChatMessage {
                text: packet.message.clone(),
                color: MessageColor::Broadcast,
            },
            NetworkEvent::ServerAnnouncement { text: packet.message },
        ]
    })?;

    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        vec![
            NetworkEvent::ChatMessage {
                text: packet.message.clone(),
                color,
            },
            NetworkEvent::ServerAnnouncement { text: packet.message },
        ]
    })?;

    packet_handler.register(|packet: OverheadMessagePacket| {
        // 2019 `ZC_NOTIFY_CHAT` (0x008D): the message is both shown in the chat
        // window and above the speaking entity. The client decides whether an
        // overhead bubble is drawn (only player entities get one).
        let OverheadMessagePacket { entity_id, message } = packet;

        vec![
            NetworkEvent::ChatMessage {
                text: message.clone(),
                color: MessageColor::Broadcast,
            },
            NetworkEvent::OverheadChatMessage {
                entity_id,
                text: message,
                color: MessageColor::Broadcast,
            },
        ]
    })?;

    packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;

    packet_handler.register(|packet: MessageTablePacket| NetworkEvent::MessageString {
        message_id: packet.message_id,
    })?;

    packet_handler.register(|packet: EntityMessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;

    packet_handler.register(|packet: UserCountPacket2019| NetworkEvent::UserCount {
        player_count: packet.player_count,
    })?;

    packet_handler.register(|packet: RefuseEnterPacket2019| NetworkEvent::RefuseEnter {
        error_code: packet.error_code,
    })?;

    packet_handler.register(|packet: NotifyBanPacket| NetworkEvent::MapServerBan {
        error_code: packet.error_code,
    })?;

    packet_handler.register(|packet: UpdateHotkeysPacket| NetworkEvent::SetHotkeyData {
        tab: packet.tab,
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;

    packet_handler.register(|packet: MapServerAuthFailedPacket| NetworkEvent::MapServerAuthFailed { reason: packet.reason })?;

    packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;

    packet_handler.register(|packet: PartyChatMessagePacket2019| NetworkEvent::PartyChatMessage {
        account_id: packet.account_id,
        message: packet.message,
    })?;

    packet_handler.register(|packet: GuildBanNoticePacket| NetworkEvent::GuildMemberRemoved {
        character_id: packet.character_id,
        expelled: true,
    })?;

    packet_handler.register(|_: BankOpenAckPacket| NetworkEvent::BankOpened)?;

    packet_handler.register(|packet: BankBalancePacket| NetworkEvent::BankBalance {
        money: packet.money,
        reason: packet.reason,
    })?;

    packet_handler.register(|packet: BankDepositAckPacket| NetworkEvent::BankDepositResult {
        reason: packet.reason,
        money: packet.money,
        zeny: packet.zeny,
    })?;

    packet_handler.register(|packet: BankWithdrawAckPacket| NetworkEvent::BankWithdrawResult {
        reason: packet.reason,
        money: packet.money,
        zeny: packet.zeny,
    })?;

    packet_handler.register(|_: BankCloseAckPacket| NetworkEvent::BankClosed)?;

    packet_handler.register(|packet: RefuseEnterChatRoomPacket| NetworkEvent::ChatRoomEnterRefused { result: packet.result })?;

    Ok(())
}
