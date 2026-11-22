use std::cell::{Ref, RefCell};
use std::sync::Arc;

use cgmath::{EuclideanSpace, Vector2};
#[cfg(feature = "debug")]
use korangar_interface::application::Clip;
use korangar_interface::application::{RenderLayer, ShadowPadding as _};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{ClipId, Icon, WindowLayout};
use korangar_interface::skin::Slice;

use crate::graphics::{
    Color, CornerDiameter, InterfaceRectangleInstruction, ScreenClip, ScreenPosition, ScreenSize, ShadowPadding, Texture,
};
use crate::loaders::{FontLoader, FontSize, GLYPH_PADDING, GlyphInstruction, ImageType, OverflowBehavior, Sprite, TextureLoader};
use crate::renderer::SpriteRenderer;
use crate::renderer::skin::{SkinImage, SkinTextures};
use crate::state::ClientState;
use crate::world::{Actions, AnimationData, SpriteAnimationState};

/// Renders the interface provided by [`korangar_interface`].
pub struct InterfaceRenderer {
    instructions: RefCell<Vec<InterfaceRectangleInstruction>>,
    glyphs: RefCell<Vec<GlyphInstruction>>,
    font_loader: Arc<FontLoader>,
    filled_box_texture: Arc<Texture>,
    unfilled_box_texture: Arc<Texture>,
    arrow_left_texture: Arc<Texture>,
    arrow_right_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    eye_open_texture: Arc<Texture>,
    eye_closed_texture: Arc<Texture>,
    trash_can_texture: Arc<Texture>,
    skin_textures: SkinTextures,
    window_size: ScreenSize,
    interface_size: ScreenSize,
    high_quality_interface: bool,
    #[cfg(feature = "debug")]
    show_rectangle_instructions: bool,
    #[cfg(feature = "debug")]
    show_glyph_instructions: bool,
    #[cfg(feature = "debug")]
    show_sprite_instructions: bool,
    #[cfg(feature = "debug")]
    show_sdf_instructions: bool,
}

impl InterfaceRenderer {
    /// Create a new interface renderer.
    ///
    /// This include loading the textures icons for rendering components.
    pub fn new(
        window_size: ScreenSize,
        font_loader: Arc<FontLoader>,
        texture_loader: Arc<TextureLoader>,
        high_quality_interface: bool,
    ) -> Self {
        let instructions = RefCell::new(Vec::default());
        let glyphs = RefCell::new(Vec::default());

        let filled_box_texture = texture_loader.get_or_load("filled_box.png", ImageType::Sdf).unwrap();
        let unfilled_box_texture = texture_loader.get_or_load("unfilled_box.png", ImageType::Sdf).unwrap();
        let arrow_left_texture = texture_loader.get_or_load("arrow_left.png", ImageType::Sdf).unwrap();
        let arrow_right_texture = texture_loader.get_or_load("arrow_right.png", ImageType::Sdf).unwrap();
        let expanded_arrow_texture = texture_loader.get_or_load("expanded_arrow.png", ImageType::Sdf).unwrap();
        let collapsed_arrow_texture = texture_loader.get_or_load("collapsed_arrow.png", ImageType::Sdf).unwrap();
        let eye_open_texture = texture_loader.get_or_load("eye_open.png", ImageType::Sdf).unwrap();
        let eye_closed_texture = texture_loader.get_or_load("eye_closed.png", ImageType::Sdf).unwrap();
        let trash_can_texture = texture_loader.get_or_load("trash_can.png", ImageType::Sdf).unwrap();
        let skin_textures = SkinTextures::new(texture_loader.clone());

        let interface_size = if high_quality_interface { window_size * 2.0 } else { window_size };

        #[cfg(feature = "debug")]
        let show_rectangle_instructions = false;
        #[cfg(feature = "debug")]
        let show_glyph_instructions = false;
        #[cfg(feature = "debug")]
        let show_sprite_instructions = false;
        #[cfg(feature = "debug")]
        let show_sdf_instructions = false;

        Self {
            instructions,
            glyphs,
            font_loader,
            filled_box_texture,
            unfilled_box_texture,
            arrow_left_texture,
            arrow_right_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            eye_open_texture,
            eye_closed_texture,
            trash_can_texture,
            skin_textures,
            window_size,
            interface_size,
            high_quality_interface,
            #[cfg(feature = "debug")]
            show_rectangle_instructions,
            #[cfg(feature = "debug")]
            show_glyph_instructions,
            #[cfg(feature = "debug")]
            show_sprite_instructions,
            #[cfg(feature = "debug")]
            show_sdf_instructions,
        }
    }

