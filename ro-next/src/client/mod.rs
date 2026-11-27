//! The `Client` behaviour that is not part of its lifecycle.
//!
//! `lib.rs` keeps the `Client` struct and `init`/`run`; this module holds the
//! event dispatchers (`network/`, `user_events.rs` + `input_*.rs`) and the
//! self-contained helpers (`effects`, `render`, `frame`, `items`, `hud`,
//! `navigation`, `chat`), each as its own `impl Client` file.

pub(crate) mod chat;
pub(crate) mod effects;
pub(crate) mod frame;
pub(crate) mod hud;
pub(crate) mod items;
pub(crate) mod loaders;
pub(crate) use self::items::*;
pub(crate) mod navigation;
pub(crate) mod network;
pub(crate) mod render;
pub(crate) use self::render::MapRenderContext;
pub(crate) mod user_events;

pub(crate) mod gameplay;
pub(crate) mod input_character;
pub(crate) mod input_chat;
pub(crate) mod input_friends;
pub(crate) mod input_guild;
pub(crate) mod input_inventory;
pub(crate) mod input_mail;
pub(crate) mod input_map;
pub(crate) mod input_party;
pub(crate) mod input_session;
pub(crate) mod input_skills;
pub(crate) mod input_trade;
pub(crate) mod input_ui;
pub(crate) mod input_windows;
pub(crate) mod lifecycle;
pub(crate) mod windows;

use crate::*;

pub(crate) static NEXT_POSITION_EFFECT_LIGHT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x5000_0000);

/// Packet callback used in release builds.
///
/// Unknown/unparseable 2019 server packets are logged at debug verbosity
/// (`-vv` / `RO_NEXT_LOG_LEVEL=2`) so that missing handlers can be spotted
/// without the debug packet inspector.
#[cfg(not(feature = "debug"))]
#[derive(Clone, Copy, Default)]
pub(crate) struct ClientPacketCallback;

#[cfg(not(feature = "debug"))]
impl ragnarok_packets::handler::PacketCallback for ClientPacketCallback {
    fn incoming_packet<Packet>(&self, _packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
    }

    fn outgoing_packet<Packet>(&self, _packet: &Packet)
    where
        Packet: ragnarok_packets::Packet,
    {
    }

    fn unknown_packet(&self, bytes: Vec<u8>) {
        let header = bytes
            .get(..2)
            .map(|header| u16::from_le_bytes([header[0], header[1]]))
            .map(|header| format!("0x{header:04X}"))
            .unwrap_or_else(|| "unknown".to_owned());

        crate::logging::log_debug!(
            "packets: unhandled server packet header {} ({} bytes remaining)",
            header,
            bytes.len()
        );
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ragnarok_bytes::ConversionError>) {
        crate::logging::log_warn!("packets: failed to parse server packet ({} bytes): {:?}", bytes.len(), error);
    }
}

pub(crate) const CLIENT_NAME: &str = "RO-Next";
/// Minimum duration of the skill-cast animation for casts without a usable cast
/// time, so that they are still visible.
/// Duration of the one-shot attack/pickup animation for instant skills, where
/// the server sends no attack motion (the official client uses a 400 ms hold).
pub(crate) const INSTANT_SKILL_ANIMATION_MS: u32 = 400;
/// Maximum number of teleport-vanish effects spawned within
/// `TELEPORT_VANISH_WINDOW`. Population-engine shells can vanish in bursts;
/// this keeps the effect renderer cheap.
pub(crate) const TELEPORT_VANISH_BURST_LIMIT: usize = 12;

/// Sliding window used for [`TELEPORT_VANISH_BURST_LIMIT`].
pub(crate) const TELEPORT_VANISH_WINDOW: Duration = Duration::from_millis(700);
pub(crate) const ITEM_PICKUP_RANGE: AttackRange = AttackRange(1);
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
pub(crate) const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;

pub(crate) const INITIAL_SCREEN_SIZE: ScreenSize = ScreenSize {
    width: 1280.0,
    height: 720.0,
};

/// Classic RO name label colours per entity type.
///
/// Players are white, NPCs are blue. Monsters only get a label while hovered;
/// the 2019 spawn packets don't expose the "aggressive" flag, so the classic
/// aggressive red is used for them.
pub(crate) fn entity_name_color(entity_type: EntityType) -> Color {
    match entity_type {
        EntityType::Player => Color::WHITE,
        EntityType::Npc => Color::rgb_u8(0x5B, 0xA8, 0xFF),
        EntityType::Monster => Color::rgb_u8(0xFF, 0x7A, 0x7A),
        // Companions use the classic light green/cyan-ish pet names.
        EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary => Color::rgb_u8(0x9A, 0xE8, 0x9A),
        EntityType::Warp | EntityType::Hidden => Color::WHITE,
    }
}

