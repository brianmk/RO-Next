//! The player-specific entity state and behaviour (`Player`).

use super::*;

#[derive(Clone, RustState, StateWindow)]
pub struct Player {
    pub(crate) common: Common,
    pub hair_id: usize,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub base_level: usize,
    pub job_level: usize,
    pub stat_points: u32,
    pub strength: i32,
    pub bonus_strength: i32,
    pub strength_stat_points_cost: u8,
    pub agility: i32,
    pub bonus_agility: i32,
    pub agility_stat_points_cost: u8,
    pub vitality: i32,
    pub bonus_vitality: i32,
    pub vitality_stat_points_cost: u8,
    pub intelligence: i32,
    pub bonus_intelligence: i32,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: i32,
    pub bonus_dexterity: i32,
    pub dexterity_stat_points_cost: u8,
    pub luck: i32,
    pub bonus_luck: i32,
    pub luck_stat_points_cost: u8,
    /// Attack motion (server `SP_ASPD`), in milliseconds.
    pub attack_speed: u32,
    /// Current attack range (`ZC_ATTACK_RANGE`, 0x013A).
    pub attack_range: AttackRange,
    /// Weight percentage above which the weight should be shown in red
    /// (`ZC_NOTIFY_CRITICALWEIGHT`, 0x0ADE; rAthena
    /// `natural_heal_weight_rate`).
    pub critical_weight_percentage: u32,
    /// Merchant cart item count/weight (`ZC_NOTIFY_CARTITEM_COUNTINFO`, 0x0121
    /// and `SP_CARTINFO` through 0x00B0).
    pub cart_items: u16,
    pub cart_maximum_items: i16,
    pub cart_weight: u32,
    pub cart_maximum_weight: u32,
    pub skill_points: u32,
    /// Base attack including the weapon attack (server `SP_ATK1`).
    pub attack: u32,
    /// Weapon refine attack bonus / damage variance (server `SP_ATK2`).
    pub bonus_attack: u32,
    /// Minimum magic attack (server `SP_MATK2`).
    pub magic_attack_min: u32,
    /// Maximum magic attack (server `SP_MATK1`).
    pub magic_attack_max: u32,
    /// Equipment defense (server `SP_DEF1`).
    pub defense: u32,
    /// Vitality based defense (server `SP_DEF2`).
    pub bonus_defense: u32,
    /// Equipment magic defense (server `SP_MDEF1`).
    pub magic_defense: u32,
    /// Intelligence based magic defense (server `SP_MDEF2`).
    pub bonus_magic_defense: u32,
    /// Hit rate (server `SP_HIT`).
    pub hit: u32,
    /// Flee rate (server `SP_FLEE1`).
    pub flee: u32,
    /// Perfect dodge (server `SP_FLEE2`, already divided by 10).
    pub perfect_dodge: u32,
    /// Critical rate (server `SP_CRITICAL`, already divided by 10).
    pub critical: u32,
    /// Carried weight (server `SP_WEIGHT`).
    pub weight: u32,
    /// Maximum carried weight (server `SP_MAXWEIGHT`).
    pub maximum_weight: u32,
    /// Base experience of the character (server `SP_BASEEXP`).
    pub base_experience: u64,
    /// Base experience needed for the next base level (server
    /// `SP_NEXTBASEEXP`).
    pub next_base_experience: u64,
    /// Job experience of the character (server `SP_JOBEXP`).
    pub job_experience: u64,
    /// Job experience needed for the next job level (server `SP_NEXTJOBEXP`).
    pub next_job_experience: u64,
}

impl Player {
    /// Current movement speed (`SP_SPEED` / `ZC_NOTIFY_MOVESPEED`), used by the
    /// status window's Speed line.
    pub fn movement_speed(&self) -> usize {
        self.common.movement_speed
    }

