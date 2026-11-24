use std::cell::{Cell, UnsafeCell};

use ragnarok_packets::StatUpType;
use ro_next_interface::event::ClickHandler;
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Player, PlayerPathExt};

struct StatTextSelector<A> {
    bonus_path: A,
    last_value: Cell<Option<i32>>,
    text: UnsafeCell<String>,
}

impl<A> StatTextSelector<A> {
    pub fn new(bonus_path: A) -> Self {
        Self {
            bonus_path,
            last_value: Cell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for StatTextSelector<A>
where
    A: Path<ClientState, i32>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let bonus_value = self.bonus_path.follow_safe(state);

        unsafe {
            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *bonus_value) {
                *self.text.get() = format!("^000001{bonus_value:+}^000000");
                self.last_value.set(Some(*bonus_value));
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

struct CostTextSelector<A> {
    cost_path: A,
    last_value: UnsafeCell<Option<u8>>,
    text: UnsafeCell<String>,
}

impl<A> CostTextSelector<A> {
    pub fn new(cost_path: A) -> Self {
        Self {
            cost_path,
            last_value: UnsafeCell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<A> Selector<ClientState, String> for CostTextSelector<A>
where
    A: Path<ClientState, u8>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        let cost = self.cost_path.follow_safe(state);

        unsafe {
            let last_value = &mut *self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| *last != *cost) {
                *self.text.get() = match *cost {
                    0 => "^000001max^000000".to_string(),
                    cost => format!("+1 (^000001{cost}^000000)"),
                };

                *last_value = Some(*cost);
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

#[derive(Default)]
pub struct StatsWindow<A> {
    player_path: A,
}

impl<A> StatsWindow<A> {
    pub fn new(player_path: A) -> Self {
        Self { player_path }
    }
}

impl<A> CustomWindow<ClientState> for StatsWindow<A>
where
    A: Path<ClientState, Player>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Stats)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        fn display<A, F>(player_path: A, formatter: F) -> impl Selector<ClientState, String>
        where
            A: Path<ClientState, Player> + 'static,
            F: Fn(&Player) -> String + 'static,
        {
            ComputedSelector::new_default(move |state: &ClientState| formatter(player_path.follow_safe(state)))
        }

        fn disabled_cutoff<A, B>(stat_points_path: A, cost_path: B) -> impl Selector<ClientState, bool>
        where
            A: Path<ClientState, u32>,
            B: Path<ClientState, u8>,
        {
            ComputedSelector::new_default(move |state: &ClientState| {
                let stat_points = stat_points_path.follow_safe(state);
                let cost = cost_path.follow_safe(state);

                // The cost is 0 if the player is at the maximum level.
                *cost == 0 || *stat_points < *cost as u32
            })
        }

        struct NoopClick;
        impl ClickHandler<ClientState> for NoopClick {
            fn handle_click(&self, _: &State<ClientState>, _: &mut EventQueue<ClientState>) {}
        }

        macro_rules! summary_row {
            ($label:expr, $value:expr) => {
                split! {
                    children: (
                        text! {
                            text: $label,
                            color: ro_next_interface::theme::theme().text().highlight_color(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: $value,
                            horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                }
            };
        }

        macro_rules! stat_row {
            ($label:expr, $base:ident, $bonus:ident, $cost:ident, $variant:ident) => {
                split! {
                    children: (
                        text! {
                            text: $label,
                            color: ro_next_interface::theme::theme().text().highlight_color(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: PartialEqDisplaySelector::new(self.player_path.$base()),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: StatTextSelector::new(self.player_path.$bonus()),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text! {
                            text: CostTextSelector::new(self.player_path.$cost()),
                            horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "+",
                            disabled: disabled_cutoff(self.player_path.stat_points(), self.player_path.$cost()),
                            event: InputEvent::StatUp { stat_type: StatUpType::$variant { amount: 1 } },
                        },
                    ),
                }
            };
        }

        let left_summary = fragment! {
            gaps: 2.0,
            children: (
                summary_row!("⚔ Atk", display(self.player_path, |player| format!("{} + {}", player.attack, player.bonus_attack))),
                summary_row!("✦ Matk", display(self.player_path, |player| format!("{} ~ {}", player.magic_attack_min, player.magic_attack_max))),
                summary_row!("◈ Hit", display(self.player_path, |player| player.hit.to_string())),
                summary_row!("✹ Crit", display(self.player_path, |player| player.critical.to_string())),
                summary_row!("➶ Range", display(self.player_path, |player| player.attack_range.0.to_string())),
            ),
        };

        let right_summary = fragment! {
            gaps: 2.0,
            children: (
                summary_row!("⛨ Def", display(self.player_path, |player| format!("{} + {}", player.defense, player.bonus_defense))),
                summary_row!("✧ Mdef", display(self.player_path, |player| format!("{} + {}", player.magic_defense, player.bonus_magic_defense))),
                summary_row!("➹ Flee", display(self.player_path, |player| format!("{} + {}", player.flee, player.perfect_dodge))),
                summary_row!("⚡ Aspd", display(self.player_path, |player| player.attack_speed.to_string())),
                summary_row!("➟ Speed", display(self.player_path, |player| format!("{}%", player.movement_speed() * 100 / 150))),
            ),
        };

        let summary = split! {
            gaps: 14.0,
            children: (left_summary, right_summary),
        };

        let header = split! {
            children: (
                text! { text: "", overflow_behavior: OverflowBehavior::Shrink },
                text! {
                    text: "Base",
                    horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: "Bonus",
                    horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: "Cost",
                    horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! { text: "", overflow_behavior: OverflowBehavior::Shrink },
            ),
        };

        window! {
            title: client_state().localization().stats_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 360.0,
            maximum_width: 420.0,
            elements: (
                summary,
                header,
                stat_row!("Str", strength, bonus_strength, strength_stat_points_cost, Strength),
                stat_row!("Agi", agility, bonus_agility, agility_stat_points_cost, Agility),
                stat_row!("Vit", vitality, bonus_vitality, vitality_stat_points_cost, Vitality),
                stat_row!("Int", intelligence, bonus_intelligence, intelligence_stat_points_cost, Intelligence),
                stat_row!("Dex", dexterity, bonus_dexterity, dexterity_stat_points_cost, Dexterity),
                stat_row!("Luk", luck, bonus_luck, luck_stat_points_cost, Luck),
                split! {
                    gaps: 12.0,
                    children: (
                        split! {
                            children: (
                                text! {
                                    text: "Points Status",
                                    color: ro_next_interface::theme::theme().text().highlight_color(),
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                                text! {
                                    text: PartialEqDisplaySelector::new(self.player_path.stat_points()),
                                    horizontal_alignment: HorizontalAlignment::Right { offset: 4.0, border: 4.0 },
                                    overflow_behavior: OverflowBehavior::Shrink,
                                },
                            ),
                        },
                        button! {
                            text: "Details",
                            event: NoopClick,
                        },
                    ),
                },
            ),
        }
    }
}