/// Base font size of the character name label, in interface pixels.
pub(crate) const NAME_FONT_SIZE: f32 = 16.0;
/// Base font size of the guild name line below the character name.
pub(crate) const GUILD_NAME_FONT_SIZE: f32 = 12.0;
/// Vertical gap between the character name and the guild name line, in
/// interface pixels (scaled with the interface scaling factor when rendering).
pub(crate) const GUILD_NAME_GAP: f32 = 2.0;
/// Dimmed blue-gray color of the guild name line, so that it is readable but
/// does not compete with the character name itself.
pub(crate) const GUILD_NAME_COLOR: Color = Color::rgb_u8(0x9D, 0xB4, 0xC8);
/// Size of the guild emblem icon next to the guild name, in UI pixels.
pub(crate) const GUILD_EMBLEM_ICON_SIZE: f32 = 14.0;
/// Neutral placeholder color for guild emblems that are not loaded yet.
pub(crate) const GUILD_EMBLEM_PLACEHOLDER_COLOR: Color = Color::rgba_u8(0x78, 0x78, 0x78, 0xB4);

pub(crate) const INITIAL_SCALING_FACTOR: Scaling = Scaling::new(1.0);
pub(crate) const FALLBACK_PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

pub(crate) static ICON_DATA: &[u8] = include_bytes!("../../archive/data/icon.png");

/// Which window backend the client should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BackendPreference {
    /// Prefer native Wayland when the session provides it, otherwise use X11.
    Auto,
    /// Force the native Wayland backend, fail if it is not usable.
    Wayland,
    /// Force the X11 backend (native X11 or XWayland), fail if it is not
    /// usable.
    X11,
}

impl BackendPreference {
    fn from_environment() -> Self {
        let enabled = |name: &str| {
            std::env::var(name).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        };

        match (enabled("RO_NEXT_X11"), enabled("RO_NEXT_WAYLAND")) {
            (true, true) => {
                crate::logging::log_warn!("[display] both RO_NEXT_X11 and RO_NEXT_WAYLAND are set, using X11");
                Self::X11
            }
            (true, false) => Self::X11,
            (false, true) => Self::Wayland,
            (false, false) => Self::Auto,
        }
    }
}

/// Returns whether a Wayland compositor is reachable through the environment.
///
/// `WAYLAND_DISPLAY` is either an absolute path to the socket or a socket name
/// inside `XDG_RUNTIME_DIR`; `WAYLAND_SOCKET` (a file descriptor passed by the
/// parent process) also counts. A stale `WAYLAND_DISPLAY` without a socket file
/// is treated as unavailable, so the client can fall back to X11 instead of
/// failing to connect.
pub(crate) fn wayland_session_available() -> bool {
    if let Some(wayland_display) = std::env::var_os("WAYLAND_DISPLAY").filter(|value| !value.is_empty()) {
        let wayland_display = Path::new(&wayland_display);

        if wayland_display.is_absolute() {
            return wayland_display.exists();
        }

        return std::env::var_os("XDG_RUNTIME_DIR")
            .filter(|runtime_dir| !runtime_dir.is_empty())
            .map(|runtime_dir| Path::new(&runtime_dir).join(wayland_display).exists())
            .unwrap_or(false);
    }

    std::env::var_os("WAYLAND_SOCKET").is_some_and(|value| !value.is_empty())
}

pub(crate) fn x11_session_available() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty())
}

#[cfg(target_os = "linux")]
pub(crate) fn create_wayland_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    let mut builder = EventLoop::builder();
    builder.with_wayland();
    builder.build()
}

#[cfg(target_os = "linux")]
pub(crate) fn create_x11_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    use winit::platform::x11::EventLoopBuilderExtX11;

    let mut builder = EventLoop::builder();
    builder.with_x11();
    builder.build()
}

