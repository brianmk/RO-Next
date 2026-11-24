# Requirements
In order to run the client, you will need to get the `data.grf` and `rdata.grf` from the official kRO client. Place them both inside the `korangar` directory (`korangar/korangar`) and start the client using the commands shown in the (Installation)[Installation.md] section, swapping out `build` for `run`. For example:

```fish
cargo run --release --features debug
```

# Game servers

### 🔓 Remote server
By default, Korangar will try to connect to a remote rAthena instance that we host for development purposes. Since it is intended to be used for testing, we have modified it to make everyone's life easier. These changes include:
- No packet obfuscation, meaning the network traffic is unencrypted
- Anyone can use @-commands from the instant they are placed into the tutorial
- Deleting characters does not require any wait time or confirmation

### 🔒 Local server
You can also tell Korangar to connect to a server on your local machine by editing `sclientinfo.xml` in `korangar/archive/data/`. Just duplicate one of the existing entries and replace the server IP and name. If you are interested in setting up a server locally I suggest reading the [install instructions](https://github.com/rathena/rathena#2-installation) on the rAthena GitHub page. If you are comfortable reading Nix coder you can also check out [korangar-rathena](https://github.com/vE5li/korangar-rathena), which is the repository containing the configuration of the development server (including all patches and settings).

# Logging in
If everything starts correctly, you should see a window prompting you for a username and a password. You can create a new user by entering your desired username with the suffix `_m` or `_f` (for `male` and `female` respectively) and your desired password.

> [!WARNING]
> Please don't use any of your usual credentials as we will be able to see them in the database. They will also be saved in _**plain text**_ in the client folder if you tick the boxes during login.

### 🫀 Creating a character
After logging in, you will be able to create a new character. If the character creation fails, it might be because you are using invalid characters in the name (e.g. `_`).

# Logging and crash diagnostics

Release builds are quiet by default. Pass `-v`, `-vv` or `-vvv` to the client (or to the
launcher script, which forwards all arguments) to get progressively more log output on stderr:

```fish
# info: startup phases, settings/theme loading, window openings
./scripts/run-korangar.sh -v

# debug: plus window cache/theme details, interface scaling, layout details
./scripts/run-korangar.sh -vv

# trace: plus every window creation/layout, input event and classic skin texture load
./scripts/run-korangar.sh -vvv 2> client.log
```

The verbosity can also be set with the `KORANGAR_LOG_LEVEL` environment variable (`0`-`3`), which
is useful when the client is started from a desktop launcher that cannot pass arguments.

Every line has a timestamp, level and module prefix, for example:

```text
[12:34:56.789 INFO  korangar] interface: opening interface settings window (in_game_theme="classic", scaling=1)
[12:34:56.790 TRACE korangar_interface] interface: create_layout_info id=9 type=`InterfaceSettingsWindow`
```

If the client panics, the panic message and a full backtrace are printed (no `RUST_BACKTRACE`
needed). Fatal signals like `SIGSEGV`/`SIGABRT` are also caught and print a marker plus a backtrace
before the signal is re-raised, so the log file ends with the crash information.


# Building and checking locally

The shader compiler `slangc` is not committed to this repository; the client build expects it on
`PATH`. Locally it lives in the workspace `tools/bin` directory:

```fish
cd /home/phantom/Games/ragnarok/CondaRO-linux/korangar
PATH="$PWD/../tools/bin:$PATH" cargo build
```

The same `PATH` prefix is needed for the quality gates (CI downloads the pinned `SLANG_VERSION`
into `$GITHUB_WORKSPACE/tools/bin` instead, so a plain `cargo clippy` works there):

```fish
PATH="$PWD/../tools/bin:$PATH" cargo fmt --all --check
PATH="$PWD/../tools/bin:$PATH" cargo clippy --workspace -- -Dwarnings
PATH="$PWD/../tools/bin:$PATH" cargo clippy --workspace --all-features -- -Dwarnings
PATH="$PWD/../tools/bin:$PATH" cargo test -p korangar -p korangar-networking -p ragnarok-packets
```

# Packet layout contract (2019)

The client speaks the rAthena `PACKETVER 20190605` protocol, but the packet structs are hand-written.
To stop 2019-vs-2022 layout drift from silently desynchronizing the stream, two committed artifacts
guard the wire format:

* `ragnarok-packets/src/opcode_lengths_2019.rs` - generated registry of the exact 2019 wire size of
  every server-to-client opcode (fixed size or variable-length base size).
* `ragnarok-packets/tests/contract_2019.rs` and the `contract_2019` module in `korangar-networking` -
  fixtures that feed synthetic 2019-sized buffers into the structs/handlers and assert that exactly
  the declared number of bytes is consumed.

Regenerate the registry after pinning a different server commit with:

```fish
./tools/gen_opcode_lengths.py           # reads the rAthena tree through ssh (see --help)
./tools/gen_opcode_lengths.py --check   # fail if the committed file is out of date
```

The generator preprocesses the rAthena headers with `-DPACKETVER=20190605`, runs a `sizeof()` probe
for every packet struct and merges the checked-in `tools/data/opcode_supplement_20190605.json` (for
hand-rolled char-server and dynamic `enum packet_headers` opcodes). Run the contract tests with:

```fish
cargo test -p ragnarok-packets --test contract_2019
cargo test -p korangar-networking --lib contract_2019
```

A layout mismatch is logged (once per opcode) but never breaks a running client; set
`KORANGAR_PACKET_STRICT=1` to turn it into a parse error for scripted runs. New opcodes must add a
registry entry (or an explicit entry in the contract ignore list) and a fixture.

# Troubleshooting
If the client keeps crashing or you have any other problems, please consult the wiki page on [Troubleshooting](Troubleshooting.md).