    #[cfg(feature = "debug")]
    pub fn update_render_options(&mut self, render_options: &crate::RenderOptions) {
        self.show_rectangle_instructions = render_options.show_rectangle_instructions;
        self.show_glyph_instructions = render_options.show_glyph_instructions;
        self.show_sprite_instructions = render_options.show_sprite_instructions;
        self.show_sdf_instructions = render_options.show_sdf_instructions;
    }

    /// Don't show rectangle instructions for the duration of the closure. This
    /// is only used to render other debug areas.
    #[cfg(feature = "debug")]
    pub fn with_rectangles_direct(&mut self, f: impl Fn(&Self)) {
        let previous_state = self.show_rectangle_instructions;
        self.show_rectangle_instructions = false;

        f(self);

        self.show_rectangle_instructions = previous_state;
    }

    /// Clear render instructions.
    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    /// Get render instructions.
    pub fn get_instructions(&self) -> Ref<'_, Vec<InterfaceRectangleInstruction>> {
        self.instructions.borrow()
    }

    /// Inform the renderer of a change in the high quality interface setting.
    pub fn update_high_quality_interface(&mut self, high_quality_interface: bool) {
        self.high_quality_interface = high_quality_interface;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Inform the renderer of new window size.
    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Get the bounds of a given text, respecting the loaded font.
    pub fn get_text_dimensions(
        &self,
        text: &str,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
        mut available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        if self.high_quality_interface {
            // We need to adjust the font size, or else we would create glyphs for a font
            // size, that we don't use.
            font_size = font_size * 2.0;
            available_width *= 2.0;
        }

        let (mut size, mut font_size) =
            self.font_loader
                .get_text_dimensions(text, color, highlight_color, font_size, 1.0, available_width, overflow_behavior);

        if self.high_quality_interface {
            size = size / 2.0;
            font_size = FontSize(font_size.0 / 2.0);
        }

        (size, font_size)
    }

    /// Add instruction for rendering a rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        mut corner_diameter: CornerDiameter,
        color: Color,
        shadow_color: Color,
        mut shadow_padding: ShadowPadding,
    ) {
        // If the rectangle is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_rectangle_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(255, 150, 0, 150),
                    shadow_padding: ShadowPadding::uniform(5.0),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
            corner_diameter = corner_diameter * 2.0;
            shadow_padding = shadow_padding.scaled(2.0);
        }

        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_rectangle_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip,
                color: Color::rgba_u8(170, 0, 170, 90),
                corner_diameter: CornerDiameter::default(),
                shadow_color: Color::rgba_u8(0, 255, 255, 150),
                shadow_padding: ShadowPadding::uniform(5.0),
            });

            return;
        }

        let corner_diameter = corner_diameter * 0.5;

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            shadow_color,
            shadow_padding,
        });
    }

    /// Add instructions for rendering glyphs.
    #[allow(clippy::too_many_arguments)]
    pub fn render_text(
        &self,
        text: &str,
        mut text_position: ScreenPosition,
        mut available_width: f32,
        mut screen_clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
    ) -> f32 {
        // TODO: Can't we scale after laying out the text? Would cut down on
        // multiplications.
        if self.high_quality_interface {
            text_position = text_position * 2.0;
            available_width *= 2.0;
            screen_clip = screen_clip * 2.0;
            font_size = font_size * 2.0;
        }

        let mut glyphs = self.glyphs.borrow_mut();

        let mut size = self.font_loader.layout_text(
            text,
            color,
            highlight_color,
            font_size,
            1.0,
            Some(available_width),
            Some(&mut glyphs),
        );

        let mut instructions = self.instructions.borrow_mut();

        glyphs.drain(..).for_each(|glyph| {
            let position = glyph.position;

            // If the character is not even within the bounds of the clip, discard it early
            // saving GPU resources.
            if text_position.left + position.min.x > screen_clip.right
                || text_position.top + position.min.y > screen_clip.bottom
                || text_position.left + position.max.x < screen_clip.left
                || text_position.top + position.max.y < screen_clip.top
            {
                #[cfg(feature = "debug")]
                if self.show_glyph_instructions {
                    let screen_position = ScreenPosition {
                        left: text_position.left + position.min.x,
                        top: text_position.top + position.min.y,
                    } / self.interface_size;

                    let screen_size = ScreenSize {
                        width: position.width(),
                        height: position.height(),
                    } / self.interface_size;

                    instructions.push(InterfaceRectangleInstruction::Solid {
                        screen_position,
                        screen_size,
                        screen_clip: ScreenClip::unbound(),
                        color: Color::rgba_u8(255, 0, 0, 150),
                        corner_diameter: CornerDiameter::default(),
                        shadow_color: Color::rgba_u8(255, 150, 0, 150),
                        shadow_padding: ShadowPadding::uniform(0.0),
                    });
                }

                return;
            }

            #[cfg(feature = "debug")]
            if self.show_glyph_instructions {
                let screen_position = ScreenPosition {
                    left: text_position.left + position.min.x,
                    top: text_position.top + position.min.y,
                } / self.interface_size;

                let screen_size = ScreenSize {
                    width: position.width(),
                    height: position.height(),
                } / self.interface_size;

                instructions.push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(180, 255, 0, 150),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(0, 255, 0, 150),
                    shadow_padding: ShadowPadding::uniform(0.0),
                });

                return;
            }

            let GlyphInstruction {
                position,
                em_coordinate,
                glyph_index,
                color,
            } = glyph.dilated(GLYPH_PADDING, font_size);

            let screen_position = ScreenPosition {
                left: text_position.left + position.min.x,
                top: text_position.top + position.min.y,
            } / self.interface_size;

            let screen_size = ScreenSize {
                width: position.width(),
                height: position.height(),
            } / self.interface_size;

            instructions.push(InterfaceRectangleInstruction::Text {
                screen_position,
                screen_size,
                screen_clip,
                color,
                em_position: em_coordinate.min.to_vec(),
                em_size: em_coordinate.max - em_coordinate.min,
                glyph_index,
            });
        });

        if self.high_quality_interface {
            size.y /= 2.0;
        }

        size.y
    }

    /// Render a checkbox icon using an SDF.
    pub fn render_checkbox(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, checked: bool) {
        let texture = match checked {
            true => self.filled_box_texture.clone(),
            false => self.unfilled_box_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render a left arrow icon using an SDF.
    pub fn render_arrow_left(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.arrow_left_texture.clone(), position, size, clip, color);
    }

    /// Render a right arrow icon using an SDF.
    pub fn render_arrow_right(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.arrow_right_texture.clone(), position, size, clip, color);
    }

    /// Render an expand arrow icon using an SDF.
    pub fn render_expand_arrow(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, expanded: bool) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render an eye icon using an SDF.
    pub fn render_eye(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, open: bool) {
        let texture = match open {
            true => self.eye_open_texture.clone(),
            false => self.eye_closed_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render a trash can icon using an SDF.
    pub fn render_trash_can(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.trash_can_texture.clone(), position, size, clip, color);
    }

    /// Render a section of a texture.
    ///
    /// This is the same as rendering a sprite, but only the given normalized
    /// section of the texture is sampled. It is used to draw the individual
    /// pieces of a nine slice skin image.
    #[allow(clippy::too_many_arguments)]
    pub fn render_texture_section(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
    ) {
        // If the sprite is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sprite {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            texture,
            smooth,
            texture_position,
            texture_size,
        });
    }

    /// Render an image of the classic interface skin.
    ///
    /// If the image has slice insets it is drawn as a nine slice, otherwise the
    /// whole texture is stretched over the area. A missing texture falls back
    /// to a solid rectangle so that an element never disappears completely.
    #[allow(clippy::too_many_arguments)]
    pub fn render_skin_image(
        &self,
        image: SkinImage,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        slices: Slice,
        smooth: bool,
    ) {
        let Some(texture) = self.skin_textures.get(image) else {
            self.render_rectangle(
                position,
                size,
                screen_clip,
                CornerDiameter::default(),
                color,
                Color::TRANSPARENT,
                ShadowPadding::none(),
            );
            return;
        };

        let texture_extent = texture.get_size();
        let texture_width = texture_extent.width as f32;
        let texture_height = texture_extent.height as f32;

        if slices.is_none() || texture_width <= 0.0 || texture_height <= 0.0 || size.width <= 0.0 || size.height <= 0.0 {
            self.render_texture_section(
                texture,
                position,
                size,
                screen_clip,
                color,
                smooth,
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 1.0),
            );
            return;
        }

        // Clamp the insets so that opposite sides can never overlap.
        let area_size = (size.width, size.height);
        let texture_size = (texture_width, texture_height);

        for piece in crate::renderer::skin::nine_slice_pieces(area_size, slices, texture_size) {
            let piece_position = ScreenPosition {
                left: position.left + piece.position.0,
                top: position.top + piece.position.1,
            };
            let piece_size = ScreenSize {
                width: piece.size.0,
                height: piece.size.1,
            };
            let piece_texture_position = Vector2::new(piece.texture_position.0, piece.texture_position.1);
            let piece_texture_size = Vector2::new(piece.texture_size.0, piece.texture_size.1);

            self.render_texture_section(
                texture.clone(),
                piece_position,
                piece_size,
                screen_clip,
                color,
                smooth,
                piece_texture_position,
                piece_texture_size,
            );
        }
    }
}

