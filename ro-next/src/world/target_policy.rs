//! Player-versus-player targeting rules.
//!
//! The classic client decides whether a click on another player is an attack
//! in `CGameMode::HandleActorPick`/`CGameMode::ProcessActorPick`
//! (`rodecomp/Source/Client/Mode/GameMode.cpp`). The relevant map state is
//! built from the `ZC_MAPPROPERTY_R2` (0x099B) flags in
//! `CGameMode::ReceiveMapProperty`
//! (`rodecomp/Source/Client/Mode/GameModeNotices.cpp`) and the
//! `ZC_NOTIFY_MAPPROPERTY` (0x0199) PK/event-PvP/siege properties.
//!
//! The rules reproduced here are the player-target branch:
//!
//! * GvG/siege (`MF_GVG`/`MF_SIEGE`, map types 7/8/18/25): a player is
//!   attackable when it is neither the local player nor a related guild member.
//!   "Related" mirrors `GuildInfo::HasRelatedGuild`: the own guild and allied
//!   guilds (antagonist guilds stay attackable).
//! * PvP/PK (`MF_PVP`, `MF_NOLOCKON`, map types 4/5/6/11/12/17/20): a player is
//!   attackable when it is neither the local player nor a party member.
//! * Battleground (`MF_BATTLEFIELD`, map type 19): the official client uses the
//!   battleground team. This client does not track the battleground team yet,
//!   so it falls back to the PvP party rule (the closest rule that can be
//!   evaluated from the state the client already has).
//! * Everywhere else (`Peaceful`): players cannot be attacked. The official
//!   client only offers the shift-click attack here, and the server rejects it;
//!   this client has no shift-click force-attack path, so it never dispatches
//!   the attack.
//!
//! Monsters are not covered by this module: the official client makes them
//! attackable on every map (`IsMonsterClass` branch of `HandleActorPick`), and
//! only excludes the local player's own homunculus/pet. The client keeps that
//! behavior.

use crate::state::{MAP_FLAG_BATTLEFIELD, MAP_FLAG_DISABLE_LOCKON, MAP_FLAG_GVG, MAP_FLAG_PVP, MAP_FLAG_SIEGE};

/// `MAPTYPE_BATTLEFIELD` (`ZC_NOTIFY_MAPPROPERTY2`, rAthena `e_map_type`).
const MAP_TYPE_BATTLEFIELD: u16 = 19;
/// Arena, free-PK, event GvG, PK/PvP server, Poring Battle and PvP tournament.
const PVP_MAP_TYPES: &[u16] = &[4, 5, 6, 11, 12, 17, 20];
/// Event GvG, castles/siege (including the low-level siege castle).
const GVG_MAP_TYPES: &[u16] = &[7, 8, 18, 25];

/// How the current map treats player-versus-player attacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCombatZone {
    /// Players cannot attack each other.
    Peaceful,
    /// Party members (and the local player) are friendly.
    Pvp,
    /// Guild members and allied guilds (and the local player) are friendly.
    Gvg,
    /// Battleground; the official client uses the team, this client falls
    /// back to the PvP rule until the team is tracked.
    Battleground,
}

/// What the client knows about the local player/target pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TargetRelations {
    pub is_self: bool,
    pub is_party_member: bool,
    pub is_related_guild: bool,
    pub is_dead: bool,
}

/// Why a player target is or is not attackable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAttackDecision {
    Allowed,
    /// The map has no player-versus-player combat.
    PeacefulMap,
    /// The target is the local player.
    SelfTarget,
    /// The target is in the local player's party (PvP/PK rule).
    PartyMember,
    /// The target is in the local player's guild or an allied guild (GvG
    /// rule).
    RelatedGuild,
    /// Dead players are never valid attack targets (`actor->render.state`).
    DeadTarget,
}

impl PlayerAttackDecision {
    #[inline]
    pub fn is_allowed(self) -> bool {
        self == PlayerAttackDecision::Allowed
    }
}

/// Classifies the current map for player attacks.
///
/// The order matters: battleground maps also set the GvG/siege flags, and
/// castle/siege maps also set the GvG flag, exactly as the server builds the
/// `ZC_MAPPROPERTY_R2` bits.
pub fn player_combat_zone(map_type: u16, map_flags: u32) -> PlayerCombatZone {
    if map_flags & MAP_FLAG_BATTLEFIELD != 0 || map_type == MAP_TYPE_BATTLEFIELD {
        return PlayerCombatZone::Battleground;
    }

    if map_flags & (MAP_FLAG_GVG | MAP_FLAG_SIEGE) != 0 || GVG_MAP_TYPES.contains(&map_type) {
        return PlayerCombatZone::Gvg;
    }

    if map_flags & (MAP_FLAG_PVP | MAP_FLAG_DISABLE_LOCKON) != 0 || PVP_MAP_TYPES.contains(&map_type) {
        return PlayerCombatZone::Pvp;
    }

    PlayerCombatZone::Peaceful
}