    /// This function creates the player entity free-floating in the
    /// "void". When a new map is loaded on map change, the server sends
    /// the correct position we need to position the player to.
    pub fn new(library: &Library, account_id: AccountId, character_information: &CharacterInformation, client_tick: ClientTick) -> Self {
        let hair_id = character_information.head as usize;
        let spell_points = character_information.spell_points as usize;
        let activity_points = 0;
        let maximum_spell_points = character_information.maximum_spell_points as usize;
        let maximum_activity_points = 0;
        let base_level = character_information.base_level as usize;
        let job_level = character_information.job_level as usize;
        let stat_points = character_information.stat_points as u32;

        let entity_data = EntityData::from_character(account_id, character_information, WorldPosition::origin());
        let tile_position = TilePosition::new(0, 0);
        let position = Point3::origin();

        let mut common = Common::new(library, &entity_data, tile_position, position, client_tick);
        // Player's own character should not fade in.
        common.fade_state = FadeState::Opaque;

        Self {
            common,
            hair_id,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
            base_level,
            job_level,
            stat_points,
            strength: character_information.strength as i32,
            bonus_strength: 0,
            strength_stat_points_cost: 0,
            agility: character_information.agility as i32,
            bonus_agility: 0,
            agility_stat_points_cost: 0,
            vitality: character_information.vitality as i32,
            bonus_vitality: 0,
            vitality_stat_points_cost: 0,
            intelligence: character_information.intelligence as i32,
            bonus_intelligence: 0,
            intelligence_stat_points_cost: 0,
            dexterity: character_information.dexterity as i32,
            bonus_dexterity: 0,
            dexterity_stat_points_cost: 0,
            luck: character_information.luck as i32,
            bonus_luck: 0,
            luck_stat_points_cost: 0,
            attack_speed: 0,
            attack_range: AttackRange(1),
            critical_weight_percentage: 0,
            cart_items: 0,
            cart_maximum_items: 0,
            cart_weight: 0,
            cart_maximum_weight: 0,
            skill_points: 0,
            attack: 0,
            bonus_attack: 0,
            magic_attack_min: 0,
            magic_attack_max: 0,
            defense: 0,
            bonus_defense: 0,
            magic_defense: 0,
            bonus_magic_defense: 0,
            hit: 0,
            flee: 0,
            perfect_dodge: 0,
            critical: 0,
            weight: 0,
            maximum_weight: 0,
            base_experience: 0,
            next_base_experience: 0,
            job_experience: 0,
            next_job_experience: 0,
        }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    /// Current health points (server `SP_HP`).
    #[inline(always)]
    pub fn health_points(&self) -> usize {
        self.common.health_points
    }

    /// Maximum health points (server `SP_MAXHP`).
    #[inline(always)]
    pub fn maximum_health_points(&self) -> usize {
        self.common.maximum_health_points
    }

    /// Base experience progress in percent (0.0 - 100.0).
    ///
    /// Returns `None` if the server did not tell us how much experience is
    /// needed for the next base level yet.
    pub fn base_experience_percent(&self) -> Option<f64> {
        match self.next_base_experience {
            0 => None,
            next => Some(self.base_experience as f64 / next as f64 * 100.0),
        }
    }

    /// Job experience progress in percent (0.0 - 100.0).
    ///
    /// Returns `None` if the server did not tell us how much experience is
    /// needed for the next job level yet.
    pub fn job_experience_percent(&self) -> Option<f64> {
        match self.next_job_experience {
            0 => None,
            next => Some(self.job_experience as f64 / next as f64 * 100.0),
        }
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_stat(&mut self, stat_type: StatType) {
        match stat_type {
            StatType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatType::HealthPoints(value) => self.common.health_points = value as usize,
            StatType::SpellPoints(value) => self.spell_points = value as usize,
            StatType::ActivityPoints(value) => self.activity_points = value as usize,
            StatType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            StatType::MovementSpeed(value) => self.common.movement_speed = value as usize,
            StatType::BaseLevel(value) => self.base_level = value as usize,
            StatType::JobLevel(value) => self.job_level = value as usize,
            StatType::StatPoints(stat_points) => self.stat_points = stat_points,
            StatType::Strength(base, bonus) => {
                self.strength = base;
                self.bonus_strength = bonus;
            }
            StatType::Agility(base, bonus) => {
                self.agility = base;
                self.bonus_agility = bonus;
            }
            StatType::Vitality(base, bonus) => {
                self.vitality = base;
                self.bonus_vitality = bonus;
            }
            StatType::Intelligence(base, bonus) => {
                self.intelligence = base;
                self.bonus_intelligence = bonus;
            }
            StatType::Dexterity(base, bonus) => {
                self.dexterity = base;
                self.bonus_dexterity = bonus;
            }
            StatType::Luck(base, bonus) => {
                self.luck = base;
                self.bonus_luck = bonus;
            }
            StatType::StrengthStatPointCost(cost) => self.strength_stat_points_cost = cost,
            StatType::AgilityStatPointCost(cost) => self.agility_stat_points_cost = cost,
            StatType::VitalityStatPointCost(cost) => self.vitality_stat_points_cost = cost,
            StatType::IntelligenceStatPointCost(cost) => self.intelligence_stat_points_cost = cost,
            StatType::DexterityStatPointCost(cost) => self.dexterity_stat_points_cost = cost,
            StatType::LuckStatPointCost(cost) => self.luck_stat_points_cost = cost,
            StatType::AttackSpeed(attack_speed) => self.attack_speed = attack_speed,
            StatType::SkillPoints(skill_points) => self.skill_points = skill_points,
            StatType::Attack1(attack) => self.attack = attack,
            StatType::Attack2(attack) => self.bonus_attack = attack,
            StatType::MagicAttack1(magic_attack) => self.magic_attack_max = magic_attack,
            StatType::MagicAttack2(magic_attack) => self.magic_attack_min = magic_attack,
            StatType::Defense1(defense) => self.defense = defense,
            StatType::Defense2(defense) => self.bonus_defense = defense,
            StatType::MagicDefense1(magic_defense) => self.magic_defense = magic_defense,
            StatType::MagicDefense2(magic_defense) => self.bonus_magic_defense = magic_defense,
            StatType::Hit(hit) => self.hit = hit,
            StatType::Flee1(flee) => self.flee = flee,
            StatType::Flee2(flee) => self.perfect_dodge = flee,
            StatType::Critical(critical) => self.critical = critical,
            StatType::Weight(weight) => self.weight = weight,
            StatType::MaximumWeight(maximum_weight) => self.maximum_weight = maximum_weight,
            StatType::BaseExperience(base_experience) => self.base_experience = base_experience,
            StatType::NextBaseExperience(next_base_experience) => self.next_base_experience = next_base_experience,
            StatType::JobExperience(job_experience) => self.job_experience = job_experience,
            StatType::NextJobExperience(next_job_experience) => self.next_job_experience = next_job_experience,
            StatType::CartInfo(items, weight, maximum_weight) => {
                self.cart_items = items;
                self.cart_weight = weight;
                self.cart_maximum_weight = maximum_weight;
            }
            _ => {}
        }
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.player_bar_width;
        let gap = theme.status_bar.gap;
        let total_height =
            theme.status_bar.health_height + theme.status_bar.spell_point_height + theme.status_bar.activity_point_height + gap * 2.0;

        let mut offset = 0.0;

        let background_position = final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0);

        let background_size = ScreenSize {
            width: bar_width,
            height: total_height,
        } + theme.status_bar.border_size * 2.0;

        renderer.render_rectangle(background_position, background_size, theme.status_bar.background_color);

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.health_height,
            },
            theme.status_bar.player_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );

        offset += gap + theme.status_bar.health_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.spell_point_height,
            },
            theme.status_bar.spell_point_color,
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );

        offset += gap + theme.status_bar.spell_point_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.activity_point_height,
            },
            theme.status_bar.activity_point_color,
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<EntityPartFile> {
        let common = self.get_common();
        get_entity_part_files(
            library,
            common.entity_type,
            common.job_id,
            common.sex,
            Some(self.hair_id),
            common.head_palette,
            common.body_palette,
            common.accessory,
            common.accessory2,
            common.accessory3,
            common.effect_state,
        )
    }
}