/// Creates the event loop and selects the window backend.
///
/// Native Wayland is preferred when the session provides a usable compositor.
/// If connecting to it fails (compositor gone, invalid `WAYLAND_DISPLAY`,
/// stale socket, ...) the client automatically retries with X11 when `DISPLAY`
/// is set, instead of panicking like the previous unconditional
/// `EventLoop::new().unwrap()`. Force a backend with `RO_NEXT_X11=1` or
/// `RO_NEXT_WAYLAND=1`.
#[cfg(target_os = "linux")]
pub(crate) fn create_event_loop() -> Result<EventLoop<()>, String> {
    let preference = BackendPreference::from_environment();

    match preference {
        BackendPreference::X11 => {
            let event_loop = create_x11_event_loop().map_err(|error| format!("failed to create the X11 event loop: {error}"))?;
            eprintln!("[display] using X11 backend (forced by RO_NEXT_X11)");
            Ok(event_loop)
        }
        BackendPreference::Wayland => {
            let event_loop = create_wayland_event_loop().map_err(|error| format!("failed to create the Wayland event loop: {error}"))?;
            eprintln!("[display] using native Wayland backend (forced by RO_NEXT_WAYLAND)");
            Ok(event_loop)
        }
        BackendPreference::Auto => {
            if wayland_session_available() {
                match create_wayland_event_loop() {
                    Ok(event_loop) => {
                        eprintln!("[display] using native Wayland backend");
                        return Ok(event_loop);
                    }
                    Err(error) => {
                        crate::logging::log_warn!("[display] could not connect to the Wayland compositor: {error}");
                        if x11_session_available() {
                            crate::logging::log_warn!("[display] falling back to the X11 backend");
                        }
                    }
                }
            }

            if x11_session_available() {
                match create_x11_event_loop() {
                    Ok(event_loop) => {
                        eprintln!("[display] using X11 backend (native X11 or XWayland)");
                        return Ok(event_loop);
                    }
                    Err(error) => {
                        return Err(format!(
                            "failed to create the X11 event loop: {error}\nhint: make sure DISPLAY points to a running X server or \
                             XWayland"
                        ));
                    }
                }
            }

            Err(
                "no usable display server found: neither WAYLAND_DISPLAY/WAYLAND_SOCKET (with a live compositor) nor DISPLAY is set"
                    .to_string(),
            )
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn create_event_loop() -> Result<EventLoop<()>, String> {
    EventLoop::new().map_err(|error| format!("failed to create the event loop: {error}"))
}

/// CTR+C (SIGINT), SIGHUP or SIGTERM was received, and the client is supposed
/// to close.
pub static SHUTDOWN_SIGNAL: AtomicBool = AtomicBool::new(false);

/// Signal handler that only requests a cooperative shutdown. It must not touch
/// anything except atomics, since it runs in signal context.
#[cfg(unix)]
unsafe extern "C" fn shutdown_signal_handler(_signal: libc::c_int) {
    SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
}

#[cfg(feature = "debug")]
pub(crate) const DEBUG_WINDOWS: &[WindowClass] = &[
    WindowClass::CacheStatistics,
    WindowClass::ClientStateInspector,
    WindowClass::PacketInspector,
    WindowClass::Profiler,
    WindowClass::RenderOptions,
];

// Create the `threads` module.
#[cfg(feature = "debug")]
ro_next_debug::create_profiler_threads!(threads, {
    Main,
    Loader,
});

pub fn init_tls_rand() {
    use std::random::*;
    let mut seed = [0; 32];
    DefaultRandomSource.fill_bytes(&mut seed);
    rand_aes::tls::rand_seed(seed.into());
}

pub(crate) fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Ctrl-C is not the only way the process can be asked to close: closing the
    // terminal sends SIGHUP, `kill`/`systemctl stop` sends SIGTERM. Without
    // handling these the process is terminated without running destructors, so
    // settings (e.g. remembered logins) are never written. Route both onto the
    // cooperative shutdown path so `Drop` handlers run.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, shutdown_signal_handler as *const () as usize);
        libc::signal(libc::SIGTERM, shutdown_signal_handler as *const () as usize);
    }
}

/// Time without a new experience packet after which the aggregated gain is
/// shown in the chat.
pub(crate) const EXPERIENCE_FEEDBACK_DELAY: Duration = Duration::from_millis(1500);

/// Experience amounts that are aggregated into one feedback line.
#[derive(Default)]
pub(crate) struct PendingExperience {
    base: i64,
    job: i64,
    quest_base: i64,
    quest_job: i64,
    last_packet: Option<Instant>,
}

impl PendingExperience {
    fn add(&mut self, amount: i64, experience_type: ExperienceType, experience_source: ExperienceSource) {
        match (experience_type, experience_source) {
            (ExperienceType::BaseExperience, ExperienceSource::Regular) => self.base += amount,
            (ExperienceType::BaseExperience, ExperienceSource::Quest) => self.quest_base += amount,
            (ExperienceType::JobExperience, ExperienceSource::Regular) => self.job += amount,
            (ExperienceType::JobExperience, ExperienceSource::Quest) => self.quest_job += amount,
        }

        self.last_packet = Some(Instant::now());
    }

    fn is_empty(&self) -> bool {
        self.base == 0 && self.job == 0 && self.quest_base == 0 && self.quest_job == 0
    }

    /// Clears and returns the four aggregated amounts (base, job, quest base,
    /// quest job), or [`None`] when nothing was gained.
    ///
    /// The chat text and its labels are localized by the caller
    /// (`flush_experience_feedback`), so this only reports the amounts.
    fn take_gains(&mut self) -> Option<[i64; 4]> {
        if self.is_empty() {
            self.last_packet = None;
            return None;
        }

        let gains = [self.base, self.job, self.quest_base, self.quest_job];

        self.base = 0;
        self.job = 0;
        self.quest_base = 0;
        self.quest_job = 0;
        self.last_packet = None;

        Some(gains)
    }
}

/// Formats a number with thousands separators, e.g. `12,345`.
pub(crate) fn format_amount(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }

    formatted
}

/// Wire value of [`BuyItemResult`] (`ZC_PC_PURCHASE_RESULT` result byte).
///
/// Only the protocol result code is mapped here; the user-facing texts live in
/// `data\client\messages.ron` (`MessageDomain::BuyItem`).
pub(crate) fn buy_item_result_code(result: &BuyItemResult) -> u16 {
    match result {
        // Handled by the outer `BuyingCompleted` match arm: a successful
        // purchase only closes the windows and never shows a message.
        BuyItemResult::Successful => unreachable!(),
        BuyItemResult::NotEoughZeny => 1,
        BuyItemResult::WeightLimitExceeded => 2,
        BuyItemResult::TooManyItems => 3,
        BuyItemResult::TooManyOfThisItem => 9,
        BuyItemResult::PropsOpenAir => 10,
        BuyItemResult::ExchangeFailed => 11,
        BuyItemResult::ExchangeWellDone => 12,
        BuyItemResult::ItemSoldOut => 13,
        BuyItemResult::NotEnoughGoods => 14,
    }
}

