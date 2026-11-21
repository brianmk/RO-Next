//! Generated table of verified monster sprite fallbacks.
//!
//! The kRO data set used by this fork is partially trimmed: 228 of the 1008
//! rAthena mob ids have no exact `data\sprite\몬스터\<identity>.spr/.act` (and
//! a few exist only in `data\sprite\npc`). This table is generated from the
//! actual GRF file set by `generate_monster_sprite_alias.py`: every entry below
//! was verified to have **both** `.spr` and `.act` present.
//!
//! The runtime resolver in `monster_sprite.rs` still prefers the exact identity
//! path first (so the table is only consulted when the exact path does not
//! exist), then falls back to `npc\<identity>`, then to
//! [`GENERIC_MONSTER_SPRITES`].
//!
//! Generation categories: compact-monster: 2, family: 82, fuzzy: 2,
//! id-override: 7, variant-monster: 138, variant-npc: 1
//!
//! All 1008 mob ids resolve to an existing sprite; no id had to fall through to
//! the generic list, which stays as a safety net for unknown/custom ids.

use ragnarok_packets::JobId;

/// Sprite paths are relative to `data\sprite\` (no `.spr`/`.act` extension).
pub const MONSTER_SPRITE_ALIASES: &[(u16, &str)] = &[
    (1016, r"몬스터\skel_archer"),    // ARCHER_SKELETON (family)
    (1028, r"몬스터\skel_soldier"),   // SOLDIER_SKELETON (family)
    (1032, r"몬스터\mutant_verit"),   // VERIT (family)
    (1047, r"npc\pecopeco_egg"),      // PECOPECO_EGG (variant-npc)
    (1053, r"몬스터\thief_bug"),      // THIEF_BUG_ (variant-monster)
    (1054, r"몬스터\thief_bug"),      // THIEF_BUG__ (variant-monster)
    (1182, r"몬스터\red_mushroom"),   // THIEF_MUSHROOM (family)
    (1183, r"몬스터\chocho"),         // CHONCHON_ (variant-monster)
    (1184, r"몬스터\fabre"),          // FABRE_ (variant-monster)
    (1185, r"몬스터\whisper"),        // WHISPER_ (variant-monster)
    (1220, r"몬스터\desert_wolf"),    // M_DESERT_WOLF (variant-monster)
    (1221, r"몬스터\savage"),         // M_SAVAGE (variant-monster)
    (1229, r"몬스터\fabre"),          // META_FABRE (variant-monster)
    (1230, r"몬스터\pupa"),           // META_PUPA (variant-monster)
    (1231, r"몬스터\creamy"),         // META_CREAMY (variant-monster)
    (1232, r"몬스터\peco_egg"),       // META_PECOPECO_EGG (variant-monster)
    (1234, r"몬스터\yoyo"),           // PROVOKE_YOYO (variant-monster)
    (1235, r"몬스터\ork_warrior"),    // SMOKING_ORC (variant-monster)
    (1236, r"몬스터\ant_egg"),        // META_ANT_EGG (variant-monster)
    (1237, r"몬스터\andre"),          // META_ANDRE (variant-monster)
    (1238, r"몬스터\piere"),          // META_PIERE (variant-monster)
    (1239, r"몬스터\deniro"),         // META_DENIRO (variant-monster)
    (1240, r"몬스터\picky"),          // META_PICKY (variant-monster)
    (1241, r"몬스터\picky_"),         // META_PICKY_ (variant-monster)
    (1285, r"몬스터\beta_guards"),    // ARCHER_GUARDIAN (family)
    (1286, r"몬스터\beta_guards"),    // KNIGHT_GUARDIAN (family)
    (1287, r"몬스터\beta_guards"),    // SOLDIER_GUARDIAN (family)
    (1288, r"몬스터\s_empel_1"),      // EMPELIUM (family)
    (1324, r"npc\4_treasure_box"),    // TREASURE_BOX1 (family)
    (1325, r"npc\4_treasure_box"),    // TREASURE_BOX2 (family)
    (1326, r"npc\4_treasure_box"),    // TREASURE_BOX3 (family)
    (1327, r"npc\4_treasure_box"),    // TREASURE_BOX4 (family)
    (1328, r"npc\4_treasure_box"),    // TREASURE_BOX5 (family)
    (1329, r"npc\4_treasure_box"),    // TREASURE_BOX6 (family)
    (1330, r"npc\4_treasure_box"),    // TREASURE_BOX7 (family)
    (1331, r"npc\4_treasure_box"),    // TREASURE_BOX8 (family)
    (1332, r"npc\4_treasure_box"),    // TREASURE_BOX9 (family)
    (1333, r"npc\4_treasure_box"),    // TREASURE_BOX10 (family)
    (1334, r"npc\4_treasure_box"),    // TREASURE_BOX11 (family)
    (1335, r"npc\4_treasure_box"),    // TREASURE_BOX12 (family)
    (1336, r"npc\4_treasure_box"),    // TREASURE_BOX13 (family)
    (1337, r"npc\4_treasure_box"),    // TREASURE_BOX14 (family)
    (1338, r"npc\4_treasure_box"),    // TREASURE_BOX15 (family)
    (1339, r"npc\4_treasure_box"),    // TREASURE_BOX16 (family)
    (1340, r"npc\4_treasure_box"),    // TREASURE_BOX17 (family)
    (1341, r"npc\4_treasure_box"),    // TREASURE_BOX18 (family)
    (1342, r"npc\4_treasure_box"),    // TREASURE_BOX19 (family)
    (1343, r"npc\4_treasure_box"),    // TREASURE_BOX20 (family)
    (1344, r"npc\4_treasure_box"),    // TREASURE_BOX21 (family)
    (1345, r"npc\4_treasure_box"),    // TREASURE_BOX22 (family)
    (1346, r"npc\4_treasure_box"),    // TREASURE_BOX23 (family)
    (1347, r"npc\4_treasure_box"),    // TREASURE_BOX24 (family)
    (1348, r"npc\4_treasure_box"),    // TREASURE_BOX25 (family)
    (1349, r"npc\4_treasure_box"),    // TREASURE_BOX26 (family)
    (1350, r"npc\4_treasure_box"),    // TREASURE_BOX27 (family)
    (1351, r"npc\4_treasure_box"),    // TREASURE_BOX28 (family)
    (1352, r"npc\4_treasure_box"),    // TREASURE_BOX29 (family)
    (1353, r"npc\4_treasure_box"),    // TREASURE_BOX30 (family)
    (1354, r"npc\4_treasure_box"),    // TREASURE_BOX31 (family)
    (1355, r"npc\4_treasure_box"),    // TREASURE_BOX32 (family)
    (1356, r"npc\4_treasure_box"),    // TREASURE_BOX33 (family)
    (1357, r"npc\4_treasure_box"),    // TREASURE_BOX34 (family)
    (1358, r"npc\4_treasure_box"),    // TREASURE_BOX35 (family)
    (1359, r"npc\4_treasure_box"),    // TREASURE_BOX36 (family)
    (1360, r"npc\4_treasure_box"),    // TREASURE_BOX37 (family)
    (1361, r"npc\4_treasure_box"),    // TREASURE_BOX38 (family)
    (1362, r"npc\4_treasure_box"),    // TREASURE_BOX39 (family)
    (1363, r"npc\4_treasure_box"),    // TREASURE_BOX40 (family)
    (1420, r"몬스터\skel_archer"),    // G_ARCHER_SKELETON (family)
    (1502, r"몬스터\poring"),         // PORING_V (variant-monster)
    (1562, r"몬스터\skel_soldier"),   // G_SOLDIER_SKELETON (family)
    (1591, r"몬스터\lunatic"),        // A_LUNATIC (variant-monster)
    (1592, r"몬스터\mobster"),        // A_MOBSTER (variant-monster)
    (1593, r"몬스터\ancient_mummy"),  // A_ANCIENT_MUMMY (variant-monster)
    (1608, r"몬스터\thief_bug"),      // G_THIEF_BUG__ (variant-monster)
    (1610, r"몬스터\munak"),          // A_MUNAK (variant-monster)
    (1611, r"몬스터\bon_gun"),        // A_BON_GUN (variant-monster)
    (1612, r"몬스터\hyegun"),         // A_HYEGUN (variant-monster)
    (1631, r"몬스터\chung_e"),        // CHUNG_E_ (variant-monster)
    (1646, r"몬스터\seyren"),         // B_SEYREN (variant-monster)
    (1647, r"몬스터\eremes"),         // B_EREMES (variant-monster)
    (1648, r"몬스터\harword"),        // B_HARWORD (variant-monster)
    (1649, r"몬스터\magaleta"),       // B_MAGALETA (variant-monster)
    (1650, r"몬스터\shecil"),         // B_SHECIL (variant-monster)
    (1651, r"몬스터\katrinn"),        // B_KATRINN (variant-monster)
    (1658, r"몬스터\ygnizem"),        // B_YGNIZEM (variant-monster)
    (1722, r"몬스터\jakk"),           // EVENT_JAKK (variant-monster)
    (1723, r"몬스터\shecil"),         // A_SHECIL (variant-monster)
    (1724, r"몬스터\poton_canon"),    // A_POTON_CANON (variant-monster)
    (1725, r"몬스터\poring"),         // R_PORING (variant-monster)
    (1726, r"몬스터\lunatic"),        // R_LUNATIC (variant-monster)
    (1727, r"몬스터\savage_babe"),    // R_SAVAGE_BABE (variant-monster)
    (1728, r"몬스터\desert_wolf_b"),  // R_DESERT_WOLF_B (variant-monster)
    (1729, r"몬스터\baphomet_"),      // R_BAPHOMET_ (variant-monster)
    (1730, r"몬스터\deviruchi"),      // R_DEVIRUCHI (variant-monster)
    (1732, r"npc\4_treasure_box"),    // G_TREASURE_BOX (family)
    (1785, r"몬스터\garm"),           // ATROCE (id-override)
    (1795, r"몬스터\bloody_knight"),  // BLOODY_KNIGHT_ (variant-monster)
    (1798, r"npc\4_treasure_box"),    // TREASURE_BOX_ (family)
    (1799, r"몬스터\seyren"),         // G_SEYREN_ (variant-monster)
    (1800, r"몬스터\eremes"),         // G_EREMES_ (variant-monster)
    (1801, r"몬스터\harword"),        // G_HARWORD_ (variant-monster)
    (1802, r"몬스터\magaleta"),       // G_MAGALETA_ (variant-monster)
    (1803, r"몬스터\shecil"),         // G_SHECIL_ (variant-monster)
    (1804, r"몬스터\katrinn"),        // G_KATRINN_ (variant-monster)
    (1805, r"몬스터\seyren"),         // B_SEYREN_ (variant-monster)
    (1806, r"몬스터\eremes"),         // B_EREMES_ (variant-monster)
    (1807, r"몬스터\harword"),        // B_HARWORD_ (variant-monster)
    (1808, r"몬스터\magaleta"),       // B_MAGALETA_ (variant-monster)
    (1809, r"몬스터\shecil"),         // B_SHECIL_ (variant-monster)
    (1810, r"몬스터\katrinn"),        // B_KATRINN_ (variant-monster)
    (1811, r"몬스터\smokie"),         // G_SMOKIE_ (variant-monster)
    (1812, r"몬스터\lude"),           // EVENT_LUDE (variant-monster)
    (1813, r"몬스터\hydro"),          // EVENT_HYDRO (variant-monster)
    (1814, r"몬스터\lunatic"),        // EVENT_MOON (family)
    (1817, r"몬스터\detale"),         // EVENT_DETALE (variant-monster)
    (1818, r"몬스터\alarm"),          // EVENT_ALARM (variant-monster)
    (1819, r"몬스터\bathory"),        // EVENT_BATHORY (variant-monster)
    (1820, r"몬스터\bigfoot"),        // EVENT_BIGFOOT (variant-monster)
    (1821, r"몬스터\desert_wolf"),    // EVENT_DESERT_WOLF (variant-monster)
    (1822, r"몬스터\deviruchi"),      // EVENT_DEVIRUCHI (variant-monster)
    (1823, r"몬스터\freezer"),        // EVENT_FREEZER (variant-monster)
    (1824, r"몬스터\garm_baby"),      // EVENT_GARM_BABY (variant-monster)
    (1825, r"몬스터\gobline_xmas"),   // EVENT_GOBLINE_XMAS (variant-monster)
    (1826, r"몬스터\myst"),           // EVENT_MYST (variant-monster)
    (1827, r"몬스터\sasquatch"),      // EVENT_SASQUATCH (variant-monster)
    (1828, r"몬스터\gullinbursti"),   // EVENT_GULLINBURSTI (variant-monster)
    (1829, r"몬스터\beta_guards"),    // SWORD_GUARDIAN (family)
    (1830, r"몬스터\beta_guards"),    // BOW_GUARDIAN (family)
    (1841, r"몬스터\snake"),          // G_SNAKE_ (variant-monster)
    (1842, r"몬스터\anacondaq"),      // G_ANACONDAQ_ (variant-monster)
    (1843, r"몬스터\side_winder"),    // G_SIDE_WINDER_ (variant-monster)
    (1844, r"몬스터\isis"),           // G_ISIS_ (variant-monster)
    (1845, r"npc\4_treasure_box"),    // G_TREASURE_BOX_ (family)
    (1846, r"몬스터\metaling"),       // DREAMMETAL (family)
    (1847, r"몬스터\poring"),         // EVENT_PORING (variant-monster)
    (1848, r"몬스터\baphomet"),       // EVENT_BAPHOMET (variant-monster)
    (1849, r"몬스터\osiris"),         // EVENT_OSIRIS (variant-monster)
    (1850, r"몬스터\ork_hero"),       // EVENT_ORCHERO (fuzzy:0.86)
    (1851, r"몬스터\mobster"),        // EVENT_MOBSTER (variant-monster)
    (1854, r"몬스터\muka"),           // E_MUKA (variant-monster)
    (1855, r"몬스터\poison_spore"),   // E_POISONSPORE (compact-monster)
    (1856, r"몬스터\magnolia"),       // E_MAGNOLIA (variant-monster)
    (1857, r"몬스터\marin"),          // E_MARIN (variant-monster)
    (1858, r"몬스터\plankton"),       // E_PLANKTON (variant-monster)
    (1859, r"몬스터\mandragora"),     // E_MANDRAGORA (variant-monster)
    (1860, r"몬스터\coco"),           // E_COCO (variant-monster)
    (1861, r"몬스터\choco"),          // E_CHOCO (variant-monster)
    (1862, r"몬스터\martin"),         // E_MARTIN (variant-monster)
    (1863, r"몬스터\spring_rabbit"),  // E_SPRING_RABBIT (variant-monster)
    (1872, r"몬스터\beelzebub_"),     // BEELZEBUB_FLY (fuzzy:0.86)
    (1873, r"몬스터\beelzebub_"),     // BEELZEBUB (compact-monster)
    (1876, r"몬스터\lord_of_death"),  // E_LORD_OF_DEATH (variant-monster)
    (1877, r"몬스터\crystal_1"),      // CRYSTAL_5 (id-override)
    (1878, r"몬스터\shining_plant"),  // E_SHINING_PLANT (variant-monster)
    (1887, r"몬스터\freezer"),        // FREEZER_R (variant-monster)
    (1888, r"몬스터\garm_baby"),      // GARM_BABY_R (variant-monster)
    (1889, r"몬스터\garm"),           // GARM_R (variant-monster)
    (1890, r"몬스터\gopinich"),       // GOPINICH_R (variant-monster)
    (1891, r"몬스터\randgris"),       // G_RANDGRIS_ (variant-monster)
    (1895, r"몬스터\seyren"),         // EVENT_SEYREN (variant-monster)
    (1896, r"몬스터\katrinn"),        // EVENT_KATRINN (variant-monster)
    (1897, r"몬스터\baphomet_"),      // EVENT_BAPHOMET_ (variant-monster)
    (1898, r"몬스터\zombie"),         // EVENT_ZOMBIE (variant-monster)
    (1899, r"몬스터\beta_guards"),    // SWORD_GUARDIAN_ (family)
    (1901, r"몬스터\condor"),         // E_CONDOR (variant-monster)
    (1902, r"npc\4_treasure_box"),    // E_TREASURE1 (family)
    (1903, r"npc\4_treasure_box"),    // E_TREASURE2 (family)
    (1906, r"몬스터\barricade"),      // BARRICADE_ (variant-monster)
    (1917, r"몬스터\morocc"),         // MOROCC_ (variant-monster)
    (1926, r"몬스터\jakk"),           // JAKK_H (variant-monster)
    (1927, r"몬스터\whisper"),        // WHISPER_H (variant-monster)
    (1928, r"몬스터\deviruchi"),      // DEVIRUCHI_H (variant-monster)
    (1937, r"몬스터\constant"),       // CONSTANT_ (variant-monster)
    (1938, r"npc\4_treasure_box"),    // TREASURE_BOX41 (family)
    (1939, r"npc\4_treasure_box"),    // TREASURE_BOX42 (family)
    (1940, r"npc\4_treasure_box"),    // TREASURE_BOX43 (family)
    (1941, r"npc\4_treasure_box"),    // TREASURE_BOX44 (family)
    (1942, r"npc\4_treasure_box"),    // TREASURE_BOX45 (family)
    (1943, r"npc\4_treasure_box"),    // TREASURE_BOX46 (family)
    (1944, r"npc\4_treasure_box"),    // TREASURE_BOX47 (family)
    (1945, r"npc\4_treasure_box"),    // TREASURE_BOX48 (family)
    (1946, r"npc\4_treasure_box"),    // TREASURE_BOX49 (family)
    (1947, r"몬스터\piamette"),       // PIAMETTE_ (variant-monster)
    (1949, r"몬스터\beta_guards"),    // B_S_GUARDIAN (family)
    (1950, r"몬스터\beta_guards"),    // B_B_GUARDIAN (family)
    (1951, r"몬스터\crystal_2"),      // CRYSTAL_6 (id-override)
    (1952, r"몬스터\crystal_3"),      // CRYSTAL_7 (id-override)
    (1953, r"몬스터\crystal_4"),      // CRYSTAL_8 (id-override)
    (1954, r"몬스터\crystal_h"),      // CRYSTAL_9 (id-override)
    (1955, r"npc\4_treasure_box"),    // TREASURE_BOX_I (family)
    (1958, r"몬스터\entweihen"),      // G_ENTWEIHEN_R (variant-monster)
    (1959, r"몬스터\entweihen"),      // G_ENTWEIHEN_H (variant-monster)
    (1960, r"몬스터\entweihen"),      // G_ENTWEIHEN_M (family)
    (1961, r"몬스터\entweihen"),      // G_ENTWEIHEN_S (family)
    (1962, r"몬스터\antonio"),        // ANTONIO_ (variant-monster)
    (1963, r"몬스터\chung_e"),        // P_CHUNG_E (variant-monster)
    (1964, r"몬스터\nightmare"),      // NIGHTMARE_T (variant-monster)
    (1965, r"몬스터\wild_rose"),      // M_WILD_ROSE (variant-monster)
    (1966, r"몬스터\doppelganger"),   // M_DOPPELGANGER (variant-monster)
    (1967, r"몬스터\ygnizem"),        // M_YGNIZEM (variant-monster)
    (1968, r"몬스터\strouf"),         // E_STROUF (variant-monster)
    (1969, r"몬스터\marc"),           // E_MARC (variant-monster)
    (1970, r"몬스터\obeaune"),        // E_OBEAUNE (variant-monster)
    (1971, r"몬스터\vadon"),          // E_VADON (variant-monster)
    (1972, r"몬스터\marina"),         // E_MARINA (variant-monster)
    (1973, r"몬스터\poring"),         // E_PORING (variant-monster)
    (1981, r"몬스터\high_orc"),       // I_HIGH_ORC (variant-monster)
    (1982, r"몬스터\orc_archer"),     // I_ORC_ARCHER (variant-monster)
    (1983, r"몬스터\orc_skeleton"),   // I_ORC_SKELETON (variant-monster)
    (1984, r"몬스터\orc_lady"),       // I_ORC_LADY (variant-monster)
    (2008, r"몬스터\drake"),          // WOOMAWANG (family)
    (2009, r"몬스터\drake"),          // WOOMAWANG_ (family)
    (2026, r"몬스터\dandelion"),      // DANDELION_ (variant-monster)
    (2030, r"몬스터\dark_priest"),    // HIDEN_PRIEST (family)
    (2031, r"몬스터\dandelion"),      // DANDELION_H (variant-monster)
    (2042, r"몬스터\bon_gun"),        // SILVERSNIPER (id-override)
    (2043, r"몬스터\mm_magic_seal"),  // MAGICDECOY_FIRE (family)
    (2044, r"몬스터\mm_magic_seal"),  // MAGICDECOY_WATER (family)
    (2045, r"몬스터\mm_magic_seal"),  // MAGICDECOY_EARTH (family)
    (2046, r"몬스터\mm_magic_seal"),  // MAGICDECOY_WIND (family)
    (2047, r"몬스터\naga"),           // W_NAGA (variant-monster)
    (2049, r"몬스터\bradium_golem"),  // W_BRADIUM_GOLEM (variant-monster)
    (2057, r"몬스터\cramp"),          // E_CRAMP (variant-monster)
    (2076, r"몬스터\wind_ghost"),     // S_WIND_GHOST (variant-monster)
    (2077, r"몬스터\skogul"),         // S_SKOGUL (variant-monster)
    (2078, r"몬스터\succubus"),       // S_SUCCUBUS (variant-monster)
    (2081, r"몬스터\hydra"),          // E_HYDRA (variant-monster)
    (2158, r"몬스터\hornet"),         // S_HORNET (variant-monster)
    (2159, r"몬스터\giant_hornet"),   // S_GIANT_HORNET (variant-monster)
    (2160, r"몬스터\luciola_vespa"),  // S_LUCIOLA_VESPA (variant-monster)
    (2308, r"몬스터\md_ninja_ghost"), // KO_ZANZOU (family)
];

