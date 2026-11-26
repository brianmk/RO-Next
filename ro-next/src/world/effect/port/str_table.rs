//! Official `.str` resource table and clip reader - the asset half of
//! `CRagEffect::Init`.
//!
//! `RagEffectInitialization.cpp:16-711` switches on the effect id and builds
//! one `.str` file name; `RagEffectInitialization.cpp:699` then loads it with
//! `ResourceMgr::Load(name)`. The `str` extension is registered with the
//! `texture\effect\` prefix (`EZeffectResource.cpp:152`), so every name below
//! is relative to `data\texture\effect\`.
//!
//! `loopLimit` is `0` unless the switch overrides it
//! (`RagEffect.cpp:109`); none of the effects that call `ProcessEZ2STR` here
//! overrides it, so their resource plays once.
//!
//! `CEZeffectRes::Load` (`EZeffectResource.cpp:49-91`) reads exactly the
//! `EffectData` layout the client's `EffectLoader` parses
//! (`loaders/effect/mod.rs:48-61`), so [`load_clips`] reuses that parser and
//! only converts the raw key frames into the port's [`Clip`]; the transform
//! fields are the `EZeffectTransform` members (`EZeffectResource.h:13-43`).

use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::effect::{EffectData, Frame as EffectFrame, LayerData};
use ragnarok_formats::version::GenericFormatMetadata;
use ragnarok_packets::EffectId;
use ro_next_loaders::FileLoader;

use super::str_player::{Clip, KeyFrame, Transform};
use crate::loaders::{GameFileLoader, strip_effect_magic_prefix};

/// `CEZeffectRes`'s registered prefix (`EZeffectResource.cpp:152`), relative to
/// the archive root.
pub const STR_DIRECTORY: &str = "data\\texture\\effect\\";

/// One effect's official `.str` resource.
#[derive(Debug, Clone, Copy)]
pub struct StrResource {
    /// `(effect, name)` rows of `RagEffectInitialization.cpp`'s switch. The
    /// original picks one name; the slice allows a fallback order and keeps a
    /// single-entry list for every effect ported today.
    pub files: &'static [&'static str],
    /// `RagEffectState::loopLimit` (`RagEffect.cpp:109`, default 0 = one pass).
    pub loop_limit: i32,
}

static SIGNUM: StrResource = StrResource {
    files: &["Cross.str"],
    loop_limit: 0,
};

static CART_REVOLUTION: StrResource = StrResource {
    files: &["CartRevolution.str"],
    loop_limit: 0,
};

static STORM_GUST: StrResource = StrResource {
    files: &["StormGust.str"],
    loop_limit: 0,
};

static GLASS_WALL: StrResource = StrResource {
    files: &["SafetyWall.str"],
    loop_limit: 0,
};

static ASPERSIO: StrResource = StrResource {
    files: &["Aspersio.str"],
    loop_limit: 0,
};

static POTION_BERSERK: StrResource = StrResource {
    files: &["버서크.str"],
    loop_limit: 0,
};

/// The official `.str` resource of an effect, if its body drives
/// `ProcessEZ2STR`.
///
/// Rows cite `RagEffectInitialization.cpp`'s switch. `Signum` and
/// `CartRevolution` were the two ids excluded from `registry::ported_ids()`
/// until this table existed; the other four belong to bodies that were already
/// registered (`StormGust`, `GlassWall`, `Aspersio`, `PotionPill`) and whose
/// `.str` layer was never opened before.
pub fn str_resource(effect: EffectId) -> Option<&'static StrResource> {
    Some(match effect {
        // `case EF_SIGNUM: std::sprintf(name, "Cross.str");` - 45-46.
        EffectId::Signum => &SIGNUM,
        // `case EF_CARTREVOLUTION: std::sprintf(name, "CartRevolution.str");`
        // - 247-248.
        EffectId::Cartrevolution => &CART_REVOLUTION,
        // `case EF_STORMGUST: std::sprintf(name, "StormGust.str");` - 96-97.
        EffectId::Stormgust => &STORM_GUST,
        // `case EF_GLASSWALL: case EF_GLASSWALL2:
        //  std::sprintf(name, "SafetyWall.str");` - 234-236.
        EffectId::Glasswall => &GLASS_WALL,
        // `case EF_ASPERSIO: std::sprintf(name, "Aspersio.str");` - 87-88.
        EffectId::Aspersio => &ASPERSIO,
        // `case EF_POTION_BERSERK: case EF_POTION_BERSERK2:
        //  std::sprintf(name, "버서크.str");` - 496-498.
        EffectId::PotionBerserk | EffectId::PotionBerserk2 => &POTION_BERSERK,
        _ => return None,
    })
}

