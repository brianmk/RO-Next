//! Parser for the chat/NPC-dialog markup of the official client.
//!
//! The 2019 server sends item, navigation and URL links as plain text markup
//! inside chat messages and NPC dialog lines:
//!
//! * `<ITEM>name<INFO>item id</INFO></ITEM>`
//! * `<ITEMLINK>name<INFO>item id,identify,refine,card0,...</INFO></ITEMLINK>`
//! * `<NAVI>map name,x,y</NAVI>`
//! * `<URL>https://example.invalid/</URL>`
//!
//! This module only *parses* the text into runs; the interface layer turns the
//! runs into clickable text areas. Text without any of these tags yields a
//! single run, so the normal rendering path stays byte-for-byte identical.
//!
//! The parser is deliberately forgiving: a tag that is malformed (unterminated,
//! unknown variant, bad item id) is kept as literal text instead of being
//! dropped, so a server typo can never make a message disappear.

use ragnarok_packets::ItemId;

/// One interactive target of a chat/dialog text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatMarkupLink {
    /// `<ITEM>` / `<ITEMLINK>`: show the item info window.
    Item {
        item_id: ItemId,
        /// Identification flag from `<INFO>` (second field of `<ITEMLINK>`).
        identified: bool,
    },
    /// `<NAVI>`: navigate to / mark a map position.
    Navi { map_name: String, x: u16, y: u16 },
    /// `<URL>`: open the URL in the system browser.
    Url { url: String },
    /// Not produced by the parser: a whisper chat line is clickable as a whole
    /// and opens the whisper conversation with the sender.
    Whisper { partner: String },
}

/// A run of text and the optional link attached to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMarkupRun {
    pub text: String,
    pub link: Option<ChatMarkupLink>,
}

impl ChatMarkupRun {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            link: None,
        }
    }
}

/// The tag names this parser understands (without the angle brackets).
const TAG_ITEM: &str = "ITEM";
const TAG_ITEMLINK: &str = "ITEMLINK";
const TAG_NAVI: &str = "NAVI";
const TAG_URL: &str = "URL";

/// Returns whether the text contains any markup the client can render as a
/// link. This is used to keep the plain rendering path untouched.
pub fn has_markup(text: &str) -> bool {
    parse_chat_markup(text).iter().any(|run| run.link.is_some())
}

/// Parse a chat/dialog text into plain and interactive runs.
///
/// Concatenating the [`ChatMarkupRun::text`] of the result always reproduces
/// the display text of the input (tags removed, their display text kept).
pub fn parse_chat_markup(text: &str) -> Vec<ChatMarkupRun> {
    let mut runs = Vec::new();
    let mut plain = String::new();
    let mut rest = text;

    while let Some(open_index) = find_next_tag(rest) {
        let before = &rest[..open_index];
        plain.push_str(before);

        let tag_text = &rest[open_index..];
        let Some((run, consumed)) = parse_tag_at(tag_text) else {
            // Not a real tag (malformed): keep the '<' as literal text and keep
            // scanning after it, so the rest of the message is still parsed.
            plain.push('<');
            rest = &tag_text[1..];
            continue;
        };

        if !plain.is_empty() {
            runs.push(ChatMarkupRun::plain(std::mem::take(&mut plain)));
        }

        runs.push(run);
        rest = &tag_text[consumed..];
    }

    plain.push_str(rest);

    if !plain.is_empty() {
        runs.push(ChatMarkupRun::plain(plain));
    }

    // A completely empty message produces no runs; callers treat that as empty.
    runs
}

/// Index of the next '<' that starts one of the known tags.
fn find_next_tag(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = 0;

    while let Some(offset) = text[index..].find('<') {
        let candidate = index + offset;

        if tag_name_at(&text[candidate..]).is_some() {
            return Some(candidate);
        }

        index = candidate + 1;

        if index >= bytes.len() {
            break;
        }
    }

    None
}

/// The uppercase tag name starting at the `<` of `text`, if it is one of the
/// known tags.
fn tag_name_at(text: &str) -> Option<&'static str> {
    let text = text.strip_prefix('<')?;
    let name_end = text.find(['>', '/'])?;
    let name = &text[..name_end];

    // Tag names are compared case-insensitively, like the official client.
    for candidate in [TAG_ITEMLINK, TAG_ITEM, TAG_NAVI, TAG_URL] {
        if name.eq_ignore_ascii_case(candidate) {
            return Some(candidate);
        }
    }

    None
}

