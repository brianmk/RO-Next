<img align="left" alt="" src=".github/logo.png" height="130" />

# RO-Next

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)

RO-Next is a next-gen Ragnarok Online client written in Rust. It features
real-time lighting with drop shadows, a fully customizable user interface and
none of the official client's fixed-aspect-ratio limitations. It supports
Linux, Windows and macOS.

It is a fork of [Korangar](https://github.com/vE5li/korangar) (MIT,
© Lucas Pütz) tuned for the 2019 rAthena/CondaRO tables: official skill
effects, caster animations and status visuals verified against the decompiled
official client, with every data table generated from the game archives rather
than hardcoded in Rust.

##### Screenshot of the current state
![geffen](.github/geffen.png)

## 🚀 Running

> [!IMPORTANT]
> RO-Next is still very early in development and is anything but feature-complete.

If you want to try it out for yourself, check out the [Installation page](wiki/Installation.md).

## 🔧 Troubleshooting

If you're running into issues while setting up or running RO-Next please check the [Troubleshooting page](wiki/Troubleshooting.md).

## 📚 Documentation

* [`docs/`](docs/README.md) — skills/effects, client systems, data pipelines and
  the patcher release process.
* [`tools/README.md`](tools/README.md) — the extractors/generators behind the
  data tables in `ro-next/archive/data/client/`.

## 🤝 Contributing

This is a very ambitious project and we are always looking for contributors. If you are interested, please read [this page](wiki/Contributing.md).

## 🔥 Updates

There is a dedicated channel for `updates` on our [Discord server](https://discord.gg/2CqRZsvKja). If you want to stay up to date with development or see recent changes, go check it out!

## 📦 Packages

We try to keep the project as modular as possible by splitting the codebase into individual crates. All the crates prefixed with `ragnarok-` are **independent of RO-Next and have no dependencies on it**.

We encourage everyone to use these crates for their own Ragnarok Online related projects and contribute back if they want to.