/// Wire value of [`AmmunitionActionType`] (`ZC_ACTION_FAILURE` result, u16).
///
/// Only the protocol result code is mapped here; the user-facing texts live in
/// `data\client\messages.ron` (`MessageDomain::AmmunitionAction`).
pub(crate) fn ammunition_action_code(action: &AmmunitionActionType) -> u16 {
    match action {
        AmmunitionActionType::EquipProperAmmunitionFirst => 0,
        AmmunitionActionType::WeightLimitExceeded1 => 1,
        AmmunitionActionType::WeightLimitExceeded2 => 2,
        AmmunitionActionType::Equipped => 3,
    }
}

/// Loads the client message table and logs whether it was found.
///
/// `data/msgstringtable.txt` is part of the client archives; when it is missing
/// the client falls back to raw message ids instead of hiding the messages.
pub(crate) fn load_message_table(game_file_loader: &GameFileLoader) -> Option<msgstringtable::MessageTable> {
    let message_table = msgstringtable::MessageTable::load(game_file_loader);

    match &message_table {
        Some(table) => crate::logging::log_debug!("msgstringtable: {} entries loaded", table.len()),
        None => crate::logging::log_warn!("data/msgstringtable.txt not found in the archives; system messages fall back to raw ids"),
    }

    message_table
}

/// Loads the archive override of the data-driven protocol result-code message
/// table.
///
/// The client always has the compiled fallback (`ro-next messages` crate,
/// `include_str!` of `data\client\messages.ron`), so a missing or malformed
/// archive override only logs a warning and never leaves result messages
/// unavailable.
pub(crate) fn load_protocol_messages(game_file_loader: &GameFileLoader) {
    use ro_next_loaders::FileLoader;

    let bytes = match game_file_loader.get(ro_next_messages::DATA_PATH) {
        Ok(bytes) => bytes,
        Err(_) => {
            crate::logging::log_debug!(
                "messages: {} not found in the archives, using the compiled fallback",
                ro_next_messages::DATA_PATH
            );
            return;
        }
    };

    let content = String::from_utf8_lossy(&bytes);

    match ro_next_messages::install(&content) {
        Ok(count) => crate::logging::log_debug!("messages: loaded {count} result messages from the archive"),
        Err(error) => crate::logging::log_warn!(
            "messages: invalid {} ({error}), using the compiled fallback",
            ro_next_messages::DATA_PATH
        ),
    }
}

pub struct Client {
    pub(crate) game_file_loader: Arc<GameFileLoader>,
    pub(crate) action_loader: Arc<ActionLoader>,
    #[cfg(feature = "debug")]
    pub(crate) animation_loader: Arc<AnimationLoader>,
    pub(crate) async_loader: Arc<AsyncLoader>,
    pub(crate) effect_loader: Arc<EffectLoader>,
    pub(crate) font_loader: Arc<FontLoader>,
    #[cfg(feature = "debug")]
    pub(crate) map_loader: Arc<MapLoader>,
    pub(crate) sprite_loader: Arc<SpriteLoader>,
    pub(crate) texture_loader: Arc<TextureLoader>,
    pub(crate) library: Arc<Library>,

    pub(crate) interface_renderer: InterfaceRenderer,
    pub(crate) bottom_interface_renderer: GameInterfaceRenderer,
    pub(crate) middle_interface_renderer: GameInterfaceRenderer,
    pub(crate) top_interface_renderer: GameInterfaceRenderer,
    pub(crate) effect_renderer: EffectRenderer,
    #[cfg(feature = "debug")]
    pub(crate) debug_marker_renderer: DebugMarkerRenderer,
    #[cfg(feature = "debug")]
    pub(crate) aabb_instructions: Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    pub(crate) circle_instructions: Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    pub(crate) rectangle_instructions: Vec<DebugRectangleInstruction>,
    pub(crate) model_batches: Vec<ModelBatch>,
    pub(crate) model_instructions: Vec<ModelInstruction>,
    pub(crate) entity_instructions: Vec<EntityInstruction>,
    pub(crate) directional_shadow_model_batches: [Vec<ModelBatch>; PARTITION_COUNT],
    pub(crate) directional_shadow_model_instructions: Vec<ModelInstruction>,
    pub(crate) directional_shadow_entity_instructions: [Vec<EntityInstruction>; PARTITION_COUNT],
    pub(crate) point_shadow_model_batches: Vec<ModelBatch>,
    pub(crate) point_shadow_model_instructions: Vec<ModelInstruction>,
    pub(crate) point_shadow_entity_instructions: Vec<EntityInstruction>,
    pub(crate) point_light_with_shadow_instructions: Vec<PointLightWithShadowInstruction>,
    pub(crate) point_light_instructions: Vec<PointLightInstruction>,

    pub(crate) input_system: InputSystem,

    pub(crate) interface: Interface<'static, ClientState>,
    pub(crate) mouse_cursor: MouseCursor,
    pub(crate) show_interface: bool,
    pub(crate) game_timer: GameTimer,

    #[cfg(feature = "debug")]
    pub(crate) debug_camera: DebugCamera,
    pub(crate) start_camera: StartCamera,
    pub(crate) player_camera: PlayerCamera,
    pub(crate) directional_shadow_camera: DirectionalShadowCamera,
    pub(crate) directional_shadow_partitions: Arc<Mutex<[DirectionalShadowPartition; PARTITION_COUNT]>>,
    pub(crate) point_shadow_camera: PointShadowCamera,