/// Generic fallback sprites in preference order. All of them were verified to
/// exist. The first existing one is used for monster ids that have neither an
/// exact path nor an alias entry (e.g. custom mobs from a different data set).
pub const GENERIC_MONSTER_SPRITES: &[&str] = &[
    r"몬스터\poring",
    r"몬스터\fabre",
    r"몬스터\lunatic",
    r"몬스터\skeleton",
    r"몬스터\zombie",
    r"몬스터\pecopeco",
    r"몬스터\chocho",
];

/// Returns the verified fallback sprite path for a monster job id, if one
/// exists.
///
/// The table is sorted by id, so the lookup is a binary search.
pub fn alias(job_id: JobId) -> Option<&'static str> {
    MONSTER_SPRITE_ALIASES
        .binary_search_by_key(&job_id.0, |(id, _)| *id)
        .ok()
        .map(|index| MONSTER_SPRITE_ALIASES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_table_is_sorted_and_unique() {
        assert!(MONSTER_SPRITE_ALIASES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(!MONSTER_SPRITE_ALIASES.is_empty());
    }

    #[test]
    fn alias_paths_are_valid_sprite_paths() {
        for (id, path) in MONSTER_SPRITE_ALIASES {
            assert!(
                path.starts_with("몬스터\\") || path.starts_with("npc\\"),
                "bad folder for {id}: {path}"
            );
            assert!(!path.ends_with(".spr") && !path.ends_with(".act"), "extension in {id}: {path}");
            assert!(!path.to_lowercase().contains("missing"), "missing placeholder in {id}: {path}");
        }
    }

    #[test]
    fn generic_sprites_are_monster_sprites() {
        assert!(!GENERIC_MONSTER_SPRITES.is_empty());
        for path in GENERIC_MONSTER_SPRITES {
            assert!(path.starts_with("몬스터\\"), "bad generic sprite: {path}");
        }
    }

    #[test]
    fn alias_lookup_uses_binary_search() {
        for (id, path) in MONSTER_SPRITE_ALIASES {
            assert_eq!(alias(JobId(*id)), Some(*path));
        }
        assert_eq!(alias(JobId(65535)), None);
    }
}
