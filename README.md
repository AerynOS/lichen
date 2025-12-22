# lichen

## Overview
An experimental text based installer run from the command line to install AerynOS.

`lichen` has been developed to a minimum viable level to allow for installing AerynOS by experienced users. Prior to installation, users will need to format their disk with a GPT partition table with an esp partition, xbootldr partition and a root partition. Instructions for this can be found on our documentation [website](https://aerynos.dev/aerynos/faq/).

The AerynOS team are currently focusing on [infrastructure](https://github.com/AerynOS/infra) and [os-tooling](https://github.com/AerynOS/os-tools) development so `lichen` is currently in maintenance mode. There are however eventual plans to build upon `lichen` to provide a more user-friendly experience including a graphical user interface. In the meantime, we are still accepting contributions to help improve the codebase.

## Build & Test

    cargo build
    sudo ./target/debug/lichen

To quit the installer, press `ESC` to switch to command mode, then press `q`.

## License

`lichen` is available under the terms of the [MPL-2.0](https://spdx.org/licenses/MPL-2.0.html)
