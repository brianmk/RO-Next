//! `NetworkEvent` handlers for the ui family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::DisplayImage`.
    pub(crate) fn handle_display_image(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::DisplayImage { image_name, location } = event else {
            unreachable!("handle_display_image called with a different event variant");
        };

        if matches!(location, ragnarok_packets::ImageLocation::ClearAll) {
            self.client_state.follow_mut(client_state().dialog_window()).clear_image();
        } else if let Some(texture) = load_cutin_texture(&self.game_file_loader, &self.texture_loader, &image_name) {
            self.client_state.follow_mut(client_state().dialog_window()).set_image(texture);
        }

        self.interface.invalidate_layouts();
        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::OpenBook`.
    pub(crate) fn handle_open_book(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenBook { book_id, page } = event else {
            unreachable!("handle_open_book called with a different event variant");
        };

        let pages = load_book_pages(&self.game_file_loader, book_id);
        self.interface.close_window_with_class(WindowClass::Book);
        self.interface.open_window(BookWindow::new(book_id, page, pages));
    }

    /// `NetworkEvent::OpenEditDialog`.
    pub(crate) fn handle_open_edit_dialog(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenEditDialog { npc_id, string_input } = event else {
            unreachable!("handle_open_edit_dialog called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().dialog_window())
            .start_input(npc_id, string_input);

        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::OpenDialog`.
    pub(crate) fn handle_open_dialog(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenDialog { text, npc_id } = event else {
            unreachable!("handle_open_dialog called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().dialog_window())
            .initialize(npc_id)
            .add_text(text);

        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::AddNextButton`.
    pub(crate) fn handle_add_next_button(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AddNextButton { npc_id } = event else {
            unreachable!("handle_add_next_button called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().dialog_window())
            // An NPCs could start the dialog with this packet so we want to make sure it's initialized.
            .initialize(npc_id)
            .add_next_button();

        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::AddCloseButton`.
    pub(crate) fn handle_add_close_button(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AddCloseButton { npc_id } = event else {
            unreachable!("handle_add_close_button called with a different event variant");
        };

        // Some NPCs send the `CloseButtonPacket` after the dialog
        // has been closed. We want to filter these out because otherwise we get a
        // close button at the start of the next dialog.
        if self.interface.is_window_with_class_open(WindowClass::Dialog) {
            self.client_state
                .follow_mut(client_state().dialog_window())
                // Technically this call is redundant since the window is already open
                // but we keep it for consistency.
                .initialize(npc_id)
                .add_close_button();
        }
    }

    /// `NetworkEvent::AddChoiceButtons`.
    pub(crate) fn handle_add_choice_buttons(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AddChoiceButtons { choices, npc_id } = event else {
            unreachable!("handle_add_choice_buttons called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().dialog_window())
            // Some NPCs start the dialog with this packet so we need to make sure it's initialized.
            .initialize(npc_id)
            .add_choice_buttons(choices);

        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::WarpPointList`.
    pub(crate) fn handle_warp_point_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::WarpPointList { skill_id, maps } = event else {
            unreachable!("handle_warp_point_list called with a different event variant");
        };

        // Warp Portal (`AL_WARP`) finished casting: the server asks for
        // the destination map and creates the portal units only after
        // the client answered. Show the destination list in the dialog
        // window and send the choice back in `SelectWarpPoint`.
        if maps.is_empty() {
            return;
        }

        self.client_state
            .follow_mut(client_state().dialog_window())
            .start_warp_point_list(skill_id, maps);
        self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
    }

    /// `NetworkEvent::AddQuestEffect`.
    pub(crate) fn handle_add_quest_effect(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AddQuestEffect { quest_effect } = event else {
            unreachable!("handle_add_quest_effect called with a different event variant");
        };

        if let Some(map) = &self.map {
            self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
        }
    }

    /// `NetworkEvent::RemoveQuestEffect`.
    pub(crate) fn handle_remove_quest_effect(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RemoveQuestEffect { entity_id } = event else {
            unreachable!("handle_remove_quest_effect called with a different event variant");
        };

        self.particle_holder.remove_quest_icon(entity_id)
    }

    /// `NetworkEvent::OpenUserInterface`.
    pub(crate) fn handle_open_user_interface(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenUserInterface { ui_type, data: _ } = event else {
            unreachable!("handle_open_user_interface called with a different event variant");
        };

        // `OUT_UI_BANK` (0, `clif.hpp:1416`) is the NPC `openbank`
        // trigger. The window itself is opened by the banking request.
        if ui_type == 0 {
            self.open_bank_window();
        }
    }

    /// `NetworkEvent::MailList`.
    pub(crate) fn handle_mail_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailList { mails } = event else {
            unreachable!("handle_mail_list called with a different event variant");
        };

        self.client_state.follow_mut(client_state().mail()).apply_list(mails);
    }

    /// `NetworkEvent::MailRead`.
    pub(crate) fn handle_mail_read(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailRead {
            mail_id,
            open_type: _,
            body,
            zeny,
            items,
        } = event
        else {
            unreachable!("handle_mail_read called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().mail())
            .apply_read(mail_id, body, zeny, items);
    }

    /// `NetworkEvent::MailAttachmentClaimed`.
    pub(crate) fn handle_mail_attachment_claimed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailAttachmentClaimed { mail_id, zeny, result } = event else {
            unreachable!("handle_mail_attachment_claimed called with a different event variant");
        };

        let mailbox = self.client_state.follow_mut(client_state().mail());
        mailbox.apply_claimed(mail_id, zeny, result);
        let status = mailbox.status.clone();
        self.push_chat_information(status);
    }

    /// `NetworkEvent::MailDeleted`.
    pub(crate) fn handle_mail_deleted(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailDeleted { mail_id } = event else {
            unreachable!("handle_mail_deleted called with a different event variant");
        };

        let mailbox = self.client_state.follow_mut(client_state().mail());
        mailbox.apply_deleted(mail_id);
        let status = mailbox.status.clone();
        self.push_chat_information(status);
    }

    /// `NetworkEvent::MailUnreadStatus`.
    pub(crate) fn handle_mail_unread_status(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailUnreadStatus { unread } = event else {
            unreachable!("handle_mail_unread_status called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().mail())
            .set_unread_mail_available(unread);
    }

    /// `NetworkEvent::MailboxWindow`.
    pub(crate) fn handle_mailbox_window(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MailboxWindow { open } = event else {
            unreachable!("handle_mailbox_window called with a different event variant");
        };

        match open {
            true => {
                if !self.interface.is_window_with_class_open(WindowClass::Mail) {
                    self.interface.open_window(MailboxWindow::new(client_state().mail()));
                }
                // `ZC_MAIL_WINDOWS` only opens the window; the inbox
                // has to be requested explicitly.
                let _ = self.networking_system.open_mailbox();
            }
            false => self.interface.close_window_with_class(WindowClass::Mail),
        }
    }

    /// `NetworkEvent::PetProperty`.
    pub(crate) fn handle_pet_property(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PetProperty { property } = event else {
            unreachable!("handle_pet_property called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().companion())
            .apply_pet_property(property);
    }

    /// `NetworkEvent::HomunculusProperty`.
    pub(crate) fn handle_homunculus_property(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::HomunculusProperty { property } = event else {
            unreachable!("handle_homunculus_property called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().companion())
            .apply_homunculus_property(property);
    }

    /// `NetworkEvent::HomunculusParameterChanged`.
    pub(crate) fn handle_homunculus_parameter_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::HomunculusParameterChanged { parameter, value } = event else {
            unreachable!("handle_homunculus_parameter_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().companion())
            .apply_homunculus_parameter_change(parameter, value);
    }

    /// `NetworkEvent::MercenaryInfo`.
    pub(crate) fn handle_mercenary_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MercenaryInfo { info } = event else {
            unreachable!("handle_mercenary_info called with a different event variant");
        };

        self.client_state.follow_mut(client_state().companion()).apply_mercenary_info(info);
    }

    /// `NetworkEvent::MercenaryParameterChanged`.
    pub(crate) fn handle_mercenary_parameter_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MercenaryParameterChanged { parameter, value } = event else {
            unreachable!("handle_mercenary_parameter_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().companion())
            .apply_mercenary_parameter_change(parameter, value);
    }

    /// `NetworkEvent::QuestList`.
    pub(crate) fn handle_quest_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::QuestList { quests } = event else {
            unreachable!("handle_quest_list called with a different event variant");
        };

        self.client_state.follow_mut(client_state().quest_log()).set_list(&quests);
    }

    /// `NetworkEvent::QuestActive`.
    pub(crate) fn handle_quest_active(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::QuestActive { quest_id, active } = event else {
            unreachable!("handle_quest_active called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().quest_log())
            .set_active(quest_id, active);
    }

    /// `NetworkEvent::QuestRemoved`.
    pub(crate) fn handle_quest_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::QuestRemoved { quest_id } = event else {
            unreachable!("handle_quest_removed called with a different event variant");
        };

        self.client_state.follow_mut(client_state().quest_log()).remove(quest_id);
    }

    /// `NetworkEvent::AchievementList`.
    pub(crate) fn handle_achievement_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AchievementList {
            achievements,
            total_score,
            level,
            experience,
            experience_to_next_level,
        } = event
        else {
            unreachable!("handle_achievement_list called with a different event variant");
        };

        self.client_state.follow_mut(client_state().achievement()).set_list(
            &achievements,
            total_score,
            level,
            experience,
            experience_to_next_level,
        );
    }

    /// `NetworkEvent::AchievementUpdate`.
    pub(crate) fn handle_achievement_update(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AchievementUpdate {
            achievement,
            total_score,
            level,
            experience,
            experience_to_next_level,
        } = event
        else {
            unreachable!("handle_achievement_update called with a different event variant");
        };

        self.client_state.follow_mut(client_state().achievement()).update(
            &achievement,
            total_score,
            level,
            experience,
            experience_to_next_level,
        );
    }

    /// `NetworkEvent::AchievementRewardResult`.
    pub(crate) fn handle_achievement_reward_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AchievementRewardResult { achievement_id, success } = event else {
            unreachable!("handle_achievement_reward_result called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().achievement())
            .set_reward_result(achievement_id, success);
        let message = match success {
            true => format!("Achievement {achievement_id}: reward granted."),
            false => format!("Achievement {achievement_id}: reward not available."),
        };
        self.push_chat_information(message);
    }

    /// `NetworkEvent::MonsterInfo`.
    pub(crate) fn handle_monster_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MonsterInfo { information } = event else {
            unreachable!("handle_monster_info called with a different event variant");
        };

        let sense = crate::state::monster_database::SenseInfo {
            job_id: information.job_id,
            level: information.level,
            size: information.size,
            health_points: information.health_points,
            defense: information.defense,
            race: information.race,
            magic_defense: information.magic_defense,
            element: information.element,
            resistances: [
                information.water,
                information.earth,
                information.fire,
                information.wind,
                information.poison,
                information.holy,
                information.shadow,
                information.ghost,
                information.undead,
            ],
        };

        self.client_state.follow_mut(client_state().monster_database()).set_sense(sense);
        self.push_chat_information(format!(
            "Sense: monster #{} level {} HP {}",
            information.job_id.0, information.level, information.health_points
        ));

        if !self.interface.is_window_with_class_open(WindowClass::MonsterDatabase) {
            self.interface
                .open_window(MonsterDatabaseWindow::new(self.library.monster_database()));
        }
    }
}
