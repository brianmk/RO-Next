//! social packet registrations.

use super::*;

pub(crate) fn register_social<Callback>(packet_handler: &mut PacketHandler<NetworkEventList, Callback>) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: WhisperReceivedPacket| NetworkEvent::WhisperReceived {
        sender_name: packet.sender_name,
        message: packet.message.trim_end_matches('\0').to_owned(),
        is_admin: packet.is_admin != 0,
    })?;

    packet_handler.register(|packet: WhisperAckPacket| {
        let text = match packet.result {
            WhisperResult::Success => None,
            WhisperResult::TargetOffline => Some("The player is not online."),
            WhisperResult::Ignored => Some("The player is ignoring whispers."),
            WhisperResult::AllIgnored => Some("The player is not accepting whispers."),
        };

        text.map(|text| NetworkEvent::ChatMessage {
            text: text.to_owned(),
            color: MessageColor::Error,
        })
    })?;

    packet_handler.register(|packet: DisplayEmotionPacket| NetworkEvent::DisplayEmotion {
        entity_id: packet.entity_id,
        emotion: packet.emotion,
    })?;

    packet_handler.register(|packet: PartyMemberHealthPacket2019| NetworkEvent::PartyMemberHealth {
        account_id: packet.account_id,
        health_points: packet.health_points,
        maximum_health_points: packet.maximum_health_points,
    })?;
    register_ignored_noop::<_, ExchangeUndoPacket2019>(packet_handler)?;

    packet_handler.register(|packet: UpdatePartyInvitationStatePacket| NetworkEvent::PartyConfig {
        allow_invites: packet.allowed == 0,
    })?;

    packet_handler.register(|packet: GuildEmblemImagePacket| NetworkEvent::GuildEmblemImage {
        guild_id: packet.guild_id,
        emblem_id: packet.emblem_id,
        emblem_data: packet.emblem_data,
    })?;

    packet_handler.register(|packet: FriendListPacket2019| NetworkEvent::SetFriendList {
        friend_list: packet
            .friend_list
            .into_iter()
            .map(|friend| Friend {
                account_id: friend.account_id,
                character_id: friend.character_id,
                name: String::new(),
            })
            .collect(),
    })?;

    packet_handler.register(|packet: FriendRequestPacket| NetworkEvent::FriendRequest {
        requestee: packet.requestee,
    })?;

    packet_handler.register(|packet: FriendRequestResultPacket| {
        let text = match packet.result {
            FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
            FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
            FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
            FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
        };

        let mut events = vec![NetworkEvent::ChatMessage {
            text,
            color: MessageColor::Information,
        }];

        if matches!(packet.result, FriendRequestResult::Accepted) {
            events.push(NetworkEvent::FriendAdded { friend: packet.friend });
        }

        events
    })?;

    packet_handler.register(|packet: NotifyFriendRemovedPacket| NetworkEvent::FriendRemoved {
        account_id: packet.account_id,
        character_id: packet.character_id,
    })?;

    packet_handler.register(|packet: PartyOptionPacket2019| NetworkEvent::PartyOption {
        experience_option: packet.experience_option,
        item_pickup: packet.item_pickup,
        item_share: packet.item_share,
    })?;

    packet_handler.register(|packet: PartyListPacket2019| NetworkEvent::PartyList {
        party_name: packet.party_name,
        members: packet.members,
    })?;

    packet_handler.register(|packet: AddPartyMemberPacket2019| NetworkEvent::PartyMemberAdded {
        member: crate::PartyMemberDetails {
            account_id: packet.account_id,
            character_id: packet.character_id,
            name: packet.player_name,
            party_name: packet.party_name,
            map_name: packet.map_name,
            job_id: packet.job_id,
            base_level: packet.base_level,
            position: packet.position,
            online: packet.offline == 0,
            leader: packet.role == 0,
        },
    })?;

    packet_handler.register(|packet: PartyInvitePacket| NetworkEvent::PartyInvite {
        party_id: packet.party_id,
        party_name: packet.party_name,
    })?;

    packet_handler.register(|packet: PartyInviteReplyPacket2019| NetworkEvent::PartyInviteReply {
        character_name: packet.character_name,
        result: packet.result,
    })?;

    packet_handler.register(|packet: PartyCreationResultPacket| NetworkEvent::PartyCreationResult { result: packet.result })?;

    packet_handler.register(|packet: RemovePartyMemberPacket2019| NetworkEvent::PartyMemberRemoved {
        account_id: packet.account_id,
        character_name: packet.character_name,
        result: packet.result,
    })?;

    packet_handler.register(|packet: PartyMemberPositionPacket2019| NetworkEvent::PartyMemberPosition {
        account_id: packet.account_id,
        position: packet.position,
    })?;

    packet_handler.register(|packet: PartyMemberJobLevelPacket2019| NetworkEvent::PartyMemberJobLevel {
        account_id: packet.account_id,
        job_id: packet.job_id,
        base_level: packet.base_level,
    })?;

    packet_handler.register(|packet: PartyMemberAlivePacket2019| NetworkEvent::PartyMemberAlive {
        account_id: packet.account_id,
        is_dead: packet.is_dead != 0,
    })?;

    packet_handler.register(|packet: GuildChatReceivedPacket| NetworkEvent::GuildChat {
        message: packet.message.trim_end_matches('\0').to_owned(),
    })?;

    packet_handler.register(|packet: GuildMenuInterfacePacket| NetworkEvent::GuildMenuInterface {
        menu_flags: packet.menu_flags,
    })?;

    packet_handler.register(|packet: GuildInfoPacket| NetworkEvent::GuildInfo { information: packet })?;

    packet_handler.register(|packet: GuildMemberManagerInfoPacket| NetworkEvent::GuildMembers { members: packet.members })?;

    packet_handler.register(|packet: GuildPositionNameInfoPacket| NetworkEvent::GuildPositionNames {
        positions: packet.positions,
    })?;

    packet_handler.register(|packet: GuildPositionInfoPacket| NetworkEvent::GuildPositionRights {
        positions: packet.positions,
    })?;

    packet_handler.register(|packet: GuildPositionChangedPacket| NetworkEvent::GuildPositionChanged {
        positions: packet.positions,
    })?;
    packet_handler
        .register(|packet: GuildMemberPositionChangedPacket| NetworkEvent::GuildMemberPositionChanged { members: packet.members })?;

    packet_handler.register(|packet: GuildNoticePacket| NetworkEvent::GuildNotice {
        subject: packet.subject,
        notice: packet.notice,
    })?;

    packet_handler.register(|packet: GuildAllianceInfoPacket| NetworkEvent::GuildAlliances {
        alliances: packet.alliances,
    })?;

    packet_handler.register(|packet: GuildMemberNamePacket| NetworkEvent::GuildMemberName {
        character_id: packet.character_id,
        name: packet.name,
    })?;

    packet_handler.register(|packet: GuildLeaveNoticePacket| NetworkEvent::GuildMemberRemoved {
        character_id: packet.character_id,
        expelled: false,
    })?;

    packet_handler.register(|packet: GuildInviteResultPacket| NetworkEvent::GuildInviteResult { result: packet.result })?;

    packet_handler.register(|packet: CreateChatRoomResultPacket| NetworkEvent::ChatRoomCreated { flag: packet.flag })?;

    packet_handler.register(|packet: ChatRoomNewEntryPacket| NetworkEvent::ChatRoomAnnounced {
        owner_id: packet.owner_id,
        chat_id: packet.chat_id,
        limit: packet.limit,
        users: packet.users,
        room_type: packet.room_type,
        title: packet.title,
    })?;

    packet_handler.register(|packet: DestroyChatRoomPacket| NetworkEvent::ChatRoomDestroyed { chat_id: packet.chat_id })?;

    packet_handler.register(|packet: EnterChatRoomPacket| NetworkEvent::ChatRoomEntered {
        chat_id: packet.chat_id,
        members: packet.members,
    })?;

    packet_handler.register(|packet: ChatRoomMemberNewEntryPacket| NetworkEvent::ChatRoomMemberJoined {
        count: packet.count,
        name: packet.name,
    })?;

    packet_handler.register(|packet: ChatRoomMemberExitPacket| NetworkEvent::ChatRoomMemberLeft {
        count: packet.count,
        name: packet.name,
        kicked: packet.kicked != 0,
    })?;

    packet_handler.register(|packet: ChangeChatRoomPacket| NetworkEvent::ChatRoomChanged {
        owner_id: packet.owner_id,
        chat_id: packet.chat_id,
        limit: packet.limit,
        users: packet.users,
        room_type: packet.flag,
        title: packet.title,
    })?;

    packet_handler.register(|packet: ChatRoomRoleChangePacket| NetworkEvent::ChatRoomRoleChanged {
        flag: packet.flag,
        name: packet.name,
    })?;

    packet_handler.register(
        |packet: PartyBookingSearchResultPacket2019| NetworkEvent::PartyBookingSearchResult {
            more_results: packet.more_results != 0,
            entries: packet.entries,
        },
    )?;
    packet_handler
        .register(|packet: PartyBookingDeleteResultPacket2019| NetworkEvent::PartyBookingDeleteResult { result: packet.result })?;

    packet_handler.register(|packet: PartyBookingInsertNotifyPacket2019| NetworkEvent::PartyBookingInserted { entry: packet.entry })?;

    packet_handler.register(|packet: PartyBookingUpdateNotifyPacket2019| NetworkEvent::PartyBookingUpdated {
        index: packet.index,
        jobs: packet.jobs,
    })?;

    packet_handler.register(|packet: PartyBookingDeleteNotifyPacket2019| NetworkEvent::PartyBookingDeleted { index: packet.index })?;

    packet_handler.register(
        |packet: PartyBookingJoinRequestPacket2019| NetworkEvent::PartyBookingJoinRequest {
            character_id: packet.character_id,
            account_id: packet.account_id,
            name: packet.name,
            position: packet.position,
        },
    )?;

    packet_handler.register(
        |packet: PartyBookingJoinResultPacket2019| NetworkEvent::PartyBookingJoinResult {
            player_name: packet.player_name,
            party_name: packet.party_name,
            account_id: packet.account_id,
            refused: packet.refused != 0,
        },
    )?;

    packet_handler.register(|packet: ClanInfoPacket| NetworkEvent::ClanInfo { info: packet })?;

    packet_handler.register(|packet: ClanOnlineCountPacket| NetworkEvent::ClanOnlineCount {
        online_members: packet.online_members,
        maximum_members: packet.maximum_members,
    })?;

    packet_handler.register(|packet: PetPropertyPacket2019| NetworkEvent::PetProperty { property: packet })?;

    packet_handler.register(|packet: HomunculusPropertyPacket2019| NetworkEvent::HomunculusProperty { property: packet })?;

    packet_handler.register(
        |packet: HomunculusParameterChangePacket2019| NetworkEvent::HomunculusParameterChanged {
            parameter: packet.parameter,
            value: packet.value,
        },
    )?;

    packet_handler.register(|packet: MercenaryInfoPacket2019| NetworkEvent::MercenaryInfo { info: packet })?;

    packet_handler.register(
        |packet: MercenaryParameterChangePacket2019| NetworkEvent::MercenaryParameterChanged {
            parameter: packet.parameter,
            value: packet.value,
        },
    )?;

    Ok(())
}