    pub(crate) input_event_buffer: Vec<InputEvent>,
    pub(crate) network_event_buffer: NetworkEventBuffer,
    // TODO: Move or remove this.
    pub(crate) saved_login_data: Option<LoginServerLoginData>,
    // TODO: Move or remove this.
    pub(crate) saved_character_server: Option<CharacterServerInformation>,
    // TODO: Move or remove this.
    pub(crate) saved_login_server_address: Option<SocketAddr>,
    // TODO: Move or remove this.
    pub(crate) saved_password: String,
    // TODO: Move or remove this.
    pub(crate) saved_username: String,
    // TODO: Move or remove this.
    pub(crate) saved_packet_version: SupportedPacketVersion,

    /// Stat updates that arrived before the local player entity existed.
    ///
    /// The server sends its initial stat burst (including the values needed for
    /// the experience bars) right after the map load. If those updates are
    /// handled before the player entity was created they are buffered here and
    /// applied as soon as the player exists (see `apply_pending_stat_updates`).
    pub(crate) pending_stat_updates: PendingStatUpdates,

    /// Last tile position the minimap marker was updated with, used to derive
    /// the direction the player is facing from the movement delta.
    pub(crate) minimap_last_tile: Option<TilePosition>,

    /// Explicit state of the `M` key window cycle: small minimap -> big map ->
    /// closed -> small minimap. The windows themselves are still the source of
    /// truth (a window can be closed with its X button), the cycle re-syncs
    /// with the actually open windows when the key is pressed.
    /// Shared style cell read by all running level-99 aura effects, so a style
    /// change in the interface settings applies live without respawning them.
    pub(crate) level_99_aura_style_cell: Arc<AtomicU8>,
    /// Entities that currently have a level-99 aura effect attached.
    pub(crate) level_99_aura_entities: HashSet<EntityId>,

    /// Status effects with a persistent glow
    /// ([`crate::world::status_aura_spec`]) that currently have an aura
    /// effect attached, keyed by entity and status id. The value is the
    /// shared "still active" flag (cleared when the status ends, which
    /// removes the effect on its next update) and the last reconciliation
    /// frame seen by [`Self::update_status_auras`].
    pub(crate) status_aura_flags: HashMap<(EntityId, u16), (Arc<AtomicBool>, u64)>,
    /// Frame counter of the status aura reconciliation (used to detect entries
    /// whose status is no longer active without allocating a set per frame).
    pub(crate) status_aura_frame: u64,
    /// Active looping/ground status visuals, keyed by (entity, status). The
    /// value is the shared "still active" flag and the last reconciliation
    /// frame, exactly like `status_aura_flags`.
    pub(crate) status_visual_flags: HashMap<(ragnarok_packets::EntityId, u16), (std::sync::Arc<std::sync::atomic::AtomicBool>, u64)>,
    pub(crate) status_visual_frame: u64,

    /// Skill sounds that were already played at the start of an *instant* skill
    /// Decides when a skill sound plays: instant casts wait for their
    /// completion/damage packet instead of playing a pre-cast sound when the
    /// cast is only acknowledged, and duplicate completion packets are
    /// collapsed (see [`SkillSoundGate`]).
    pub(crate) skill_sound_gate: SkillSoundGate,

    pub(crate) map_window_cycle: MapWindowCycle,

    /// Explicit session/connection lifecycle (login → character selection →
    /// map entry → in game → recovery → shutdown). Owns the map entry/abort,
    /// map load retry, character-server reconnect and shutdown timers that used
    /// to be loose fields on the client.
    pub(crate) session: SessionMachine,

    /// Client message table (`data/msgstringtable.txt`) for `ZC_MSG` (0x0291)
    /// system messages.
    pub(crate) message_table: Option<msgstringtable::MessageTable>,

    /// Aggregated experience gains. rAthena sends one `ZC_NOTIFY_EXP2` packet
    /// per kill (and per party member share), so the amounts are summed and
    /// shown as a single chat line after a short idle period.
    pub(crate) pending_experience: PendingExperience,

    /// Rate limiter for non-modal "some resources could not be loaded" chat
    /// notices (transient asset failures; the warn log has no such limit).
    pub(crate) transient_asset_notices: TransientAssetNoticeLimiter,
    /// Explicit per-entity lifecycle: first spawn vs. duplicate `AddEntity`
    /// (job change/respawn), death/despawn and warp cleanup.
    pub(crate) entity_lifecycle: EntityLifecycle,

    /// Context menu that should be opened once the name of the target entity
    /// is known. Stores the entity id and the screen position of the click.
    pub(crate) pending_context_menu: Option<(EntityId, ScreenPosition)>,

    /// Whether an NPC shop is currently open on the server (i.e. the client has
    /// received the shop item list and has not closed the shop yet). Used to
    /// send the close-shop packet when the shop windows are closed by the user
    /// so that the map server does not keep the NPC interaction (and the
    /// movement lock) alive.
    pub(crate) shop_open: bool,