impl SpriteRenderer for InterfaceRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) {
        self.render_texture_section(
            texture,
            position,
            size,
            screen_clip,
            color,
            smooth,
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
        );
    }

    fn render_sdf(&self, texture: Arc<Texture>, position: ScreenPosition, size: ScreenSize, mut screen_clip: ScreenClip, color: Color) {
        // If the SDF is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_sdf_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                    shadow_color: Color::rgba_u8(255, 150, 0, 150),
                    shadow_padding: ShadowPadding::uniform(5.0),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_sdf_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip: ScreenClip::unbound(),
                color: Color::rgba_u8(150, 255, 0, 90),
                corner_diameter: CornerDiameter::default(),
                shadow_color: Color::rgba_u8(255, 255, 0, 150),
                shadow_padding: ShadowPadding::uniform(5.0),
            });

            return;
        }

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sdf {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            texture,
        });
    }
}

/// An instruction to render a texture.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct TextureInstruction {
    texture: Arc<Texture>,
    clip_id: ClipId,
    area: Area,
    color: Color,
    smooth: bool,
}

/// An instruction to render a sprite.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct SpriteInstruction<'a> {
    actions: &'a Actions,
    sprite: &'a Sprite,
    animation_state: &'a SpriteAnimationState,
    clip_id: ClipId,
    area: Area,
    color: Color,
    scaling: f32,
}