/// Try to parse a recognised tag at the start of `text`. Returns the run and
/// the number of bytes the tag occupies in the input.
fn parse_tag_at(text: &str) -> Option<(ChatMarkupRun, usize)> {
    let tag_name = tag_name_at(text)?;

    match tag_name {
        TAG_ITEM | TAG_ITEMLINK => parse_item_tag(text, tag_name),
        TAG_NAVI => parse_simple_tag(text, "NAVI").and_then(|(content, consumed)| {
            let link = parse_navi(&content)?;
            let display = match &link {
                ChatMarkupLink::Navi { map_name, x, y } => format!("{map_name} ({x}, {y})"),
                _ => content,
            };

            Some((
                ChatMarkupRun {
                    text: display,
                    link: Some(link),
                },
                consumed,
            ))
        }),
        TAG_URL => parse_simple_tag(text, "URL").and_then(|(content, consumed)| {
            let url = content.trim();
            let link = match url.starts_with("http://") || url.starts_with("https://") {
                true => ChatMarkupLink::Url { url: url.to_owned() },
                false => return None,
            };

            Some((
                ChatMarkupRun {
                    text: content,
                    link: Some(link),
                },
                consumed,
            ))
        }),
        _ => None,
    }
}

/// Parse `<TAG>content</TAG>`. Returns the content and the consumed length.
fn parse_simple_tag(text: &str, closing_name: &str) -> Option<(String, usize)> {
    let open_end = text.find('>')?;
    let content_start = open_end + 1;
    let closing = format!("</{closing_name}>");

    // The closing tag may be spelled with a different case.
    let lower = text.to_ascii_lowercase();
    let closing_start = lower[content_start..].find(&closing.to_ascii_lowercase())? + content_start;
    let content = text[content_start..closing_start].to_owned();
    let consumed = closing_start + closing.len();

    Some((content, consumed))
}

/// Parse the `<ITEM>` family with its embedded `<INFO>` block.
fn parse_item_tag(text: &str, tag_name: &str) -> Option<(ChatMarkupRun, usize)> {
    let open_end = text.find('>')?;
    let body_start = open_end + 1;
    let lower = text.to_ascii_lowercase();

    let info_open = lower[body_start..].find("<info>")? + body_start;
    let display = text[body_start..info_open].to_owned();

    let info_start = info_open + "<info>".len();
    let info_close = lower[info_start..].find("</info>")? + info_start;
    let info = &text[info_start..info_close];

    let closing = format!("</{}>", tag_name.to_ascii_lowercase());
    let closing_start = lower[info_close..].find(&closing)? + info_close;
    let consumed = closing_start + closing.len();

    let (item_id, identified) = parse_item_info(info)?;

    let display = match display.trim().is_empty() {
        true => format!("Item #{}", item_id.0),
        false => display,
    };

    Some((
        ChatMarkupRun {
            text: display,
            link: Some(ChatMarkupLink::Item { item_id, identified }),
        },
        consumed,
    ))
}

/// Parse the `<INFO>` payload of an item tag.
///
/// `ITEM` uses just the item id, `ITEMLINK` uses
/// `id,identify,refine,card0,...`. Both are accepted for both tags; the second
/// numeric field is the identification flag.
fn parse_item_info(info: &str) -> Option<(ItemId, bool)> {
    let mut fields = info
        .split(|character: char| character == ',' || character == ';' || character == ':' || character.is_whitespace())
        .filter(|field| !field.is_empty());

    let item_id = fields.next()?.parse::<u32>().ok()?;
    let identified = match fields.next() {
        Some(value) => value.parse::<u32>().map(|value| value != 0).unwrap_or(true),
        // `<ITEM>` has no identification field and is always identified.
        None => true,
    };

    Some((ItemId(item_id), identified))
}

