mod event;
mod key;
mod mode;

use std::mem::variant_count;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ragnarok_packets::{ClientTick, HotbarSlot};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::KeyCode;

pub use self::event::InputEvent;
pub use self::key::Key;
pub use self::mode::{Grabbed, MouseInputMode, MouseModeExt};
use crate::graphics::{PickerTarget, ScreenPosition, ScreenSize};

const MOUSE_SCOLL_MULTIPLIER: f32 = 30.0;
const KEY_COUNT: usize = variant_count::<KeyCode>();
const DOUBLE_CLICK_TIME_MS: u32 = 250;

#[derive(Debug, Clone, Copy)]
struct PreviousMouseButton {
    button: MouseButton,
    tick: ClientTick,
}

// TODO: Rename
pub struct InputReport {
    pub mouse_click: Option<korangar_interface::layout::MouseButton>,
    pub mouse_position: ScreenPosition,
    pub mouse_delta: ScreenSize,
    pub mouse_button_released: bool,
    /// Whether the right mouse button was released in this frame.
    pub right_mouse_button_released: bool,
    /// Accumulated mouse movement in pixels while the right mouse button was
    /// held down. Used to distinguish a right click (context menu) from a right
    /// drag (camera rotation).
    pub right_mouse_button_drag_distance: f32,
    pub left_mouse_button_down: bool,
    pub scroll: Option<f32>,
    pub drag: Option<ScreenSize>,
    pub characters: Vec<char>,
    pub mouse_target: PickerTarget,
}

pub struct InputSystem {
    previous_mouse_position: ScreenPosition,
    new_mouse_position: ScreenPosition,
    pointer_position_seen: bool,
    mouse_delta: ScreenSize,
    previous_scroll_position: f32,
    new_scroll_position: f32,
    scroll_delta: f32,
    left_mouse_button: Key,
    right_mouse_button: Key,
    keys: [Key; KEY_COUNT],
    input_buffer: Vec<char>,
    picker_value: Arc<AtomicU64>,
    previous_mouse_button: Option<PreviousMouseButton>,
    right_mouse_button_press_position: ScreenPosition,
    right_mouse_button_drag_distance: f32,
}

impl InputSystem {
    pub fn new(picker_value: Arc<AtomicU64>) -> Self {
        let previous_mouse_position = ScreenPosition::default();
        let new_mouse_position = ScreenPosition::default();
        let mouse_delta = ScreenSize::default();

        let previous_scroll_position = 0.0;
        let new_scroll_position = 0.0;
        let scroll_delta = 0.0;

        let left_mouse_button = Key::default();
        let right_mouse_button = Key::default();
        let keys = [Key::default(); KEY_COUNT];

        let input_buffer = Vec::new();
        let previous_mouse_button = None;

        Self {
            previous_mouse_position,
            new_mouse_position,
            pointer_position_seen: false,
            mouse_delta,
            previous_scroll_position,
            new_scroll_position,
            scroll_delta,
            left_mouse_button,
            right_mouse_button,
            keys,
            input_buffer,
            picker_value,
            previous_mouse_button,
            right_mouse_button_press_position: ScreenPosition::default(),
            right_mouse_button_drag_distance: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.left_mouse_button.reset();
        self.right_mouse_button.reset();
        self.keys.iter_mut().for_each(|key| key.reset());
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.pointer_position_seen = true;
        self.new_mouse_position = ScreenPosition {
            left: position.x as f32,
            top: position.y as f32,
        };
    }

    /// Whether at least one pointer position was reported by the windowing
    /// system.
    ///
    /// Clicks that arrive before the first pointer event must not be dispatched
    /// with the default (0, 0) position.
    pub fn has_pointer_position(&self) -> bool {
        self.pointer_position_seen
    }

    /// The window scale factor changed (fractional scaling, moving the window
    /// between outputs).
    ///
    /// The compositor reports cursor positions in the new scale, so the last
    /// known position has to be converted as well. Otherwise a click that
    /// arrives before the next pointer motion event would be hit-tested
    /// against the old coordinate space.
    pub fn rescale_mouse_position(&mut self, factor: f32) {
        self.previous_mouse_position = ScreenPosition {
            left: self.previous_mouse_position.left * factor,
            top: self.previous_mouse_position.top * factor,
        };
        self.new_mouse_position = ScreenPosition {
            left: self.new_mouse_position.left * factor,
            top: self.new_mouse_position.top * factor,
        };
        self.right_mouse_button_press_position = ScreenPosition {
            left: self.right_mouse_button_press_position.left * factor,
            top: self.right_mouse_button_press_position.top * factor,
        };
    }

    /// Drop any held mouse button without emitting a press or release event.
    ///
    /// On Wayland a button release can be lost when the pointer leaves the
    /// surface or the window loses focus while the button is held. A stuck
    /// button would otherwise turn all following clicks into drags and make
    /// the interface unresponsive.
    pub fn clear_mouse_buttons(&mut self) {
        self.left_mouse_button.force_up();
        self.right_mouse_button.force_up();
        self.previous_mouse_button = None;
        self.right_mouse_button_drag_distance = 0.0;
    }

    pub fn update_mouse_buttons(&mut self, button: MouseButton, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);

        match button {
            MouseButton::Left => self.left_mouse_button.set_down(pressed),
            MouseButton::Right => self.right_mouse_button.set_down(pressed),
            _ignored => {}
        }
    }

