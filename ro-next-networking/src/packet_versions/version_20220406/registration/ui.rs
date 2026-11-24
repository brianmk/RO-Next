//! ui packet registrations.

use super::*;

pub(crate) fn register_ui<Callback>(packet_handler: &mut PacketHandler<NetworkEventList, Callback>) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: AchievementListPacket| NetworkEvent::AchievementList {
        achievements: packet.acheivement_data,
        total_score: packet.total_score,
        level: packet.level,
        experience: packet.acheivement_experience,
        experience_to_next_level: packet.acheivement_experience_to_next_level,
    })?;

    packet_handler.register(|packet: AchievementUpdatePacket| NetworkEvent::AchievementUpdate {
        achievement: packet.acheivement_data,
        total_score: packet.total_score,
        level: packet.level,
        experience: packet.acheivement_experience,
        experience_to_next_level: packet.acheivement_experience_to_next_level,
    })?;

    packet_handler.register(|packet: MarkMinimapPositionPacket| NetworkEvent::MinimapMarker {
        npc_id: packet.npc_id,
        marker_type: packet.marker_type,
        position: packet.position,
        id: packet.id,
        color: packet.color,
    })?;

    packet_handler.register(|packet: CloseButtonPacket| {
        let CloseButtonPacket { npc_id } = packet;

        NetworkEvent::AddCloseButton { npc_id }
    })?;

    packet_handler.register(|packet: DialogMenuPacket| {
        let DialogMenuPacket { npc_id, message } = packet;

        let choices = message.split(':').map(String::from).filter(|text| !text.is_empty()).collect();

        NetworkEvent::AddChoiceButtons { choices, npc_id }
    })?;

    packet_handler.register(|packet: QuestListPacket| NetworkEvent::QuestList { quests: packet.quests })?;

    packet_handler.register(|packet: QuestNotificationPacket1| {
        let objective_names = packet
            .objective_details
            .iter()
            .map(|objective| objective.mob_name.trim_end_matches('\0').trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();

        vec![
            NetworkEvent::QuestActive {
                quest_id: packet.quest_id,
                active: packet.active != 0,
            },
            NetworkEvent::QuestBanner {
                quest_id: packet.quest_id,
                active: packet.active != 0,
                objective_names,
            },
        ]
    })?;

    packet_handler.register(|packet: QuestRemovedPacket| NetworkEvent::QuestRemoved { quest_id: packet.quest_id })?;
    register_ignored_noop::<_, HuntingQuestNotificationPacket>(packet_handler)?;
    register_ignored_noop::<_, HuntingQuestUpdateObjectivePacket>(packet_handler)?;

    packet_handler.register(|packet: DisplayImagePacket| NetworkEvent::DisplayImage {
        image_name: packet.image_name,
        location: packet.location,
    })?;

    packet_handler.register(|packet: OpenEditDialogPacket| NetworkEvent::OpenEditDialog {
        npc_id: packet.npc_id,
        string_input: false,
    })?;

    packet_handler.register(|packet: OpenEditDialogStrPacket| NetworkEvent::OpenEditDialog {
        npc_id: packet.npc_id,
        string_input: true,
    })?;

    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;

    packet_handler.register(|packet: MailboxListPacket2019| NetworkEvent::MailList {
        mails: packet.entries.mails,
    })?;

    packet_handler.register(|packet: MailReadPacket2019| NetworkEvent::MailRead {
        mail_id: packet.mail_id,
        open_type: packet.open_type,
        body: packet.body,
        zeny: packet.zeny,
        items: packet.items.items,
    })?;

    packet_handler.register(|packet: MailDeleteAckPacket| NetworkEvent::MailDeleted { mail_id: packet.mail_id })?;

    packet_handler.register(|packet: MailboxWindowPacket| NetworkEvent::MailboxWindow {
        open: packet.window_type == 0,
    })?;

    packet_handler.register(|packet: OpenUserInterfacePacket| NetworkEvent::OpenUserInterface {
        ui_type: packet.ui_type,
        data: packet.data,
    })?;

    packet_handler.register(|_packet: OpenRefiningUiPacket| NetworkEvent::RefiningUiOpened)?;

    packet_handler.register(|packet: ReadBookPacket| NetworkEvent::OpenBook {
        book_id: packet.book_id,
        page: packet.page,
    })?;

    packet_handler.register(|packet: AchievementRewardAckPacket| NetworkEvent::AchievementRewardResult {
        achievement_id: packet.achievement_id,
        success: packet.result != 0,
    })?;

    Ok(())
}
