//! Generated table of verified skill icon fallbacks.
//!
//! The icon set of this (trimmed/custom) data set only covers part of the skill
//! list: 988 of 1229 skills have an exact
//! `data\\sprite\\아이템\\<file_name>.spr`, the remaining 241 entries below are
//! verified existing icons (`.spr` **and** `.act`).
//!
//! Resolution order in `skill_icon.rs`: exact sprite -> this table -> generic
//! icon.
//!
//! Categories: explicit: 4, family: 213, npc-token-suffix: 20, variant: 4
//!
//! Generation details: `generate_skill_icon_alias.py` (kept untracked in the
//! clone). NPC_ skill copies are mapped to the icon of their player-skill
//! counterpart (e.g. `NPC_JACKFROST` -> `wl_jackfrost`); element-spirit, Doram
//! and pet skills that have no icon family at all use a documented neutral
//! fallback.

use ragnarok_packets::SkillId;

/// Icon paths are relative to `data\\sprite\\` (no `.spr`/`.act` extension).
pub const SKILL_ICON_ALIASES: &[(u16, &str)] = &[
    (96, r"아이템\rk_enchantblade"),       // BS_ENCHANTEDSTONE (explicit)
    (107, r"아이템\bs_hiltbinding"),       // BS_WEAPONRESEARCH (explicit)
    (291, r"아이템\nv_basic"),             // SA_MONOCELL (family:sa)
    (292, r"아이템\nv_basic"),             // SA_CLASSCHANGE (family:sa)
    (293, r"아이템\nv_basic"),             // SA_SUMMONMONSTER (family:sa)
    (294, r"아이템\nv_basic"),             // SA_REVERSEORCISH (family:sa)
    (295, r"아이템\nv_basic"),             // SA_DEATH (family:sa)
    (296, r"아이템\nv_basic"),             // SA_FORTUNE (family:sa)
    (297, r"아이템\nv_basic"),             // SA_TAMINGMONSTER (family:sa)
    (298, r"아이템\nv_basic"),             // SA_QUESTION (family:sa)
    (299, r"아이템\nv_basic"),             // SA_GRAVITY (family:sa)
    (300, r"아이템\nv_basic"),             // SA_LEVELUP (family:sa)
    (301, r"아이템\nv_basic"),             // SA_INSTANTDEATH (family:sa)
    (302, r"아이템\nv_basic"),             // SA_FULLRECOVERY (family:sa)
    (303, r"아이템\nv_basic"),             // SA_COMA (family:sa)
    (545, r"아이템\nv_basic"),             // MB_FIGHTING (family:mb)
    (546, r"아이템\nv_basic"),             // MB_NEUTRAL (family:mb)
    (547, r"아이템\nv_basic"),             // MB_TAIMING_PUTI (family:mb)
    (548, r"아이템\nv_basic"),             // MB_WHITEPOTION (family:mb)
    (549, r"아이템\nv_basic"),             // MB_MENTAL (family:mb)
    (550, r"아이템\nv_basic"),             // MB_CARDPITCHER (family:mb)
    (551, r"아이템\nv_basic"),             // MB_PETPITCHER (family:mb)
    (552, r"아이템\nv_basic"),             // MB_BODYSTUDY (family:mb)
    (553, r"아이템\nv_basic"),             // MB_BODYALTER (family:mb)
    (554, r"아이템\nv_basic"),             // MB_PETMEMORY (family:mb)
    (555, r"아이템\nv_basic"),             // MB_M_TELEPORT (family:mb)
    (556, r"아이템\nv_basic"),             // MB_B_GAIN (family:mb)
    (557, r"아이템\nv_basic"),             // MB_M_GAIN (family:mb)
    (558, r"아이템\nv_basic"),             // MB_MISSION (family:mb)
    (559, r"아이템\nv_basic"),             // MB_MUNAKKNOWLEDGE (family:mb)
    (560, r"아이템\nv_basic"),             // MB_MUNAKBALL (family:mb)
    (561, r"아이템\nv_basic"),             // MB_SCROLL (family:mb)
    (562, r"아이템\nv_basic"),             // MB_B_GATHERING (family:mb)
    (563, r"아이템\nv_basic"),             // MB_M_GATHERING (family:mb)
    (564, r"아이템\nv_basic"),             // MB_B_EXCLUDE (family:mb)
    (565, r"아이템\nv_basic"),             // MB_B_DRIFT (family:mb)
    (566, r"아이템\nv_basic"),             // MB_B_WALLRUSH (family:mb)
    (567, r"아이템\nv_basic"),             // MB_M_WALLRUSH (family:mb)
    (568, r"아이템\nv_basic"),             // MB_B_WALLSHIFT (family:mb)
    (569, r"아이템\nv_basic"),             // MB_M_WALLCRASH (family:mb)
    (570, r"아이템\nv_basic"),             // MB_M_REINCARNATION (family:mb)
    (571, r"아이템\nv_basic"),             // MB_B_EQUIP (family:mb)
    (572, r"아이템\nv_basic"),             // SL_DEATHKNIGHT (family:sl)
    (573, r"아이템\nv_basic"),             // SL_COLLECTOR (family:sl)
    (574, r"아이템\nv_basic"),             // SL_NINJA (family:sl)
    (575, r"아이템\nv_basic"),             // SL_GUNNER (family:sl)
    (576, r"아이템\am_twilight3"),         // AM_TWILIGHT4 (explicit)
    (578, r"아이템\su_basic_skill"),       // DE_BERSERKAIZER (family:de)
    (579, r"아이템\su_basic_skill"),       // DA_DARKPOWER (family:da)
    (580, r"아이템\su_basic_skill"),       // DE_PASSIVE (family:de)
    (581, r"아이템\su_basic_skill"),       // DE_PATTACK (family:de)
    (582, r"아이템\su_basic_skill"),       // DE_PSPEED (family:de)
    (583, r"아이템\su_basic_skill"),       // DE_PDEFENSE (family:de)
    (584, r"아이템\su_basic_skill"),       // DE_PCRITICAL (family:de)
    (585, r"아이템\su_basic_skill"),       // DE_PHP (family:de)
    (586, r"아이템\su_basic_skill"),       // DE_PSP (family:de)
    (587, r"아이템\su_basic_skill"),       // DE_RESET (family:de)
    (588, r"아이템\su_basic_skill"),       // DE_RANKING (family:de)
    (589, r"아이템\su_basic_skill"),       // DE_PTRIPLE (family:de)
    (590, r"아이템\su_basic_skill"),       // DE_ENERGY (family:de)
    (591, r"아이템\su_basic_skill"),       // DE_NIGHTMARE (family:de)
    (592, r"아이템\su_basic_skill"),       // DE_SLASH (family:de)
    (593, r"아이템\su_basic_skill"),       // DE_COIL (family:de)
    (594, r"아이템\su_basic_skill"),       // DE_WAVE (family:de)
    (595, r"아이템\su_basic_skill"),       // DE_REBIRTH (family:de)
    (596, r"아이템\su_basic_skill"),       // DE_AURA (family:de)
    (597, r"아이템\su_basic_skill"),       // DE_FREEZER (family:de)
    (598, r"아이템\su_basic_skill"),       // DE_CHANGEATTACK (family:de)
    (599, r"아이템\su_basic_skill"),       // DE_PUNISH (family:de)
    (600, r"아이템\su_basic_skill"),       // DE_POISON (family:de)
    (601, r"아이템\su_basic_skill"),       // DE_INSTANT (family:de)
    (602, r"아이템\su_basic_skill"),       // DE_WARNING (family:de)
    (603, r"아이템\su_basic_skill"),       // DE_RANKEDKNIFE (family:de)
    (604, r"아이템\su_basic_skill"),       // DE_RANKEDGRADIUS (family:de)
    (605, r"아이템\su_basic_skill"),       // DE_GAUGE (family:de)
    (606, r"아이템\su_basic_skill"),       // DE_GTIME (family:de)
    (607, r"아이템\su_basic_skill"),       // DE_GPAIN (family:de)
    (608, r"아이템\su_basic_skill"),       // DE_GSKILL (family:de)
    (609, r"아이템\su_basic_skill"),       // DE_GKILL (family:de)
    (610, r"아이템\su_basic_skill"),       // DE_ACCEL (family:de)
    (611, r"아이템\su_basic_skill"),       // DE_BLOCKDOUBLE (family:de)
    (612, r"아이템\su_basic_skill"),       // DE_BLOCKMELEE (family:de)
    (613, r"아이템\su_basic_skill"),       // DE_BLOCKFAR (family:de)
    (614, r"아이템\su_basic_skill"),       // DE_FRONTATTACK (family:de)
    (615, r"아이템\su_basic_skill"),       // DE_DANGERATTACK (family:de)
    (616, r"아이템\su_basic_skill"),       // DE_TWINATTACK (family:de)
    (617, r"아이템\su_basic_skill"),       // DE_WINDATTACK (family:de)
    (618, r"아이템\su_basic_skill"),       // DE_WATERATTACK (family:de)
    (619, r"아이템\su_basic_skill"),       // DA_ENERGY (family:da)
    (620, r"아이템\su_basic_skill"),       // DA_CLOUD (family:da)
    (621, r"아이템\su_basic_skill"),       // DA_FIRSTSLOT (family:da)
    (622, r"아이템\su_basic_skill"),       // DA_HEADDEF (family:da)
    (623, r"아이템\su_basic_skill"),       // DA_SPACE (family:da)
    (624, r"아이템\su_basic_skill"),       // DA_TRANSFORM (family:da)
    (625, r"아이템\su_basic_skill"),       // DA_EXPLOSION (family:da)
    (626, r"아이템\su_basic_skill"),       // DA_REWARD (family:da)
    (627, r"아이템\su_basic_skill"),       // DA_CRUSH (family:da)
    (628, r"아이템\su_basic_skill"),       // DA_ITEMREBUILD (family:da)
    (629, r"아이템\su_basic_skill"),       // DA_ILLUSION (family:da)
    (630, r"아이템\su_basic_skill"),       // DA_NUETRALIZE (family:da)
    (631, r"아이템\su_basic_skill"),       // DA_RUNNER (family:da)
    (632, r"아이템\su_basic_skill"),       // DA_TRANSFER (family:da)
    (633, r"아이템\su_basic_skill"),       // DA_WALL (family:da)
    (635, r"아이템\su_basic_skill"),       // DA_REVENGE (family:da)
    (636, r"아이템\su_basic_skill"),       // DA_EARPLUG (family:da)
    (637, r"아이템\su_basic_skill"),       // DA_CONTRACT (family:da)
    (638, r"아이템\su_basic_skill"),       // DA_BLACK (family:da)
    (639, r"아이템\su_basic_skill"),       // DA_DREAM (family:da)
    (640, r"아이템\su_basic_skill"),       // DA_MAGICCART (family:da)
    (641, r"아이템\su_basic_skill"),       // DA_COPY (family:da)
    (642, r"아이템\su_basic_skill"),       // DA_CRYSTAL (family:da)
    (643, r"아이템\su_basic_skill"),       // DA_EXP (family:da)
    (644, r"아이템\su_basic_skill"),       // DA_CARTSWING (family:da)
    (645, r"아이템\su_basic_skill"),       // DA_REBUILD (family:da)
    (646, r"아이템\su_basic_skill"),       // DA_JOBCHANGE (family:da)
    (647, r"아이템\su_basic_skill"),       // DA_EDARKNESS (family:da)
    (648, r"아이템\su_basic_skill"),       // DA_EGUARDIAN (family:da)
    (649, r"아이템\su_basic_skill"),       // DA_TIMEOUT (family:da)
    (650, r"아이템\nv_basic"),             // ALL_TIMEIN (family:all)
    (651, r"아이템\su_basic_skill"),       // DA_ZENYRANK (family:da)
    (652, r"아이템\su_basic_skill"),       // DA_ACCESSORYMIX (family:da)
    (653, r"아이템\nv_basic"),             // NPC_EARTHQUAKE (family:npc)
    (659, r"아이템\nv_basic"),             // NPC_DRAGONFEAR (family:npc)
    (661, r"아이템\nv_basic"),             // NPC_PULSESTRIKE (family:npc)
    (662, r"아이템\nv_basic"),             // NPC_HELLJUDGEMENT (family:npc)
    (663, r"아이템\nv_basic"),             // NPC_WIDESILENCE (family:npc)
    (664, r"아이템\nv_basic"),             // NPC_WIDEFREEZE (family:npc)
    (665, r"아이템\nv_basic"),             // NPC_WIDEBLEEDING (family:npc)
    (666, r"아이템\nv_basic"),             // NPC_WIDESTONE (family:npc)
    (667, r"아이템\nv_basic"),             // NPC_WIDECONFUSE (family:npc)
    (668, r"아이템\nv_basic"),             // NPC_WIDESLEEP (family:npc)
    (669, r"아이템\nv_basic"),             // NPC_WIDESIGHT (family:npc)
    (670, r"아이템\nv_basic"),             // NPC_EVILLAND (family:npc)
    (671, r"아이템\nv_basic"),             // NPC_MAGICMIRROR (family:npc)
    (672, r"아이템\nv_basic"),             // NPC_SLOWCAST (family:npc)
    (673, r"아이템\nv_basic"),             // NPC_CRITICALWOUND (family:npc)
    (675, r"아이템\nv_basic"),             // NPC_STONESKIN (family:npc)
    (676, r"아이템\nv_basic"),             // NPC_ANTIMAGIC (family:npc)
    (677, r"아이템\nv_basic"),             // NPC_WIDECURSE (family:npc)
    (678, r"아이템\nv_basic"),             // NPC_WIDESTUN (family:npc)
    (679, r"아이템\nv_basic"),             // NPC_VAMPIRE_GIFT (family:npc)
    (680, r"아이템\nv_basic"),             // NPC_WIDESOULDRAIN (family:npc)
    (681, r"아이템\nv_basic"),             // ALL_INCCARRY (family:all)
    (683, r"아이템\nv_basic"),             // NPC_HELLPOWER (family:npc)
    (687, r"아이템\nv_basic"),             // NPC_ALLHEAL (family:npc)
    (693, r"아이템\nv_basic"),             // ALL_PARTYFLEE (family:all)
    (694, r"아이템\nv_basic"),             // ALL_ANGEL_PROTECT (family:all)
    (697, r"아이템\nv_basic"),             // ALL_REVERSEORCISH (family:all)
    (700, r"아이템\nv_basic"),             // NPC_WIDEHEALTHFEAR (family:npc)
    (701, r"아이템\nv_basic"),             // NPC_WIDEBODYBURNNING (family:npc)
    (702, r"아이템\nv_basic"),             // NPC_WIDEFROSTMISTY (family:npc)
    (703, r"아이템\nv_basic"),             // NPC_WIDECOLD (family:npc)
    (704, r"아이템\wm_lullaby_deepsleep"), // NPC_WIDE_DEEP_SLEEP (npc-token-suffix)
    (705, r"아이템\nv_basic"),             // NPC_WIDESIREN (family:npc)
    (706, r"아이템\nv_basic"),             // NPC_VENOMFOG (family:npc)
    (707, r"아이템\rk_millenniumshield"),  // NPC_MILLENNIUMSHIELD (npc-token-suffix)
    (708, r"아이템\nv_basic"),             // NPC_COMET (family:npc)
    (709, r"아이템\nv_basic"),             // NPC_ICEMINE (family:npc)
    (711, r"아이템\nv_basic"),             // NPC_FLAMECROSS (family:npc)
    (712, r"아이템\nv_basic"),             // NPC_PULSESTRIKE2 (family:npc)
    (716, r"아이템\nv_basic"),             // NPC_MAXPAIN (family:npc)
    (720, r"아이템\wl_jackfrost"),         // NPC_JACKFROST (npc-token-suffix)
    (721, r"아이템\nv_basic"),             // NPC_WIDEWEB (family:npc)
    (722, r"아이템\nv_basic"),             // NPC_WIDESUCK (family:npc)
    (723, r"아이템\nv_basic"),             // NPC_STORMGUST2 (family:npc)
    (724, r"아이템\nv_basic"),             // NPC_FIRESTORM (family:npc)
    (725, r"아이템\wm_reverberation"),     // NPC_REVERBERATION (npc-token-suffix)
    (727, r"아이템\lexaeterna"),           // NPC_LEX_AETERNA (npc-token-suffix)
    (730, r"아이템\sr_cursedcircle"),      // NPC_SR_CURSEDCIRCLE (npc-token-suffix)
    (731, r"아이템\rk_dragonbreath"),      // NPC_DRAGONBREATH (npc-token-suffix)
    (732, r"아이템\sc_fatalmenace"),       // NPC_FATALMENACE (npc-token-suffix)
    (733, r"아이템\nc_magma_eruption"),    // NPC_MAGMA_ERUPTION (npc-token-suffix)
    (735, r"아이템\gn_mandragora"),        // NPC_MANDRAGORA (npc-token-suffix)
    (736, r"아이템\so_psychic_wave"),      // NPC_PSYCHIC_WAVE (npc-token-suffix)
    (737, r"아이템\lg_rayofgenesis"),      // NPC_RAYOFGENESIS (npc-token-suffix)
    (739, r"아이템\so_cloud_kill"),        // NPC_CLOUD_KILL (npc-token-suffix)
    (740, r"아이템\rk_ignitionbreak"),     // NPC_IGNITIONBREAK (npc-token-suffix)
    (743, r"아이템\gc_hallucinationwalk"), // NPC_HALLUCINATIONWALK (npc-token-suffix)
    (744, r"아이템\so_electricwalk"),      // NPC_ELECTRICWALK (npc-token-suffix)
    (745, r"아이템\so_firewalk"),          // NPC_FIREWALK (npc-token-suffix)
    (750, r"아이템\nv_basic"),             // NPC_EARTHQUAKE_K (family:npc)
    (2055, r"아이템\ab_duplelight"),       // AB_DUPLELIGHT_MELEE (variant)
    (2056, r"아이템\ab_duplelight"),       // AB_DUPLELIGHT_MAGIC (variant)
    (2533, r"아이템\nv_basic"),            // ALL_ODINS_RECALL (family:all)
    (2539, r"아이템\ba_assassincross"),    // NPC_ASSASSINCROSS (npc-token-suffix)
    (2540, r"아이템\ba_dissonance"),       // NPC_DISSONANCE (npc-token-suffix)
    (2541, r"아이템\dc_uglydance"),        // NPC_UGLYDANCE (npc-token-suffix)
    (2543, r"아이템\nv_basic"),            // ALL_RAY_OF_PROTECTION (family:all)
    (2545, r"아이템\nv_basic"),            // GM_ITEM_ATKMAX (family:gm)
    (2546, r"아이템\nv_basic"),            // GM_ITEM_ATKMIN (family:gm)
    (2547, r"아이템\nv_basic"),            // GM_ITEM_MATKMAX (family:gm)
    (2548, r"아이템\nv_basic"),            // GM_ITEM_MATKMIN (family:gm)
    (2594, r"아이템\sj_fallingstar"),      // SJ_FALLINGSTAR_ATK (variant)
    (2595, r"아이템\sj_fallingstar"),      // SJ_FALLINGSTAR_ATK2 (variant)
    (3041, r"아이템\nv_basic"),            // ALL_NIFLHEIM_RECALL (family:all)
    (3043, r"아이템\nv_basic"),            // ALL_GLASTHEIM_RECALL (family:all)
    (3044, r"아이템\nv_basic"),            // ALL_THANATOS_RECALL (family:all)
    (8401, r"아이템\mg_firebolt"),         // EL_CIRCLE_OF_FIRE (family:el)
    (8402, r"아이템\mg_firebolt"),         // EL_FIRE_CLOAK (family:el)
    (8403, r"아이템\mg_firebolt"),         // EL_FIRE_MANTLE (family:el)
    (8404, r"아이템\mg_coldbolt"),         // EL_WATER_SCREEN (family:el)
    (8405, r"아이템\mg_coldbolt"),         // EL_WATER_DROP (family:el)
    (8406, r"아이템\mg_coldbolt"),         // EL_WATER_BARRIER (family:el)
    (8407, r"아이템\mg_lightningbolt"),    // EL_WIND_STEP (family:el)
    (8408, r"아이템\mg_lightningbolt"),    // EL_WIND_CURTAIN (family:el)
    (8409, r"아이템\mg_lightningbolt"),    // EL_ZEPHYR (family:el)
    (8410, r"아이템\wz_earthspike"),       // EL_SOLID_SKIN (family:el)
    (8411, r"아이템\wz_earthspike"),       // EL_STONE_SHIELD (family:el)
    (8412, r"아이템\wz_earthspike"),       // EL_POWER_OF_GAIA (family:el)
    (8413, r"아이템\mg_firebolt"),         // EL_PYROTECHNIC (family:el)
    (8414, r"아이템\mg_firebolt"),         // EL_HEATER (family:el)
    (8415, r"아이템\mg_coldbolt"),         // EL_TROPIC (family:el)
    (8416, r"아이템\mg_coldbolt"),         // EL_AQUAPLAY (family:el)
    (8417, r"아이템\mg_coldbolt"),         // EL_COOLER (family:el)
    (8418, r"아이템\mg_coldbolt"),         // EL_CHILLY_AIR (family:el)
    (8419, r"아이템\mg_lightningbolt"),    // EL_GUST (family:el)
    (8420, r"아이템\mg_firebolt"),         // EL_BLAST (family:el)
    (8421, r"아이템\mg_lightningbolt"),    // EL_WILD_STORM (family:el)
    (8422, r"아이템\wz_earthspike"),       // EL_PETROLOGY (family:el)
    (8423, r"아이템\wz_earthspike"),       // EL_CURSED_SOIL (family:el)
    (8424, r"아이템\wz_earthspike"),       // EL_UPHEAVAL (family:el)
    (8425, r"아이템\mg_firebolt"),         // EL_FIRE_ARROW (family:el)
    (8426, r"아이템\mg_firebolt"),         // EL_FIRE_BOMB (family:el)
    (8427, r"아이템\mg_firebolt"),         // EL_FIRE_BOMB_ATK (family:el)
    (8428, r"아이템\mg_firebolt"),         // EL_FIRE_WAVE (family:el)
    (8429, r"아이템\mg_firebolt"),         // EL_FIRE_WAVE_ATK (family:el)
    (8430, r"아이템\mg_coldbolt"),         // EL_ICE_NEEDLE (family:el)
    (8431, r"아이템\mg_coldbolt"),         // EL_WATER_SCREW (family:el)
    (8432, r"아이템\mg_coldbolt"),         // EL_WATER_SCREW_ATK (family:el)
    (8433, r"아이템\mg_coldbolt"),         // EL_TIDAL_WEAPON (family:el)
    (8434, r"아이템\mg_lightningbolt"),    // EL_WIND_SLASH (family:el)
    (8435, r"아이템\mg_lightningbolt"),    // EL_HURRICANE (family:el)
    (8436, r"아이템\mg_lightningbolt"),    // EL_HURRICANE_ATK (family:el)
    (8437, r"아이템\mg_lightningbolt"),    // EL_TYPOON_MIS (family:el)
    (8438, r"아이템\mg_lightningbolt"),    // EL_TYPOON_MIS_ATK (family:el)
    (8439, r"아이템\wz_earthspike"),       // EL_STONE_HAMMER (family:el)
    (8440, r"아이템\wz_earthspike"),       // EL_ROCK_CRUSHER (family:el)
    (8441, r"아이템\wz_earthspike"),       // EL_ROCK_CRUSHER_ATK (family:el)
    (8442, r"아이템\wz_earthspike"),       // EL_STONE_RAIN (family:el)
    (10015, r"아이템\gd_emergencycall"),   // GD_ITEMEMERGENCYCALL (explicit)
    (10016, r"아이템\nv_basic"),           // GD_GUILD_STORAGE (family:gd)
];