    /// Loaded guild emblem textures, keyed by guild id. Requests are sent when
    /// a player's guild name becomes known (the entity-appear packets only
    /// carry the guild id) and are rate-limited to one request per guild
    /// per map session.
    pub(crate) guild_emblems: GuildEmblemCache,

    /// Deadline/retry tracking for asynchronous animation and asset loads of
    /// visible entities. A load that never completes or fails is retried a
    /// bounded number of times instead of silently leaving the resource
    /// missing forever.
    pub(crate) asset_retries: ResourceRetryQueue<LoaderId>,

    /// Name of the map that is currently loaded. Used to detect same-map
    /// teleports (fly wing, AL_TELEPORT, ...): the server sends a normal map
    /// move packet even when the destination is the current map, but reloading
    /// the whole map for that would hide the teleport and stall the client.
    pub(crate) current_map_name: Option<String>,

    pub(crate) particle_holder: ParticleHolder,
    /// Lazily loaded flying-projectile sprite/action pairs, keyed by the
    /// projectile kind from the weapon data file. `None` marks a projectile
    /// whose sprites are missing, so the failed load is not retried per hit.
    pub(crate) projectile_assets: HashMap<crate::world::weapon::ProjectileKind, Option<Arc<ArrowAssets>>>,
    pub(crate) point_light_manager: PointLightManager,
    pub(crate) effect_holder: EffectHolder,
    pub(crate) chat_bubbles: ChatBubbles,
    /// Official status icon textures drawn above entity heads (`opt1`/`opt2`).
    pub(crate) head_marks: HeadMarks,
    /// Pending item drag: (press time, press position). A drag only starts
    /// after a short hold or a few pixels of movement, so a plain click
    /// does not pick items up.
    pub(crate) pending_item_drag: Option<(std::time::Instant, ScreenPosition)>,
    pub(crate) skill_name_texts: SkillNameTexts,
    pub(crate) cast_bars: CastBars,
    /// De-duplicates the completion effect of one skill cast (ground-skill,
    /// no-damage and damage packets can all describe the same cast).
    pub(crate) skill_completion_tracker: SkillCompletionTracker,
    /// When the after-cast act delay of the last own skill ends, in client
    /// ticks. Server table values; paces follow-up skill/item actions only.
    pub(crate) aftercast_act_until: Option<ClientTick>,
    pub(crate) warp_portal: Option<WarpPortal>,
    /// Active map weather (`ZC_NOTIFY_EFFECT2` snow/fog/cloud/...).
    pub(crate) weather: Option<crate::world::Weather>,
    /// Classic spirit-sphere texture (`effect\thunder_center.bmp`), drawn as
    /// orbiting blue-white balls around entities with active spirit spheres.
    pub(crate) spirit_sphere_texture: Option<Arc<Texture>>,
    /// Sound of the entity teleport/logout disappear effect (data-driven, see
    /// `client_config::ui_audio`; `None` disables it).
    pub(crate) teleport_vanish_sound_effect: Option<SoundEffectKey>,
    /// Spawn times of recently started teleport-vanish effects (burst limiter).
    pub(crate) recent_teleport_vanishes: Vec<Instant>,
    pub(crate) path_finder: PathFinder,

    pub(crate) point_light_set_buffer: ResourceSetBuffer<LightSourceKey>,
    pub(crate) directional_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    pub(crate) point_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    pub(crate) deferred_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    pub(crate) bounding_box_object_set_buffer: ResourceSetBuffer<ObjectKey>,

    #[cfg(feature = "debug")]
    pub(crate) pathing_texture_set: Arc<TextureSet>,
    #[cfg(feature = "debug")]
    pub(crate) tile_texture_set: Arc<TextureSet>,

    /// Interface/button click sound (data-driven; `None` disables it).
    pub(crate) main_menu_click_sound_effect: Option<SoundEffectKey>,

    #[cfg(feature = "debug")]
    pub(crate) networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    pub(crate) networking_system: NetworkingSystem<ClientPacketCallback>,
    pub(crate) audio_engine: Arc<AudioEngine<GameFileLoader>>,
    pub(crate) skill_sounds: SkillSoundPlayer,
    pub(crate) active_master_volume: f32,
    pub(crate) active_sound_effect_volume: f32,
    pub(crate) active_background_music_volume: f32,
    pub(crate) active_interface_settings: InterfaceSettings,
    pub(crate) active_graphics_settings: GraphicsSettings,
    pub(crate) graphics_engine: GraphicsEngine,
    pub(crate) queue: Queue,
    #[cfg(feature = "debug")]
    pub(crate) device: Device,
    pub(crate) window: Option<Arc<Window>>,
    /// Whether the interface scaling should be initialized from the DPI scale
    /// factor of the window. This is only done on the first start, i.e. when
    /// there is no saved interface settings file yet.
    pub(crate) initialize_interface_scaling_from_dpi: bool,
    /// Scale factor that was last reported by the windowing system.
    ///
    /// Used to keep the last known pointer position in the coordinate space of
    /// the new scale when the compositor changes the scale factor (native
    /// Wayland fractional scaling, moving the window between outputs with
    /// different scales), so that clicks arriving before the next pointer
    /// motion event are hit-tested correctly.
    pub(crate) last_scale_factor: f64,