/// Shorthand for [`StrResource::files`].
pub fn str_clips_for(effect: EffectId) -> Option<&'static [&'static str]> {
    str_resource(effect).map(|resource| resource.files)
}

/// `RagEffectState::loopLimit` the player must use for `effect` (`0` when the
/// effect has no `.str` resource).
pub fn str_loop_limit(effect: EffectId) -> i32 {
    str_resource(effect).map_or(0, |resource| resource.loop_limit)
}

/// Loads the first loadable file of `resource` as a set of clips.
///
/// This is the `.str` read of `CEZeffectRes::Load` on top of the same
/// `EffectData` parser the client's `EffectLoader` uses. `None` means no file
/// could be resolved/parsed, in which case the body plays without its `.str`
/// layer instead of aborting the effect.
pub fn load_clips(game_file_loader: &GameFileLoader, resource: &StrResource) -> Option<Vec<Clip>> {
    resource.files.iter().find_map(|file| load_clip_file(game_file_loader, file))
}

/// Reads and parses one `.str` file (relative to [`STR_DIRECTORY`]).
pub fn load_clip_file(game_file_loader: &GameFileLoader, file: &str) -> Option<Vec<Clip>> {
    let path = format!("{STR_DIRECTORY}{file}");
    let mut bytes = game_file_loader.get(&path).ok()?;
    strip_effect_magic_prefix(&mut bytes);

    let mut byte_reader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
    let effect_data = EffectData::from_bytes(&mut byte_reader).ok()?;

    Some(clips_from_layers(&effect_data.layers))
}

/// Converts the parsed layers of a `.str` file into the port's clips.
///
/// One `EZeffectClip` is one `Clip`; `textureCount` and every key frame are
/// carried over verbatim (`EZeffectResource.h:39-57`). The raw key-frame
/// `type` (offset `0x04`) is the absolute/delta selector `ProcessEZ2STR` reads
/// as `key->type` (`RagEffect.cpp:1443`), stored in
/// [`Transform::kind`] by the port.
pub(crate) fn clips_from_layers(layers: &[LayerData]) -> Vec<Clip> {
    layers.iter().map(clip_from_layer).collect()
}

fn clip_from_layer(layer: &LayerData) -> Clip {
    Clip {
        texture_count: layer.texture_count.max(0) as u32,
        frames: layer.frames.iter().map(key_frame).collect(),
    }
}

