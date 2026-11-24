//! Monster database window (client-side data + Sense `ZC_MONSTER_INFO`).
//!
//! The searchable database is built at startup by
//! [`crate::world::library::monster_database`] from the client's navigation
//! monster table, the job identity table and the monster sprite aliases.
//! Clicking a result selects it and marks its first spawn map as navigation
//! destination.
//!
//! Drop tables are **not** part of the client data: the archives contain no
//! monster database, so drop items/rates would require the server DB and are
//! deliberately not shown. The window lists name, level, spawn maps and the
//! resolved sprite, and shows the full Sense result when the server sends
//! `ZC_MONSTER_INFO` (0x018C).

use std::sync::Arc;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::monster_database::MonsterDatabaseStatePathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{MonsterDatabase, MonsterEntry};

pub struct MonsterDatabaseWindow {
    database: Arc<MonsterDatabase>,
}

impl MonsterDatabaseWindow {
    pub fn new(database: Arc<MonsterDatabase>) -> Self {
        Self { database }
    }
}

/// Header with the search hint and the Sense result.
fn header_text(database: Arc<MonsterDatabase>) -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(move |state: &ClientState| {
        let database_state = client_state().monster_database().follow_safe(state);
        let mut text = format!(
            "{} monsters from the client navigation data / job identities. Drop tables need the server mob DB and are not available.",
            database.len()
        );

        if let Some(sense) = database_state.sense() {
            let name = database.entry(sense.job_id).map(|entry| entry.name.as_str()).unwrap_or("unknown");
            text.push_str(&format!("\n\nSense result: {} (#{})", name, sense.job_id.0));
            text.push_str(&format!(
                "\nLevel {}   HP {}   Size {}   Race {}   Element {}",
                sense.level,
                sense.health_points,
                sense.size_name(),
                sense.race_name(),
                sense.element_name(),
            ));
            text.push_str(&format!(
                "\nDEF {}   MDEF {}   Resistances: {}",
                sense.defense,
                sense.magic_defense,
                sense.resistance_summary(),
            ));
        }

        if !database_state.status().is_empty() {
            text.push('\n');
            text.push_str(database_state.status());
        }

        text
    })
}

/// Detail text of the selected monster (or the Sense result when the selected
/// id is not in the client table).
fn detail_text(database: Arc<MonsterDatabase>) -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(move |state: &ClientState| {
        let database_state = client_state().monster_database().follow_safe(state);

        if let Some(job_id) = database_state.selected()
            && let Some(entry) = database.entry(job_id)
        {
            return entry.detail_text();
        }

        if let Some(sense) = database_state.sense() {
            return format!(
                "#{}  {}\nLevel {}\nHP {}\nSize {}   Race {}   Element {}\nDEF {}   MDEF {}\nResistances: {}",
                sense.job_id.0,
                database.entry(sense.job_id).map(|entry| entry.name.as_str()).unwrap_or("unknown"),
                sense.level,
                sense.health_points,
                sense.size_name(),
                sense.race_name(),
                sense.element_name(),
                sense.defense,
                sense.magic_defense,
                sense.resistance_summary(),
            );
        }

        "Search for a monster, or cast Sense on one in game to load its full stats.".to_owned()
    })
}

impl CustomWindow<ClientState> for MonsterDatabaseWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::MonsterDatabase)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let search_path = client_state().monster_database().query();

        struct MonsterSearch;

        window! {
            title: client_state().localization().ui().monster_database_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 500.0,
            maximum_width: 760.0,
            maximum_height: 720.0,
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                    text! {
                        text: header_text(self.database.clone()),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: detail_text(self.database.clone()),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text_box! {
                        ghost_text: "Search monster name, Aegis name or map",
                        state: search_path,
                        input_handler: DefaultHandler::<_, _, 60>::new(search_path, |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {}),
                        focus_id: MonsterSearch,
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    MonsterResultList::new(self.database.clone()),
                ),
            },
        }
    }
}

/// One button per search result, rebuilt when the search text changes.
struct MonsterResultList {
    database: Arc<MonsterDatabase>,
    shown_query: String,
    elements: Vec<ElementBox<ClientState>>,
}

impl MonsterResultList {
    fn new(database: Arc<MonsterDatabase>) -> Self {
        Self {
            database,
            shown_query: String::new(),
            elements: Vec::new(),
        }
    }

    fn rebuild(&mut self, query: &str) {
        self.shown_query = query.to_owned();
        self.elements.clear();

        for entry in self.database.search(query) {
            self.elements.push(self.result_button(entry));
        }
    }

    fn result_button(&self, entry: &MonsterEntry) -> ElementBox<ClientState> {
        use korangar_interface::prelude::*;

        let label = entry.label();
        let job_id = entry.job_id;
        let name = entry.name.clone();
        let first_map = entry.maps.first().cloned();

        ErasedElement::new(button! {
            text: label,
            event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                queue.queue(InputEvent::SelectMonster { job_id });

                if let Some(map_name) = first_map.clone() {
                    queue.queue(InputEvent::NavigateToDestination {
                        name: name.clone(),
                        map_name,
                        position: None,
                    });
                }
            },
        })
    }
}

impl Element<ClientState> for MonsterResultList {
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let path = client_state().monster_database();
            let query = state.get(&path).search().to_owned();

            if query != self.shown_query {
                self.rebuild(&query);
            }

            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}