/// Parse `<NAVI>` content: a map name followed by an x/y tile position.
fn parse_navi(content: &str) -> Option<ChatMarkupLink> {
    let mut fields = content
        .split(|character: char| character == ',' || character == ';' || character.is_whitespace())
        .filter(|field| !field.is_empty());

    let map_name = fields.next()?.trim().to_owned();
    let x = fields.next()?.parse::<u16>().ok()?;
    let y = fields.next()?.parse::<u16>().ok()?;

    Some(ChatMarkupLink::Navi { map_name, x, y })
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::ItemId;

    use super::{ChatMarkupLink, has_markup, parse_chat_markup};

    #[test]
    fn plain_text_is_returned_unchanged() {
        let text = "party: hello ^FF0000world^000000 <not a tag> 100%";
        let runs = parse_chat_markup(text);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, text);
        assert!(runs[0].link.is_none());
        assert!(!has_markup(text));
    }

    #[test]
    fn item_links_keep_the_display_name_and_parse_the_id() {
        let runs = parse_chat_markup("take this <ITEM>Red Potion<INFO>501</INFO></ITEM> now");

        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "take this ");
        assert_eq!(runs[1].text, "Red Potion");
        assert_eq!(
            runs[1].link,
            Some(ChatMarkupLink::Item {
                item_id: ItemId(501),
                identified: true,
            })
        );
        assert_eq!(runs[2].text, " now");
    }

    #[test]
    fn itemlink_parses_the_identification_flag() {
        let runs = parse_chat_markup("<ITEMLINK>Unknown Sword<INFO>1101,0,5,0,0,0,0</INFO></ITEMLINK>");

        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].link,
            Some(ChatMarkupLink::Item {
                item_id: ItemId(1101),
                identified: false,
            })
        );
    }

    #[test]
    fn navi_links_parse_map_and_position() {
        let runs = parse_chat_markup("<NAVI>prontera,150,160</NAVI>");

        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].link,
            Some(ChatMarkupLink::Navi {
                map_name: "prontera".to_owned(),
                x: 150,
                y: 160,
            })
        );

        // The space separated variant is accepted as well.
        let runs = parse_chat_markup("<NAVI>payon 10 20</NAVI>");
        assert_eq!(
            runs[0].link,
            Some(ChatMarkupLink::Navi {
                map_name: "payon".to_owned(),
                x: 10,
                y: 20,
            })
        );
    }

    #[test]
    fn url_links_are_recognized_and_restricted_to_http() {
        let runs = parse_chat_markup("see <URL>https://example.invalid/guide</URL>");
        assert_eq!(
            runs[1].link,
            Some(ChatMarkupLink::Url {
                url: "https://example.invalid/guide".to_owned(),
            })
        );

        // Non-http schemes are not rendered as a clickable URL.
        let runs = parse_chat_markup("<URL>file:///etc/passwd</URL>");
        assert!(runs.iter().all(|run| run.link.is_none()));
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "<URL>file:///etc/passwd</URL>"
        );
    }

    #[test]
    fn malformed_tags_are_kept_as_literal_text() {
        for text in [
            "<ITEM>no info block</ITEM>",
            "<ITEM>name<INFO>not a number</INFO></ITEM>",
            "<NAVI>prontera</NAVI>",
            "<UNKNOWN>value</UNKNOWN>",
            "<<ITEM>>",
        ] {
            let runs = parse_chat_markup(text);
            assert_eq!(
                runs.iter().map(|run| run.text.as_str()).collect::<String>(),
                text,
                "malformed markup must not lose text"
            );
            assert!(runs.iter().all(|run| run.link.is_none()), "unexpected link for {text:?}");
        }
    }

    #[test]
    fn tags_are_case_insensitive() {
        let runs = parse_chat_markup("<item>Potion<info>501</info></item>");
        assert_eq!(
            runs[0].link,
            Some(ChatMarkupLink::Item {
                item_id: ItemId(501),
                identified: true,
            })
        );
    }

    #[test]
    fn multiple_links_in_one_line_are_all_parsed() {
        let runs = parse_chat_markup("<ITEM>Potion<INFO>501</INFO></ITEM> and <URL>https://example.invalid</URL>");
        let links: Vec<_> = runs.iter().filter_map(|run| run.link.as_ref()).collect();

        assert_eq!(links.len(), 2);
        assert!(matches!(links[0], ChatMarkupLink::Item { .. }));
        assert!(matches!(links[1], ChatMarkupLink::Url { .. }));
    }

    #[test]
    fn has_markup_matches_the_parser() {
        assert!(has_markup("<URL>https://example.invalid</URL>"));
        assert!(!has_markup("plain < text"));
        assert!(!has_markup("<ITEM>broken</ITEM>"));
    }
}
