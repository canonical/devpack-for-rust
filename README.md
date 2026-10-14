# devpack-for-rust

Zero to Rust in seconds.
An easy installer for Rustup, IDEs, and Rusty accessories on Linux.

[Get it from the Snap Store now!](https://snapcraft.io/devpack-for-rust)

```
$ sudo snap install devpack-for-rust --classic
```

## What it Does

devpack-for-rust is an opinionated tree-based install wizard for Rust development.
You can quickly pick which Rust channel you want, an IDE, and some popular Rust-based CLI tools.
Then it handles running `snap`, `rustup`, and `cargo install` for you.

Use the arrow-keys or HJKL to navigate the tree, and space/enter to make your choices.

## Contributing

I'd love your feedback!
I'm especially looking for Rust-based CLI tools for those who want to really oxidize their workflows.

## Testing

The Spread integration suite installs the built snap into LXD virtual machines.
It checks the snap's command-line interface and drives the interactive wizard
to select stable Rust, `just`, and `ripgrep`, then verifies all three tools,
compiles and runs a Rust program, and runs a `just` recipe.

Run it after setting up LXD and Snapcraft:

```sh
snapcraft pack --use-lxd
snapcraft test
```

The suite runs on the Ubuntu systems listed in [spread.yaml](./spread.yaml).

## License

Copyright (C) 2026 Canonical Ltd.

This project is free software, licensed under the GNU General Public License
version 3 (or, at your option, any later version). See the [LICENSE](./LICENSE)
file for the full text.

PLEASE NOTE that the tools that devpack-for-rust installs are under their own licenses, and are not affiliated with devpack-for-rust.
See their repositories for their licensing information.

The thumbnails (`images/thumbnail.svg`, and their PNG exports) are released under CC0 1.0 into the public domain.
It is a combination of (Karen Tölva's Ferris design)[https://www.rustacean.net/], licensed under CC0 1.0, and (Twemoji emote "Technologist")[https://github.com/twitter/twemoji/], licensed under MIT.