/// Generic fallback icons in preference order. All of them were verified to
/// exist.
pub const GENERIC_SKILL_ICONS: &[&str] = &[
    r"아이템\nv_basic",    // nv_basic
    r"아이템\sm_bash",     // sm_bash
    r"아이템\mg_firebolt", // mg_firebolt
];

/// Returns the verified fallback icon path for a skill id, if one exists.
pub fn alias(skill_id: SkillId) -> Option<&'static str> {
    SKILL_ICON_ALIASES
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| SKILL_ICON_ALIASES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_table_is_sorted_and_unique() {
        assert!(SKILL_ICON_ALIASES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(!SKILL_ICON_ALIASES.is_empty());
    }

    #[test]
    fn alias_paths_are_valid_icon_paths() {
        for (id, path) in SKILL_ICON_ALIASES {
            assert!(path.starts_with("아이템\\"), "bad folder for {id}: {path}");
            assert!(!path.ends_with(".spr") && !path.ends_with(".act"), "extension in {id}: {path}");
            assert!(!path.to_lowercase().contains("missing"), "missing placeholder in {id}: {path}");
        }
    }

    #[test]
    fn generic_icons_are_item_folder_icons() {
        assert!(!GENERIC_SKILL_ICONS.is_empty());
        for path in GENERIC_SKILL_ICONS {
            assert!(path.starts_with("아이템\\"), "bad generic icon: {path}");
        }
    }

    #[test]
    fn alias_lookup_uses_binary_search() {
        for (id, path) in SKILL_ICON_ALIASES {
            assert_eq!(alias(SkillId(*id)), Some(*path));
        }
        assert_eq!(alias(SkillId(65535)), None);
    }
}
