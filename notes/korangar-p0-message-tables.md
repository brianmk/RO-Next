# P0 — data-driven protocol result-code message tables

**Repo:** `/home/phantom/Games/ragnarok/CondaRO-linux/korangar`
**Branch:** `p0/message-tables` (worktree `/home/phantom/korangar-p0-messages`)
**Base:** `f454127e992af85e555ee759b9afb2a9ca649103` (`integration/all-fixes-20261121`, 2026-11-22)
**Scope:** `notes/client-hardcoded-systems-audit.md` §4.4 / §8 Phase 0 item 3 (result-code tables)
**Mode:** no Wine, no user characters, no game sessions, no network traffic to the game servers.
**Date:** 2026-11-22

---

## 1. Result

The two hardcoded protocol result-code → English message tables are replaced by one
data-driven table with a strict schema, a committed facts file, a generator with a
`--check` drift guard, an archive-overridable loader with a compiled fallback, a
`korangar --check-messages` CLI and tests. **No rendered user-facing text changes.**

| before | after |
| --- | --- |
| `korangar/src/lib.rs` party/shop/skill-fail/ammo/ban `match` arms (69 mappings) | `korangar/archive/data/client/messages.ron` rows, rendered through `korangar_messages::format_message` |
| `korangar-networking/src/packet_versions/version_20220406.rs:41–92` 25 login-message arms | same table (`MessageDomain::LoginFailed`, `LoginFailed2`), only protocol result codes remain in Rust |
| no schema, no drift guard, no coverage test | `tools/extract_message_facts.py --check`, `tools/gen_messages.py --check`, `korangar --check-messages`, schema/coverage/rendering tests |

**81 message rows** (4 party-member-removed, 11 party-invite-reply, 4 party-creation,
9 shop purchase, 17 skill-fail, 4 ammunition, 7 map-server ban, 3 login-server failure,
22 login-server refusal) — every previously handled code is present exactly once; the
data file is sorted and duplicate-free by construction and by validation.

**64 of 81 rows carry the official rAthena `MsgStringTable` id**, 57 of them with the
official English text committed next to the current text; 7 ids point at `MSI_*` macro
placeholders that the official client substitutes with its own built-in strings (so no
usable table text, documented per row). The remaining 17 rows have no official message
id (details in §7).

## 2. Data pipeline (facts → generator → data file)

```
tools/extract_message_facts.py  ──>  tools/message_facts.json
        (rAthena + client msgstringtable)         (committed facts)
                                                      │
tools/gen_messages.py  <──────────────────────────────┘
        └──> korangar/archive/data/client/messages.ron   (runtime data, embedded fallback)
```

* **Facts** (`tools/message_facts.json`, 40 KiB): one entry per result code with
  `code`, stable `key`, call-site `args`, current client `default` text, `display`,
  `msg_id`, official `official` text, `msg_id_source`, `client_source`, and the
  provenance of all sources.
  Sources:
  * rAthena `226bd83d39584bc91b6bf133f40bce95aa1f178c`
    (`src/map/clif.cpp`, `src/map/clif.hpp`, `src/map/mmo.hpp`,
    `src/login/loginclif.cpp`): protocol result codes and `MsgStringTable` ids/texts
    from the packet documentation comments and enums. Only plain facts are extracted
    (no rAthena code or comments).
  * CondaRO `CondaRO.grf` → `data\msgstringtable.txt` (EUC-KR, 3,575 entries): resolves
    every msg id to the shipped English text, so ids are validated and the official
    text is exact. The extracted table has one stray leading Node.js warning line that
    does not end in the classic `#` terminator; the extractor drops it exactly like
    `korangar/src/msgstringtable.rs` does at runtime (otherwise every id is shifted).
  * roBrowserLegacy `src/Engine/MapEngine/Skill.js` (`onSkillResult`): the official
    client's own `useskill_fail_cause` → `MsgStringTable` mapping (a client-side fact
    rAthena does not document). Same RE source the repo already uses for the official
    skill tables.
  * Current client default texts: copied verbatim from `korangar/src/lib.rs` and
    `.../version_20220406.rs` at `f454127e` (line references per row).