/// Applies the official player-target rule of a zone to the local
/// player/target relationship.
pub fn player_attack_decision(zone: PlayerCombatZone, relations: TargetRelations) -> PlayerAttackDecision {
    if relations.is_dead {
        return PlayerAttackDecision::DeadTarget;
    }

    if relations.is_self {
        return PlayerAttackDecision::SelfTarget;
    }

    match zone {
        PlayerCombatZone::Peaceful => PlayerAttackDecision::PeacefulMap,
        PlayerCombatZone::Pvp | PlayerCombatZone::Battleground => match relations.is_party_member {
            true => PlayerAttackDecision::PartyMember,
            false => PlayerAttackDecision::Allowed,
        },
        PlayerCombatZone::Gvg => match relations.is_related_guild {
            true => PlayerAttackDecision::RelatedGuild,
            false => PlayerAttackDecision::Allowed,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerAttackDecision, PlayerCombatZone, TargetRelations, player_attack_decision, player_combat_zone};
    use crate::state::{
        MAP_FLAG_BATTLEFIELD, MAP_FLAG_COUNT_PK, MAP_FLAG_DISABLE_COSTUME, MAP_FLAG_DISABLE_LOCKON, MAP_FLAG_GVG, MAP_FLAG_PVP,
        MAP_FLAG_SIEGE,
    };

    fn relation(is_self: bool, is_party_member: bool, is_related_guild: bool) -> TargetRelations {
        TargetRelations {
            is_self,
            is_party_member,
            is_related_guild,
            is_dead: false,
        }
    }

    #[test]
    fn map_flags_select_the_official_zone() {
        assert_eq!(player_combat_zone(0, 0), PlayerCombatZone::Peaceful);
        assert_eq!(player_combat_zone(2, 0), PlayerCombatZone::Peaceful);
        assert_eq!(player_combat_zone(0, MAP_FLAG_PVP), PlayerCombatZone::Pvp);
        assert_eq!(player_combat_zone(0, MAP_FLAG_DISABLE_LOCKON), PlayerCombatZone::Pvp);
        assert_eq!(player_combat_zone(0, MAP_FLAG_GVG), PlayerCombatZone::Gvg);
        assert_eq!(player_combat_zone(0, MAP_FLAG_SIEGE), PlayerCombatZone::Gvg);
        assert_eq!(player_combat_zone(0, MAP_FLAG_BATTLEFIELD), PlayerCombatZone::Battleground);
        assert_eq!(player_combat_zone(19, 0), PlayerCombatZone::Battleground);
        // Battleground maps also carry the GvG/siege bits; the order must
        // still classify them as battleground.
        assert_eq!(
            player_combat_zone(19, MAP_FLAG_GVG | MAP_FLAG_SIEGE | MAP_FLAG_BATTLEFIELD),
            PlayerCombatZone::Battleground
        );
        // Counter and costume bits do not create a combat zone on their own.
        assert_eq!(player_combat_zone(0, MAP_FLAG_COUNT_PK), PlayerCombatZone::Peaceful);
        assert_eq!(player_combat_zone(0, MAP_FLAG_DISABLE_COSTUME), PlayerCombatZone::Peaceful);
        // Map types from rAthena e_map_type.
        for map_type in [4, 5, 6, 11, 12, 17, 20] {
            assert_eq!(player_combat_zone(map_type, 0), PlayerCombatZone::Pvp, "map type {map_type}");
        }
        for map_type in [7, 8, 18, 25] {
            assert_eq!(player_combat_zone(map_type, 0), PlayerCombatZone::Gvg, "map type {map_type}");
        }
    }

    #[test]
    fn peaceful_maps_never_allow_player_attacks() {
        for relations in [
            relation(false, false, false),
            relation(false, true, true),
            relation(true, false, false),
        ] {
            assert_eq!(
                player_attack_decision(PlayerCombatZone::Peaceful, relations),
                if relations.is_self {
                    PlayerAttackDecision::SelfTarget
                } else {
                    PlayerAttackDecision::PeacefulMap
                }
            );
        }
    }

    #[test]
    fn pvp_allows_non_party_players_only() {
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Pvp, relation(false, false, false)),
            PlayerAttackDecision::Allowed
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Pvp, relation(false, true, false)),
            PlayerAttackDecision::PartyMember
        );
        // The PvP rule ignores the guild relation, like the official branch.
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Pvp, relation(false, false, true)),
            PlayerAttackDecision::Allowed
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Pvp, relation(true, true, true)),
            PlayerAttackDecision::SelfTarget
        );
    }

    #[test]
    fn gvg_allows_non_guild_players_only() {
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Gvg, relation(false, false, false)),
            PlayerAttackDecision::Allowed
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Gvg, relation(false, false, true)),
            PlayerAttackDecision::RelatedGuild
        );
        // The GvG rule ignores party membership, like the siege branch of
        // `HandleActorPick`.
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Gvg, relation(false, true, false)),
            PlayerAttackDecision::Allowed
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Gvg, relation(true, false, true)),
            PlayerAttackDecision::SelfTarget
        );
    }

    #[test]
    fn battleground_falls_back_to_the_pvp_party_rule() {
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Battleground, relation(false, false, false)),
            PlayerAttackDecision::Allowed
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Battleground, relation(false, true, true)),
            PlayerAttackDecision::PartyMember
        );
    }

    #[test]
    fn dead_players_are_never_attackable() {
        let mut relations = relation(false, false, false);
        relations.is_dead = true;
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Pvp, relations),
            PlayerAttackDecision::DeadTarget
        );
        assert_eq!(
            player_attack_decision(PlayerCombatZone::Gvg, relations),
            PlayerAttackDecision::DeadTarget
        );
    }
}
