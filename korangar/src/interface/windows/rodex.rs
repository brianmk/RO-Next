//! Rodex mailbox window.
//!
//! The 2019 server sends the whole inbox as `ZC_ACK_MAIL_LIST3` (0x0AC2) and
//! the body of a mail as `ZC_ACK_READ_RODEX` (0x09EB), see
//! [`crate::state::mail`]. Every mail is a collapsible row: the header shows
//! the read state, sender, title, attachment flags and expiry, and expanding it
//! shows the body plus the Read/Claim/Delete buttons.
//!
//! The row list is built by a custom element because its size is dynamic and
//! every row needs text + a horizontal button strip.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::mail::{Mail, Mailbox, MailboxPathExt};
use crate::state::theme::InterfaceThemeType;

/// Custom element that renders one collapsible row per inbox mail.
struct MailList<A> {
    mailbox_path: A,
    elements: Vec<korangar_interface::element::ElementBox<ClientState>>,
}

impl<A> MailList<A> {
    fn new(mailbox_path: A) -> Self {
        Self {
            mailbox_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for MailList<A>
where
    A: Path<ClientState, Mailbox> + 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let mail_count = state.get(&self.mailbox_path).mails.len();

            self.elements.truncate(mail_count);

            for index in self.elements.len()..mail_count {
                // The paths are index based: the row data is read from the
                // current state when the layout is created and when a button
                // is clicked, so a list refresh never leaves a stale mail id.
                let mail_path = self.mailbox_path.mails().index(index).manually_asserted();
                let header_path = mail_path;
                let details_path = mail_path;
                let read_path = mail_path;
                let zeny_path = mail_path;
                let items_path = mail_path;
                let delete_path = mail_path;

                let header_text = ComputedSelector::new_default(move |state: &ClientState| {
                    let mail: &Mail = header_path.follow_safe(state);
                    mail.list_line()
                });

                let details_text = ComputedSelector::new_default(move |state: &ClientState| {
                    let mail: &Mail = details_path.follow_safe(state);
                    mail.details_text()
                });

                let zeny_disabled = ComputedSelector::new_default(move |state: &ClientState| {
                    let mail: &Mail = zeny_path.follow_safe(state);
                    !mail.has_zeny()
                });

                let items_disabled = ComputedSelector::new_default(move |state: &ClientState| {
                    let mail: &Mail = items_path.follow_safe(state);
                    !mail.has_items()
                });

                // rAthena refuses to delete a mail that still has attachments
                // (clif_parse_Mail_delete), so the button mirrors that rule.
                let delete_disabled = ComputedSelector::new_default(move |state: &ClientState| {
                    let mail: &Mail = delete_path.follow_safe(state);
                    mail.has_attachments()
                });

                self.elements.push(ErasedElement::new(collapsible! {
                    text: header_text,
                    overflow_behavior: OverflowBehavior::Shrink,
                    children: (
                        text! {
                            text: details_text,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        split! {
                            gaps: 4.0,
                            children: (
                                button! {
                                    text: "Read",
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let mail: &Mail = state.get(&read_path);

                                        queue.queue(InputEvent::ReadMail {
                                            mail_id: mail.mail_id,
                                            open_type: mail.open_type,
                                        });
                                    },
                                },
                                button! {
                                    text: "Claim Zeny",
                                    disabled: zeny_disabled,
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let mail: &Mail = state.get(&zeny_path);

                                        queue.queue(InputEvent::ClaimMailZeny {
                                            mail_id: mail.mail_id,
                                            open_type: mail.open_type,
                                        });
                                    },
                                },
                                button! {
                                    text: "Claim Items",
                                    disabled: items_disabled,
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let mail: &Mail = state.get(&items_path);

                                        queue.queue(InputEvent::ClaimMailItems {
                                            mail_id: mail.mail_id,
                                            open_type: mail.open_type,
                                        });
                                    },
                                },
                                button! {
                                    text: "Delete",
                                    disabled: delete_disabled,
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let mail: &Mail = state.get(&delete_path);

                                        queue.queue(InputEvent::DeleteMail {
                                            mail_id: mail.mail_id,
                                            open_type: mail.open_type,
                                        });
                                    },
                                },
                            ),
                        },
                    ),
                }));
            }

            self.elements
                .iter_mut()
                .enumerate()
                .for_each(|(index, element)| element.create_layout_info(state, store.child_store(index as u64), resolver));
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

/// The Rodex mailbox window.
pub struct MailboxWindow<A> {
    mailbox_path: A,
}

impl<A> MailboxWindow<A> {
    pub fn new(mailbox_path: A) -> Self {
        Self { mailbox_path }
    }
}

impl<A> CustomWindow<ClientState> for MailboxWindow<A>
where
    A: Path<ClientState, Mailbox> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Mail)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let mailbox_path = self.mailbox_path;
        let status_path = mailbox_path;

        let status_text = ComputedSelector::new_default(move |state: &ClientState| {
            let mailbox: &Mailbox = status_path.follow_safe(state);
            mailbox.status_line()
        });

        window! {
            title: "Mail",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            resizable: true,
            closable: true,
            minimum_width: 460.0,
            maximum_width: 720.0,
            elements: (
                text! {
                    text: status_text,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    gaps: 4.0,
                    children: (
                        button! {
                            text: "Refresh",
                            event: InputEvent::RefreshMailbox,
                        },
                    ),
                },
                scroll_view! {
                    scroll_bar: true,
                    children: MailList::new(mailbox_path),
                },
            ),
        }
    }
}