* **Generator** (`tools/gen_messages.py`): validates the facts (sorted/unique codes and
  keys, placeholder arguments, display flag, official text/msg-id consistency) and
  renders `messages.ron` deterministically. `--check` fails when the committed file
  differs; it needs no toolchain, no rAthena checkout and no game archives.
* **Data file** (`korangar/archive/data/client/messages.ron`, ~28 KiB): provenance
  line, `version: 1`, one block per domain with a committed `unknown` fallback and the
  sorted rows. Loaded at runtime like every other client data file, so a server can
  override it inside its archive.

Regenerate / verify:

```sh
./tools/extract_message_facts.py --rathena-dir <rathena@226bd83d> \
    --msgstringtable <client>/game/data/msgstringtable.txt   # facts (local drift guard)
./tools/gen_messages.py            # regenerate messages.ron
./tools/gen_messages.py --check    # CI drift guard
cargo run -p korangar -- --check-messages [PATH]   # schema validation, no window/network
```

## 3. Runtime loader / API

New crate `korangar-messages` (used by `korangar` and `korangar-networking`):

* `messages.ron` is embedded with `include_str!` as the compiled fallback; the client
  loads an archive override in `Client::init` (`load_protocol_messages`). A missing
  file uses the fallback (debug log); a malformed override is rejected with a warning
  and the fallback stays active, so result messages can never disappear.
* `format_message(domain, code, args: &[(&str, &str)]) -> String`: binary search over
  the domain rows. Call-site arguments (`character_name`, `skill_id`, `date`) are
  supplied by the handler; templates use `{argument}` placeholders and always have the
  numeric `{code}`.
* **Missing code:** renders the domain's committed `unknown` fallback (e.g.
  `Party invitation result: 12.`, `Cannot use skill 5: the skill cannot be used.`,
  `Disconnected: the server closed the connection.`) — the message is never dropped.
  `{code}` is always available, and a missing argument is rendered literally instead of
  truncating the message.
* Strict schema (`serde(deny_unknown_fields)`): unknown fields, unknown domains,
  wrong version, missing/duplicated/out-of-order domains, unsorted or duplicate rows,
  duplicate keys, empty texts, and undeclared/malformed placeholders are errors.
* Rendering is allocation-light: the table is a process-wide immutable `OnceLock`,
  lookups return `&'static` rows, only the final `String` is allocated.

### `display` flag: why P0 changes no text

Every row carries both texts:

| field | meaning |
| --- | --- |
| `default` | current client text (copied byte for byte from the old `match` arms) |
| `official` / `msg_id` | official `MsgStringTable` entry (id + text) for the later localization pass |
| `args` | call-site argument names, in the order official-client `%s`/`%d` placeholders are substituted |
| `display` | `Current` → render `default`; `Official` → render `official`, falling back to `default` when the row has no official text or the call site lacks its arguments |

**P0 commits all 81 rows as `display: Current`** (asserted by a test), so the rendered
text is exactly the pre-change text. The official ids/texts are already in the data and
the loader already implements `Official`, so a later localization pass is a data-only
change per row after review — no Rust changes.

## 4. Call sites replaced

| handler | event / packet | domain |
| --- | --- | --- |
| `korangar/src/lib.rs` | `PartyMemberRemoved` (`ZC_DELETE_MEMBER_FROM_GROUP`) | `PartyMemberRemoved` |
| `korangar/src/lib.rs` | `PartyInviteReply` (`ZC_PARTY_JOIN_REQ_ACK`) | `PartyInviteReply` |
| `korangar/src/lib.rs` | `PartyCreationResult` (`ZC_ACK_MAKE_GROUP`) | `PartyCreation` |
| `korangar/src/lib.rs` | `BuyingCompleted` (`ZC_PC_PURCHASE_RESULT`) | `BuyItem` |
| `korangar/src/lib.rs` | `SkillUseFailed` (`ZC_ACK_TOUSESKILL`) | `SkillUseFailed` |
| `korangar/src/lib.rs` | `AmmunitionAction` (`ZC_ACTION_FAILURE`) | `AmmunitionAction` |
| `korangar/src/lib.rs` | `MapServerBan` (`SC_NOTIFY_BAN`) | `MapServerBan` |
| `korangar-networking/.../version_20220406.rs` | login-server + character-server `LoginFailedPacket` (`SC_NOTIFY_BAN`) | `LoginFailed` |
| `korangar-networking/.../version_20220406.rs` | `LoginFailedPacket2` (`AC_REFUSE_LOGIN2`) | `LoginFailed2` |

