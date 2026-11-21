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

# Troubleshooting
If the client keeps crashing or you have any other problems, please consult the wiki page on [Troubleshooting](Troubleshooting.md).