/// An instruction to render a composited character (all sprite parts of an
/// animation data set, idle pose, front facing) into an interface area. This is
/// used by the equipment paper doll.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
/// Vertical anchor of the character feet inside the paper doll area (fraction
/// of the area height).
const CHARACTER_FEET_ANCHOR: f32 = 0.80;

struct CharacterInstruction<'a> {
    animation_data: &'a AnimationData,
    clip_id: ClipId,
    area: Area,
    color: Color,
    scaling: f32,
}

/// A custom layout instruction.
///
/// Only pub to make the compiler happy, its not used outside of this module.
#[allow(private_interfaces)]
pub enum CustomInstruction<'a> {
    /// An instruction to render a texture.
    Texture(TextureInstruction),
    /// An instruction to render a sprite.
    Sprite(SpriteInstruction<'a>),
    /// An instruction to render a composited character.
    Character(CharacterInstruction<'a>),
}

impl RenderLayer<ClientState> for InterfaceRenderer {
    type CustomIcon = ();
    type CustomInstruction<'a> = CustomInstruction<'a>;

    fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        clip: ScreenClip,
        corner_diameter: CornerDiameter,
        color: Color,
        shadow_color: Color,
        shadow_padding: ShadowPadding,
    ) {
        self.render_rectangle(position, size, clip, corner_diameter, color, shadow_color, shadow_padding);
    }

    fn render_text(
        &self,
        text: &str,
        position: ScreenPosition,
        available_width: f32,
        clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        font_size: FontSize,
    ) {
        self.render_text(text, position, available_width, clip, color, highlight_color, font_size);
    }

    fn render_image(
        &self,
        image: SkinImage,
        position: ScreenPosition,
        size: ScreenSize,
        clip: ScreenClip,
        color: Color,
        slices: Slice,
        smooth: bool,
    ) {
        self.render_skin_image(image, position, size, clip, color, slices, smooth);
    }

    fn render_icon(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, icon: Icon<ClientState>, color: Color) {
        match icon {
            Icon::ArrowLeft => self.render_arrow_left(position, size, clip, color),
            Icon::ArrowRight => self.render_arrow_right(position, size, clip, color),
            Icon::ExpandArrow { expanded } => self.render_expand_arrow(position, size, clip, color, expanded),
            Icon::Checkbox { checked } => self.render_checkbox(position, size, clip, color, checked),
            Icon::Eye { open } => self.render_eye(position, size, clip, color, open),
            Icon::TrashCan => self.render_trash_can(position, size, clip, color),
            Icon::Custom { .. } => {}
        }
    }

    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clips: &[ScreenClip]) {
        match instruction {
            CustomInstruction::Sprite(SpriteInstruction {
                actions,
                sprite,
                animation_state,
                clip_id,
                area,
                color,
                scaling,
            }) => {
                let position = ScreenPosition {
                    left: area.left + area.width / 2.0,
                    top: area.top + area.height / 2.0,
                };
                let screen_clip = clips[clip_id.as_index()];

                actions.render_sprite(self, sprite, animation_state, position, 0, screen_clip, color, scaling);
            }
            CustomInstruction::Character(CharacterInstruction {
                animation_data,
                clip_id,
                area,
                color,
                scaling,
            }) => {
                // The sprite origin is at the feet of the character, so the character is
                // anchored slightly below the vertical center of the area and grows
                // upwards. This matches the original client's paper doll, where the
                // character stands at the bottom of the panel.
                let position = ScreenPosition {
                    left: area.left + area.width / 2.0,
                    top: area.top + area.height * CHARACTER_FEET_ANCHOR,
                };
                let screen_clip = clips[clip_id.as_index()];

                // Every sprite part of the character (body, hair, headgear layers) is an
                // animation pair. The idle action facing south is action index 0 with
                // frame 0; all parts are drawn at the same position, their action clips
                // contain the offsets relative to the character origin, exactly like in
                // the original 2D client.
                for animation_pair in &animation_data.animation_pair {
                    animation_pair
                        .actions
                        .render_sprite_frame(self, &animation_pair.sprites, 0, 0, position, screen_clip, color, scaling);
                }
            }
            CustomInstruction::Texture(TextureInstruction {
                texture,
                clip_id,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition {
                    left: area.left,
                    top: area.top,
                };
                let size = ScreenSize {
                    width: area.width,
                    height: area.height,
                };
                let screen_clip = clips[clip_id.as_index()];

                self.render_sprite(texture, position, size, screen_clip, color, smooth);
            }
        }
    }
}