Protocol opcodes/enums are untouched: the handlers still match the packet enums, but
only to obtain the numeric wire code (`buy_item_result_code`, `ammunition_action_code`,
`login_failed_reason_code`, `login_failed_reason2_code`). Tests assert those local code
maps equal the enums' serialized wire values, so they cannot silently drift. The
`CharacterServerConnectionFailed.message` event field became `String` (the table
renders at runtime; it used to be a `&'static str`).

## 5. Validation and tests

Data-side (new `korangar-messages` crate, 8 tests):

* compiled fallback validates; exactly **81 rows**, all 9 domains present;
* **behaviour snapshot:** every previously hardcoded code renders byte-for-byte the
  previous string (81-entry expected table built from the pre-change sources), with the
  same call-site arguments;
* P0 guard: every row is `display: Current`;
* documented missing-code fallback per domain;
* malformed documents are rejected: unknown field, wrong version, unknown/duplicate
  domain, unsorted rows, duplicate key, empty default/provenance, undeclared
  placeholder, official text without msg id, too many official placeholders, official
  display without text, unclosed/stray braces;
* malformed/missing arguments never panic or drop the message;
* `Official` display substitutes the official text and falls back when the call site
  lacks arguments.

Repo-level tests (`korangar/tests/message_tables.rs`, 4 tests):

* source-grep guard: the 73 removed English literals are gone from
  `korangar/src/lib.rs` and `version_20220406.rs` (a re-introduced hardcoded arm fails
  the build);
* handlers go through `format_message`/`MessageDomain` and the data file contains all
  domains, provenance, `display: Current` and official texts;
* `korangar::check_messages(None)` returns 81 and rejects a malformed file and an
  unreadable path.

CLI evidence:

```
$ korangar --check-messages
messages: 81 protocol result messages are valid          # exit 0
$ korangar --check-messages /tmp/bad-messages.ron
messages: invalid data: ...: schema error: Unexpected field named `broken` ...  # exit 1
$ ./tools/gen_messages.py --check
korangar/archive/data/client/messages.ron is up to date (81 rows in 9 domains)
$ ./tools/extract_message_facts.py --check --rathena-dir … --msgstringtable …
tools/message_facts.json is up to date (81 rows)
```

## 6. CI parity

* `.github/workflows/quality.yml`: new lightweight `generated-data` job runs
  `python3 tools/gen_messages.py --check` (+ `py_compile` of both tool scripts); the
  focused `tests` matrix jobs now include `-p korangar-messages`, so the schema,
  coverage, snapshot and fallback tests are part of CI.
* Local CI parity run (same commands as the workflows):
  `cargo fmt --all --check`, `cargo clippy --workspace -- -Dwarnings`,
  `cargo clippy --workspace --all-features -- -Dwarnings`,
  `cargo test --workspace --all-features`,
  `cargo build --release --features "unicode,debug"` — see §8 for the recorded results.
* No user-facing behaviour change: the rendering snapshot is the proof; `display:
  Current` for all rows is asserted.

## 7. Notes, limitations and decisions

* **Party member removed** (4 codes): rAthena/the client only send the result byte; the
  official client composes the text from the member name (no `MsgStringTable` id in the
  server documentation). The rows keep the call-site `{character_name}` and the current
  text; `msg_id: None` is documented per row.
* **Shop purchase** (9 codes): rAthena documents the English texts but not the ids; the
  ids were matched against the shipped `msgstringtable.txt` text once and are marked
  `msgstringtable text match` in the facts. Where the rAthena text and the English table
  wording differ (e.g. code 12 `The exchange was well done.` vs `Exchange successfuly
  completed.`), both texts are committed; P0 still renders the current text.
* **Map-server ban code 8**: rAthena documents `MsgStringTable[441]`, but in the 2019
  CondaRO English table 441 is unrelated (`Release Falcon`); the actual text
  (`Server still recognizes your last log-in…`) is id **440**. The facts keep the
  rAthena id as `rathena_msg_id` and commit the resolved id 440 with the reason.