    /// Whether the client window is backed by the native Wayland backend.
    pub(crate) backend_is_wayland: bool,
    /// Whether presentation is currently paused because the native Wayland
    /// window is in the background (see [`Self::should_pause_presentation`]).
    pub(crate) presentation_paused: bool,
    /// Whether at least one frame was presented already.
    ///
    /// The first frame has to be presented even when the window is in the
    /// background: without a committed buffer the compositor never maps the
    /// window, so the client would be invisible. Only after that it is safe to
    /// stop presenting while the window is hidden.
    pub(crate) has_presented_frame: bool,
    /// Whether the window is currently reported as occluded by the windowing
    /// system. Occluded windows receive no redraw events on some platforms, but
    /// the state is applied as soon as the client renders again.
    pub(crate) window_occluded: bool,
    /// Whether the window was minimized the last time we checked. Querying this
    /// state is a synchronous round trip on X11, so it is only refreshed when
    /// the focus or the occlusion state of the window changes.
    pub(crate) window_minimized: bool,
    /// Set through `RO_NEXT_FORCE_UNFOCUSED`/`RO_NEXT_UNFOCUSED_ALWAYS`.
    /// Forces the background frame rate limit, which is useful for headless
    /// verification clients.
    pub(crate) force_background: bool,
    /// The frame rate limit that is currently applied to the graphics engine.
    /// `0` means unlimited.
    pub(crate) active_frames_per_second_limit: u32,
    /// When the effective frame rate should be logged next. Only set when
    /// frame rate logging is enabled through `RO_NEXT_FPS_LOG`.
    pub(crate) frames_per_second_log_deadline: Option<Instant>,
    /// When the next layout/text cache performance line should be logged. Only
    /// set when profiling logging is enabled through `RO_NEXT_PERF_LOG`.
    pub(crate) performance_log_deadline: Option<Instant>,
    /// Set when UI state was changed while the interface frame was alive (where
    /// `self.interface` cannot be borrowed again). Applied at the start of the
    /// next frame.
    pub(crate) interface_layout_invalidation_pending: bool,

    pub(crate) map: Option<Arc<Map>>,
    pub(crate) client_state: State<ClientState>,
}

/// Whether an incoming `ChangeMap` packet with the same map name is a
/// same-map teleport (fly wing, `AL_TELEPORT`, ...) that can be applied without
/// loading the map data again.
///
/// This must be false for a fresh character map entry: after a logout the
/// client keeps the name of the last map (for the "same map" teleport check)
/// but has no loaded map and is in `MapLoading`. Treating that case as a
/// teleport used to skip the map load and left the client on a black screen
/// until the map entry watchdog fired.
pub(crate) fn is_same_map_teleport(
    current_map_name: Option<&str>,
    incoming_map_name: &str,
    has_loaded_map: bool,
    session_state: SessionState,
) -> bool {
    current_map_name == Some(incoming_map_name) && has_loaded_map && session_state == SessionState::InGame
}

impl Client {}

/// Returns `true` if the given environment variable is set to a truthy value.
pub(crate) fn environment_flag(variable: &str) -> bool {
    std::env::var(variable).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// The deadline of the first log line for `RO_NEXT_FPS_LOG`, if enabled.
pub(crate) fn frames_per_second_log_deadline() -> Option<Instant> {
    if environment_flag(FRAMERATE_LOG_ENVIRONMENT_VARIABLE) {
        Some(Instant::now() + Duration::from_secs(FRAMERATE_LOG_INTERVAL_SECONDS))
    } else {
        None
    }
}

/// Environment variable that enables the periodic layout/text cache profiling
/// line (`[perf] ...`).
pub(crate) const PERFORMANCE_LOG_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_PERF_LOG";
/// Seconds between two `[perf]` lines.
pub(crate) const PERFORMANCE_LOG_INTERVAL_SECONDS: u64 = 2;

/// The deadline of the first `[perf]` line, if profiling logging is enabled.
pub(crate) fn performance_log_deadline() -> Option<Instant> {
    if environment_flag(PERFORMANCE_LOG_ENVIRONMENT_VARIABLE) {
        Some(Instant::now() + Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS))
    } else {
        None
    }
}

/// Frame phase profiling counters for `RO_NEXT_PERF_LOG`.
pub(crate) static FRAME_PROFILE_FRAMES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static FRAME_PROFILE_WAIT_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static FRAME_PROFILE_SIM_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static FRAME_PROFILE_INTERFACE_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static FRAME_PROFILE_WORLD_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub(crate) static FRAME_PROFILE_GPU_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Snapshot of the frame phase profiling counters.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FramePhaseProfile {
    frames: u64,
    wait_nanos: u64,
    sim_nanos: u64,
    interface_nanos: u64,
    world_nanos: u64,
    gpu_nanos: u64,
}

pub(crate) fn take_frame_phase_profile() -> FramePhaseProfile {
    use std::sync::atomic::Ordering::Relaxed;

    FramePhaseProfile {
        frames: FRAME_PROFILE_FRAMES.swap(0, Relaxed),
        wait_nanos: FRAME_PROFILE_WAIT_NANOS.swap(0, Relaxed),
        sim_nanos: FRAME_PROFILE_SIM_NANOS.swap(0, Relaxed),
        interface_nanos: FRAME_PROFILE_INTERFACE_NANOS.swap(0, Relaxed),
        world_nanos: FRAME_PROFILE_WORLD_NANOS.swap(0, Relaxed),
        gpu_nanos: FRAME_PROFILE_GPU_NANOS.swap(0, Relaxed),
    }
}