/// Extension trait to make adding custom instructions to the [`Layout`]
/// seamless by mirroring its API.
pub trait LayoutExt<'a> {
    /// Add an instruction to render a texture.
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool);

    /// Add an instruction to render a sprite.
    fn add_sprite(
        &mut self,
        area: Area,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        color: Color,
        scale: f32,
    );

    /// Add an instruction to render a composited character in its idle pose
    /// (front facing), e.g. for the equipment paper doll.
    fn add_character(&mut self, area: Area, animation_data: &'a AnimationData, color: Color, scale: f32);
}

impl<'a> LayoutExt<'a> for WindowLayout<'a, ClientState> {
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);

        self.add_custom_instruction(CustomInstruction::Texture(TextureInstruction {
            texture,
            clip_id,
            area,
            color,
            smooth,
        }));
    }

    fn add_sprite(
        &mut self,
        area: Area,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        color: Color,
        scale: f32,
    ) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let scaling = scale * self.get_interface_scaling();

        self.add_custom_instruction(CustomInstruction::Sprite(SpriteInstruction {
            actions,
            sprite,
            animation_state,
            clip_id,
            area,
            color,
            scaling,
        }));
    }

    fn add_character(&mut self, area: Area, animation_data: &'a AnimationData, color: Color, scale: f32) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let scaling = scale * self.get_interface_scaling();

        self.add_custom_instruction(CustomInstruction::Character(CharacterInstruction {
            animation_data,
            clip_id,
            area,
            color,
            scaling,
        }));
    }
}