* **Skill-fail ids** come from the official client's own mapping (roBrowserLegacy
  `onSkillResult`), not from rAthena: the table is a client-side message and rAthena
  only documents the `useskill_fail_cause` enum. Causes the official client leaves
  without a message (0, 11, 12, 14, 22, 23, 26) have `msg_id: None`; their current P0
  text is preserved.
* **`AC_REFUSE_LOGIN2` codes 9–14** map to `MSI_REFUSE_*` macro entries: the id is
  committed (703–708) but there is no usable official text in `msgstringtable.txt` (the
  client substitutes its own built-in string, which is the current text). Codes not in
  the table (5 `Game outdated` resolves to 310; 8, 99, 105 have no entry) are marked
  accordingly.
* **Party invite codes 10/11** use rAthena `MSG_ID_BD3`/`MSG_ID_C9A`, which are
  `msg_conf` ids for a server-sent chat line, not `msgstringtable` client messages:
  `msg_id: None`, current text preserved.
* The message table intentionally does **not** touch `korangar/src/interface/**`,
  `state/localization/**` or the en/de RON files (another lane owns them). Official
  texts are committed in the message data, not as localization keys.
* Out of scope (left as-is, audited later): `StatUpFailed`, `RefuseEnter`, `/who`
  count, character-creation/deletion messages and the other client feedback strings
  (audit §4.4 lists those separately from the result-code tables), and `ZC_MSG`
  system messages that already use `msgstringtable`.

## 8. Evidence

```
$ cargo fmt --all --check
(clean)                                                                    exit 0

$ cargo clippy --workspace -- -Dwarnings
    Finished `dev` profile ...                                             exit 0
$ cargo clippy --workspace --all-features -- -Dwarnings
    Finished `dev` profile ...                                             exit 0

$ cargo test --workspace --all-features --no-fail-fast
42 test binaries: 41 `test result: ok`, 1 failing test — pre-existing and out
of this lane's scope:
  hardcoded_style_literals_do_not_exceed_the_token_budget (ui_consistency):
  "the interface has 29 hardcoded color literals, but the token budget is 28"
  -> audit §1/§4.5 already records the design-token ratchet as RED at HEAD
     (`./scripts/ui-token-budget.sh`: colors=29/28, FontSize=16/12, exit 1);
     this branch changes no file under `korangar/src/interface/**`.
The new tests all pass in that run:
  korangar-messages                              8 passed
  korangar/tests/message_tables.rs               4 passed
  korangar lib (incl. protocol-code wire tests) 443 passed, 11 ignored
  korangar-networking                           61 passed

$ cargo test --workspace --all-features --no-fail-fast -- \
      --skip hardcoded_style_literals_do_not_exceed_the_token_budget
42 test binaries: all `test result: ok` (0 failed)                         exit 0

$ cargo build --release --features "unicode,debug"
    Finished `release` profile ...                                         exit 0

$ ./tools/gen_messages.py --check
korangar/archive/data/client/messages.ron is up to date (81 rows in 9 domains)
$ ./tools/extract_message_facts.py --check --rathena-dir ... --msgstringtable ...
tools/message_facts.json is up to date (81 rows)
$ korangar --check-messages
messages: 81 protocol result messages are valid                            exit 0
$ korangar --check-messages /tmp/bad-messages.ron
messages: invalid data: ...: schema error: Unexpected field named `broken` exit 1
```

## 9. Commit

* Branch: `p0/message-tables`, worktree `/home/phantom/korangar-p0-messages`
* Base commit: `f454127e992af85e555ee759b9afb2a9ca649103`; the lane commit is the
  branch tip (`git -C /home/phantom/korangar-p0-messages log -1`), the SHA is
  reported in the lane hand-off.
* Files: `korangar-messages/` (new crate), `korangar/archive/data/client/messages.ron`,
  `notes/korangar-p0-message-tables.md`,
  `korangar/src/lib.rs`, `korangar/src/main.rs`, `korangar/tests/message_tables.rs`,
  `korangar-networking/src/event.rs`,
  `korangar-networking/src/packet_versions/version_20220406.rs`,
  `tools/extract_message_facts.py`, `tools/gen_messages.py`,
  `tools/message_facts.json`, `Cargo.toml`, `Cargo.lock`,
  `korangar/Cargo.toml`, `korangar-networking/Cargo.toml`,
  `.github/workflows/quality.yml`.