    pub fn update_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_x, y) => self.new_scroll_position += y * MOUSE_SCOLL_MULTIPLIER,
            MouseScrollDelta::PixelDelta(position) => self.new_scroll_position += position.y as f32,
        }
    }

    pub fn update_keyboard(&mut self, key_code: KeyCode, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);
        self.keys[key_code as usize].set_down(pressed);
    }

    pub fn buffer_character(&mut self, character: char) {
        self.input_buffer.push(character);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update input system"))]
    pub fn update_delta(&mut self, client_tick: ClientTick) -> InputReport {
        self.mouse_delta = self.new_mouse_position - self.previous_mouse_position;
        self.previous_mouse_position = self.new_mouse_position;

        self.scroll_delta = self.new_scroll_position - self.previous_scroll_position;
        self.previous_scroll_position = self.new_scroll_position;

        self.left_mouse_button.update();
        self.right_mouse_button.update();
        self.keys.iter_mut().for_each(|key| key.update());

        let mouse_button_released = self.left_mouse_button.released() || self.right_mouse_button.released();

        // Track whether the right button was pressed and released without moving
        // the mouse, so that a right click can open the character context menu
        // while a right drag keeps rotating the camera.
        let right_mouse_button_released = self.right_mouse_button.released();

        if self.right_mouse_button.pressed() {
            self.right_mouse_button_press_position = self.new_mouse_position;
            self.right_mouse_button_drag_distance = 0.0;
        } else if self.right_mouse_button.down() {
            let offset = self.new_mouse_position - self.right_mouse_button_press_position;
            self.right_mouse_button_drag_distance = self
                .right_mouse_button_drag_distance
                .max(offset.width.abs().max(offset.height.abs()));
        }

        let last_pixel_value = self.picker_value.load(Ordering::Acquire);
        let mouse_target = PickerTarget::from(last_pixel_value);

        let mut mouse_click = None;

        if self.left_mouse_button.pressed() {
            if let Some(previous_mouse_button) = self.previous_mouse_button
                && previous_mouse_button.button == MouseButton::Left
                && client_tick.0.wrapping_sub(previous_mouse_button.tick.0) <= DOUBLE_CLICK_TIME_MS
            {
                self.previous_mouse_button = None;

                mouse_click = Some(korangar_interface::layout::MouseButton::DoubleLeft);
            } else {
                self.previous_mouse_button = Some(PreviousMouseButton {
                    button: MouseButton::Left,
                    tick: client_tick,
                });

                mouse_click = Some(korangar_interface::layout::MouseButton::Left);
            }
        } else if self.right_mouse_button.pressed() {
            if let Some(previous_mouse_button) = self.previous_mouse_button
                && previous_mouse_button.button == MouseButton::Right
                && client_tick.0.wrapping_sub(previous_mouse_button.tick.0) <= DOUBLE_CLICK_TIME_MS
            {
                self.previous_mouse_button = None;

                mouse_click = Some(korangar_interface::layout::MouseButton::DoubleRight);
            } else {
                self.previous_mouse_button = Some(PreviousMouseButton {
                    button: MouseButton::Right,
                    tick: client_tick,
                });

                mouse_click = Some(korangar_interface::layout::MouseButton::Right);
            }
        }

        InputReport {
            mouse_click,
            mouse_position: self.new_mouse_position,
            mouse_delta: self.mouse_delta,
            mouse_button_released,
            right_mouse_button_released,
            right_mouse_button_drag_distance: self.right_mouse_button_drag_distance,
            left_mouse_button_down: self.left_mouse_button.down(),
            scroll: (self.scroll_delta != 0.0).then_some(self.scroll_delta),
            drag: self.left_mouse_button.down().then_some(self.mouse_delta),
            characters: self.input_buffer.drain(..).collect(),
            mouse_target,
        }
    }

    /// Returns whether the given key is currently held down.
    pub fn key_pressed(&self, key_code: KeyCode) -> bool {
        self.get_key(key_code).pressed()
    }

    fn get_key(&self, key_code: KeyCode) -> &Key {
        &self.keys[key_code as usize]
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn handle_keyboard_input(
        &mut self,
        events: &mut Vec<InputEvent>,
        #[cfg(feature = "debug")] process_mouse: bool,
        #[cfg(feature = "debug")] use_debug_camera: bool,
    ) {
        let alt_down = self.get_key(KeyCode::AltLeft).down();
        let control_down = self.get_key(KeyCode::ControlLeft).down();
        let shift_down = self.get_key(KeyCode::ShiftLeft).down();

        if self.get_key(KeyCode::Escape).pressed() {
            events.push(InputEvent::ToggleMenuWindow);
        }

        if alt_down && self.get_key(KeyCode::KeyE).pressed() {
            events.push(InputEvent::ToggleInventoryWindow);
        }

        if alt_down && self.get_key(KeyCode::KeyS).pressed() {
            events.push(InputEvent::ToggleSkillTreeWindow);
        }

        if alt_down && self.get_key(KeyCode::KeyA).pressed() {
            events.push(InputEvent::ToggleStatsWindow);
        }

        if alt_down && self.get_key(KeyCode::KeyZ).pressed() {
            events.push(InputEvent::ToggleFriendListWindow);
        }

        if alt_down && self.get_key(KeyCode::KeyQ).pressed() {
            events.push(InputEvent::ToggleEquipmentWindow);
        }

        if control_down && self.get_key(KeyCode::KeyS).pressed() {
            events.push(InputEvent::ToggleGameSettingsWindow);
        }

        if control_down && self.get_key(KeyCode::KeyI).pressed() {
            events.push(InputEvent::ToggleInterfaceSettingsWindow);
        }

        if control_down && self.get_key(KeyCode::KeyG).pressed() {
            events.push(InputEvent::ToggleGraphicsSettingsWindow);
        }

        if control_down && self.get_key(KeyCode::KeyA).pressed() {
            events.push(InputEvent::ToggleAudioSettingsWindow);
        }

        if control_down && self.get_key(KeyCode::KeyH).pressed() {
            events.push(InputEvent::ToggleShowInterface);
        }

        // Opens/closes the minimap window (Ctrl+M, like the classic client).
        if control_down && self.get_key(KeyCode::KeyM).pressed() {
            events.push(InputEvent::ToggleMinimapWindow);
        }

        // Plain M cycles the map windows: small minimap -> big map -> closed.
        // It is ignored while a text input (chat, login, ...) has focus, those
        // keys belong to the text box. Ctrl+M still toggles the small minimap.
        if !control_down && !alt_down && !shift_down && self.get_key(KeyCode::KeyM).pressed() {
            events.push(InputEvent::CycleMapWindows);
        }

        // Plain I opens/closes the inventory window, like the classic client. It
        // is ignored while a text input (chat, login, ...) has focus, those keys
        // belong to the text box. Alt+E remains as the alternative shortcut.
        if !control_down && !alt_down && !shift_down && self.get_key(KeyCode::KeyI).pressed() {
            events.push(InputEvent::ToggleInventoryWindow);
        }

        if control_down && self.get_key(KeyCode::KeyQ).pressed() {
            events.push(InputEvent::CloseTopWindow);
        }

        // The first nine emotions, matching the Alt + 1-9 binds of the
        // official client.
        const EMOTION_KEYS: [KeyCode; 9] = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ];

        for (emotion, key_code) in EMOTION_KEYS.into_iter().enumerate() {
            if alt_down && self.get_key(key_code).pressed() {
                events.push(InputEvent::SendEmotion { emotion: emotion as u8 });
            }
        }

        // The hotbar slots are mapped to the function keys, like in the
        // original client. Pressing a function key uses the skill in that
        // slot, which either casts it directly or starts a target selection.
        const HOTBAR_KEYS: [KeyCode; 10] = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
            KeyCode::F10,
        ];

        for (slot, key_code) in HOTBAR_KEYS.into_iter().enumerate() {
            if self.get_key(key_code).pressed() {
                events.push(InputEvent::UseSkillInSlot {
                    slot: HotbarSlot(slot as u16),
                });
            }

            if self.get_key(key_code).released() {
                events.push(InputEvent::StopSkill {
                    slot: HotbarSlot(slot as u16),
                });
            }
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyM).pressed() && self.get_key(KeyCode::ShiftLeft).down() {
            events.push(InputEvent::ToggleMapsWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyC).pressed() {
            events.push(InputEvent::ToggleClientStateInspectorWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyR).pressed() {
            events.push(InputEvent::ToggleRenderOptionsWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyP).pressed() {
            events.push(InputEvent::ToggleProfilerWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyO).pressed() {
            events.push(InputEvent::ToggleCommandsWindow);
        }

        #[cfg(feature = "debug")]
        if control_down && self.get_key(KeyCode::KeyN).pressed() {
            events.push(InputEvent::TogglePacketInspectorWindow);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::ShiftLeft).pressed() && use_debug_camera {
            events.push(InputEvent::CameraAccelerate);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::ShiftLeft).released() && use_debug_camera {
            events.push(InputEvent::CameraDecelerate);
        }

        // TODO: This should be moved.
        #[cfg(feature = "debug")]
        if self.right_mouse_button.down() && !self.right_mouse_button.pressed() && process_mouse && use_debug_camera {
            let offset = -cgmath::Vector2::new(self.mouse_delta.width, self.mouse_delta.height);
            events.push(InputEvent::CameraLookAround { offset });
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyW).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveForward);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyS).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveBackward);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyA).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveLeft);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::KeyD).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveRight);
        }

        #[cfg(feature = "debug")]
        if self.get_key(KeyCode::Space).down() && use_debug_camera {
            events.push(InputEvent::CameraMoveUp);
        }

        self.input_buffer.clear();
    }
}
