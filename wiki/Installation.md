# Requirements

### 🦀 Cargo

Cargo (the Rust package manager) can best be installed and managed through [Rustup](https://rustup.rs/).

Note that RO-Next requires Rust nightly to compile. If you need help configuring Rustup accordingly, please
read [this stackoverflow issue](https://stackoverflow.com/questions/58226545/how-to-switch-between-rust-toolchains).

# OS-specific

### 🪟 Notes on Windows

On Windows you will need the following additional dependencies to be installed:

- Git (https://git-scm.com/downloads/win)
- Slangc (https://github.com/shader-slang/slang/releases)

Slangc is also part of the VulkanSDK and can be installed with it.
You can also use the windows packet manager winget to download the dependencies.

e.g.:

```powershell
winget install --id Git.Git -e --source winget
winget install --id KhronosGroup.VulkanSDK -e --source winget
```

### ❄️ Nix & NixOS

There is a `flake.nix` in the repository that exposes a dev shell with all dependencies for testing and running RO-Next
on Linux and MacOS.

# Compiling

You can compile RO-Next by running:

```fish
cargo build --release
```

### 🪲 Debug tools

To gain access to RO-Next's built-in developer tools, explicitly enable the `debug` feature:

```fish
cargo build --release --features debug
```

If your terminal supports Unicode, you might want to enable the `unicode` feature as well:

```fish
cargo build --release --features "debug unicode"
```

# Running

For information on how to use RO-Next, please take a look at the wiki page called [Running](Running.md).