pub(crate) fn record_frame_phase(
    wait: std::time::Duration,
    sim: std::time::Duration,
    interface: std::time::Duration,
    world: std::time::Duration,
    gpu: std::time::Duration,
    phase_allocations: [u64; 4],
) {
    use std::sync::atomic::Ordering::Relaxed;

    crate::perf::record_frame(
        wait.as_nanos() as u64,
        sim.as_nanos() as u64,
        interface.as_nanos() as u64,
        world.as_nanos() as u64,
        gpu.as_nanos() as u64,
        phase_allocations,
    );

    FRAME_PROFILE_FRAMES.fetch_add(1, Relaxed);
    FRAME_PROFILE_WAIT_NANOS.fetch_add(wait.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_SIM_NANOS.fetch_add(sim.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_INTERFACE_NANOS.fetch_add(interface.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_WORLD_NANOS.fetch_add(world.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_GPU_NANOS.fetch_add(gpu.as_nanos() as u64, Relaxed);
}

/// Logs the one-line hint when the client was restarted by the presentation
/// watchdog.
pub fn log_presentation_fallback_startup_line() {
    graphics::presentation_watchdog::log_fallback_startup_line();
}

/// Whether the given window is backed by the native Wayland backend.
pub(crate) fn window_is_wayland(window: &Window) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    matches!(
        window.window_handle().map(|handle| handle.as_raw()),
        Ok(RawWindowHandle::Wayland(_))
    )
}

/// Whether the given job may move while casting because of the Sage/Scholar/
/// Professor `Free Cast` passive.
///
/// Both sides are data: the learned skill has to carry the
/// `move_while_casting` behaviour flag (`skills.ron`, Free Cast 278) and the
/// job has to carry the `move_while_casting` capability (`jobs.ron`, Sage 16 /
/// Professor 4017). A passive that a job cannot learn therefore never applies,
/// and an unknown job or skill list defaults to "rooted" (the safe behaviour).
pub(crate) fn can_move_while_casting(job_id: JobId, skills: &[LearnedSkill]) -> bool {
    crate::world::jobs::can_move_while_casting(job_id)
        && skills
            .iter()
            .any(|skill| crate::world::skills::grants_move_while_casting(skill.skill_id))
}

impl Client {}

/// Requests the equipment animation layers (weapon, shield, robe) for
/// `entity` through the data-driven appearance modules and applies the layers
/// that are already available.
///
/// Every packet path that changes the body/equipment appearance (character
/// login, entity spawn, job change, equipment change) goes through this
/// function. The candidate paths come from [`crate::world::weapon`],
/// [`crate::world::shield`] and [`crate::world::robe`], so no call site knows
/// weapon types, item view ids or sprite suffixes.
pub(crate) fn request_equipment_layers(async_loader: &AsyncLoader, entity: &mut Entity) {
    request_weapon_animation(async_loader, entity);
    request_shield_animation(async_loader, entity);
    request_robe_animation(async_loader, entity);
}

/// Requests the equipped weapon animation layer for `entity`.
pub(crate) fn request_weapon_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let profile = crate::world::weapon::profile(entity.get_weapon());
    let candidates = entity.get_weapon_part_candidates();

    crate::logging::log_debug!(
        "weapon layer: entity {} kind {:?} ({}) range {} ammo {:?} -> {} candidate(s)",
        entity.get_entity_id().0,
        profile.kind(),
        profile.subtype(),
        profile.attack_range(),
        profile.ammo(),
        candidates.len()
    );

    // A player without any candidate path would silently lose the weapon
    // layer; report it once per entity so a missing/custom weapon sprite is
    // visible in the normal log instead of a debug-only line.
    if candidates.is_empty() && entity.get_entity_type() == crate::world::EntityType::Player {
        crate::logging::log_warn!(
            "weapon layer: entity {} has no sprite candidates (weapon view id {}, kind {:?}, subtype {:?})",
            entity.get_entity_id().0,
            entity.get_weapon(),
            profile.kind(),
            profile.subtype()
        );
    }

    if candidates.is_empty() || entity.has_weapon_animation() {
        return;
    }

    if let Some(weapon_animation) = async_loader.request_weapon_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_weapon_animation(weapon_animation);
    }
}

/// Requests the equipped shield animation layer for `entity`.
pub(crate) fn request_shield_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let candidates = entity.get_shield_part_candidates();

    if candidates.is_empty() || entity.has_shield_animation() {
        return;
    }

    if let Some(shield_animation) = async_loader.request_shield_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_shield_animation(shield_animation);
    }
}

/// Requests the equipped robe animation layer for `entity`.
pub(crate) fn request_robe_animation(async_loader: &AsyncLoader, entity: &mut Entity) {
    let candidates = entity.get_robe_part_candidates();

    if candidates.is_empty() || entity.has_robe_animation() {
        return;
    }

    if let Some(robe_animation) = async_loader.request_robe_animation_data_load(entity.get_entity_id(), candidates) {
        entity.set_robe_animation(robe_animation);
    }
}
