//! Registry entries for the `TexturedAura` effects (core, clean blocks).

use ragnarok_packets::EffectId;

use super::rag::TexturedAura;
use super::runtime::PortedEffect;

pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::Tarotcard1 => Box::new(TexturedAura::new(&[("effect\\Tarot01.tga", 1)], 300)),
        EffectId::Tarotcard2 => Box::new(TexturedAura::new(&[("effect\\Tarot02.tga", 1)], 300)),
        EffectId::Tarotcard3 => Box::new(TexturedAura::new(&[("effect\\Tarot03.tga", 1)], 300)),
        EffectId::Tarotcard4 => Box::new(TexturedAura::new(&[("effect\\Tarot04.tga", 1)], 300)),
        EffectId::Tarotcard5 => Box::new(TexturedAura::new(&[("effect\\Tarot05.tga", 1)], 300)),
        EffectId::Tarotcard6 => Box::new(TexturedAura::new(&[("effect\\Tarot06.tga", 1)], 300)),
        EffectId::Tarotcard7 => Box::new(TexturedAura::new(&[("effect\\Tarot07.tga", 1)], 300)),
        EffectId::Tarotcard8 => Box::new(TexturedAura::new(&[("effect\\Tarot08.tga", 1)], 300)),
        EffectId::Tarotcard9 => Box::new(TexturedAura::new(&[("effect\\Tarot09.tga", 1)], 300)),
        EffectId::Tarotcard10 => Box::new(TexturedAura::new(&[("effect\\Tarot10.tga", 1)], 300)),
        EffectId::Tarotcard11 => Box::new(TexturedAura::new(&[("effect\\Tarot11.tga", 1)], 300)),
        EffectId::Tarotcard12 => Box::new(TexturedAura::new(&[("effect\\Tarot12.tga", 1)], 300)),
        EffectId::Tarotcard13 => Box::new(TexturedAura::new(&[("effect\\Tarot13.tga", 1)], 300)),
        EffectId::Tarotcard14 => Box::new(TexturedAura::new(&[("effect\\Tarot14.tga", 1)], 300)),
        EffectId::Oratio => Box::new(TexturedAura::new(&[("effect\\마법진1-3.bmp", 19)], 99999999)),
        EffectId::Glow1 => Box::new(TexturedAura::new(&[("effect\\glow01.bmp", 14)], 99999999)),
        EffectId::Glow2 => Box::new(TexturedAura::new(&[("effect\\glow02.bmp", 14)], 99999999)),
        EffectId::Glow11 => Box::new(TexturedAura::new(&[("effect\\glow11.bmp", 15)], 99999999)),
        EffectId::Glow12 => Box::new(TexturedAura::new(&[("effect\\glow12b .bmp", 14)], 99999999)),
        EffectId::Mapsphere => Box::new(TexturedAura::new(
            &[
                ("effect\\구체2.bmp", 12),
                ("effect\\구체3.bmp", 13),
                ("effect\\구체4.bmp", 11),
                ("effect\\구체5.bmp", 10),
            ],
            36000,
        )),
        EffectId::Mapsphere2 => Box::new(TexturedAura::new(
            &[
                ("effect\\구체12.bmp", 12),
                ("effect\\구체3.bmp", 13),
                ("effect\\구체14.bmp", 11),
                ("effect\\구체15.bmp", 10),
            ],
            36000,
        )),
        EffectId::Venomimpress => Box::new(TexturedAura::new(&[("effect\\베놈_상태.bmp", 22)], 99999999)),
        EffectId::Frostmisty => Box::new(TexturedAura::new(&[("effect\\빙결_상태.bmp", 22)], 99999999)),
        EffectId::Coldstatus => Box::new(TexturedAura::new(&[("effect\\얼음결정체1.tga", 22)], 99999999)),
        EffectId::Burning => Box::new(TexturedAura::new(&[("effect\\발화_상태.bmp", 22)], 99999999)),
        EffectId::PngTest => Box::new(TexturedAura::new(&[("pe_fur1_01.bmp", 1)], 1000)),
        _ => return None,
    })
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Tarotcard1,
        EffectId::Tarotcard2,
        EffectId::Tarotcard3,
        EffectId::Tarotcard4,
        EffectId::Tarotcard5,
        EffectId::Tarotcard6,
        EffectId::Tarotcard7,
        EffectId::Tarotcard8,
        EffectId::Tarotcard9,
        EffectId::Tarotcard10,
        EffectId::Tarotcard11,
        EffectId::Tarotcard12,
        EffectId::Tarotcard13,
        EffectId::Tarotcard14,
        EffectId::Oratio,
        EffectId::Glow1,
        EffectId::Glow2,
        EffectId::Glow11,
        EffectId::Glow12,
        EffectId::Mapsphere,
        EffectId::Mapsphere2,
        EffectId::Venomimpress,
        EffectId::Frostmisty,
        EffectId::Coldstatus,
        EffectId::Burning,
        EffectId::PngTest,
    ]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::Tarotcard1, 300),
        (EffectId::Tarotcard2, 300),
        (EffectId::Tarotcard3, 300),
        (EffectId::Tarotcard4, 300),
        (EffectId::Tarotcard5, 300),
        (EffectId::Tarotcard6, 300),
        (EffectId::Tarotcard7, 300),
        (EffectId::Tarotcard8, 300),
        (EffectId::Tarotcard9, 300),
        (EffectId::Tarotcard10, 300),
        (EffectId::Tarotcard11, 300),
        (EffectId::Tarotcard12, 300),
        (EffectId::Tarotcard13, 300),
        (EffectId::Tarotcard14, 300),
        (EffectId::Oratio, 99999999),
        (EffectId::Glow1, 99999999),
        (EffectId::Glow2, 99999999),
        (EffectId::Glow11, 99999999),
        (EffectId::Glow12, 99999999),
        (EffectId::Mapsphere, 36000),
        (EffectId::Mapsphere2, 36000),
        (EffectId::Venomimpress, 99999999),
        (EffectId::Frostmisty, 99999999),
        (EffectId::Coldstatus, 99999999),
        (EffectId::Burning, 99999999),
        (EffectId::PngTest, 1000),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_aura_effect_spawns_with_its_lifetime() {
        for &(effect, lifetime) in lifetimes() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }
}