fn key_frame(frame: &EffectFrame) -> KeyFrame {
    KeyFrame {
        frame: frame.frame_index,
        transform: Transform {
            x: frame.offset.x,
            y: frame.offset.y,
            u: frame.uv[0],
            v: frame.uv[1],
            us: frame.uv[2],
            vs: frame.uv[3],
            u2: frame.uv[4],
            v2: frame.uv[5],
            us2: frame.uv[6],
            vs2: frame.uv[7],
            ax: [frame.xy[0], frame.xy[1], frame.xy[2], frame.xy[3]],
            ay: [frame.xy[4], frame.xy[5], frame.xy[6], frame.xy[7]],
            cr_r: frame.color[0],
            cr_g: frame.color[1],
            cr_b: frame.color[2],
            cr_a: frame.color[3],
            rz: frame.angle,
            kind: frame.frame_type,
            anitype: frame.animation_type,
            anidelta: frame.delay,
            aniframe: frame.texture_index,
        },
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Vector2;

    use super::*;

    #[test]
    fn official_str_files_match_the_initialization_switch() {
        assert_eq!(str_clips_for(EffectId::Signum), Some(&["Cross.str"][..]));
        assert_eq!(str_clips_for(EffectId::Cartrevolution), Some(&["CartRevolution.str"][..]));
        assert_eq!(str_clips_for(EffectId::Stormgust), Some(&["StormGust.str"][..]));
        assert_eq!(str_clips_for(EffectId::Glasswall), Some(&["SafetyWall.str"][..]));
        assert_eq!(str_clips_for(EffectId::Aspersio), Some(&["Aspersio.str"][..]));
        assert_eq!(str_clips_for(EffectId::PotionBerserk), Some(&["버서크.str"][..]));
        assert_eq!(str_clips_for(EffectId::PotionBerserk2), Some(&["버서크.str"][..]));
    }

    #[test]
    fn defaults_match_the_rag_effect_state_constructor() {
        // `CRagEffect::CRagEffect`: loopCount = loopLimit = 0, and none of the
        // rows above overrides loopLimit in `Init`.
        assert_eq!(str_loop_limit(EffectId::Signum), 0);
        assert_eq!(str_loop_limit(EffectId::Cartrevolution), 0);
        assert_eq!(str_loop_limit(EffectId::Bash), 0);
    }

    #[test]
    fn effects_without_a_str_resource_have_no_clips() {
        assert!(str_resource(EffectId::Bash).is_none());
        assert_eq!(str_clips_for(EffectId::Bash), None);
        assert_eq!(str_clips_for(EffectId::Magnumbreak), None);
    }

    #[test]
    fn raw_key_frames_convert_field_for_field() {
        let layers = [LayerData {
            texture_count: 2,
            texture_names: Vec::new(),
            frame_count: 2,
            frames: vec![
                EffectFrame {
                    frame_index: 0,
                    frame_type: 0,
                    offset: Vector2::new(1.0, 2.0),
                    uv: [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
                    xy: [11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
                    texture_index: 1.0,
                    animation_type: 5,
                    delay: 0.5,
                    angle: 90.0,
                    color: [0.1, 0.2, 0.3, 0.4],
                    source_blend_factor: 5,
                    destination_blend_factor: 6,
                    mt_present: 0,
                },
                EffectFrame {
                    frame_index: 12,
                    frame_type: 1,
                    offset: Vector2::new(-1.0, -2.0),
                    uv: [0.0; 8],
                    xy: [0.0; 8],
                    texture_index: 0.0,
                    animation_type: 0,
                    delay: 0.0,
                    angle: 0.0,
                    color: [0.0; 4],
                    source_blend_factor: 0,
                    destination_blend_factor: 0,
                    mt_present: 0,
                },
            ],
        }];

        let clips = clips_from_layers(&layers);

        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].texture_count, 2);
        assert_eq!(clips[0].frames.len(), 2);
        assert_eq!(clips[0].frames[0].frame, 0);
        assert_eq!(clips[0].frames[0].transform, Transform {
            x: 1.0,
            y: 2.0,
            u: 3.0,
            v: 4.0,
            us: 5.0,
            vs: 6.0,
            u2: 7.0,
            v2: 8.0,
            us2: 9.0,
            vs2: 10.0,
            ax: [11.0, 12.0, 13.0, 14.0],
            ay: [15.0, 16.0, 17.0, 18.0],
            cr_r: 0.1,
            cr_g: 0.2,
            cr_b: 0.3,
            cr_a: 0.4,
            rz: 90.0,
            kind: 0,
            anitype: 5,
            anidelta: 0.5,
            aniframe: 1.0,
        });
        assert_eq!(clips[0].frames[1].frame, 12);
        assert_eq!(clips[0].frames[1].transform.kind, 1, "delta key type");
        assert_eq!(clips[0].frames[1].transform.x, -1.0);
    }
}
