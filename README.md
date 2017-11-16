# colonize

<table>
    <tr>
        <td><strong>Linux / OS X</strong></td>
        <td><a href="https://travis-ci.org/indiv0/colonize" title="Travis Build Status"><img src="https://travis-ci.org/indiv0/colonize.svg?branch=master" alt="travis-badge"></img></a></td>
    </tr>
    <tr>
        <td colspan="2">
            <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg" alt=license"></img>
        </td>
    </tr>
</table>

A Dwarf-Fortress/Rimworld-like game written in Rust.

***See the [changelog] for what's new in the most recent release.***

![colonize-screenshot](https://i.imgur.com/YI68SsY.jpg "Colonize - Game scene")

# Table of Contents

* [Introduction](#introduction)
* [Platforms & Tool Chains](#platforms--tool-chains)
* [Compiling & Running From Source](#compiling--running-from-source)
* [Configuration](#configuration)
* [Contributing](#contributing)
* [License](#license)

# Introduction

Colonize is a project of [mine](https://github.com/indiv0) to write a Dwarf
Fortress/Rimworld-like game in the Rust language.

My eventual vision is for this game to provide a real-time simulation of a world
in which individual entities (like dwarves in Dwarf Fortress) perform actions to
satisfy goals set by the player (e.g. "build a house").
The gameplay will focus on getting a player to build a fort/base and protect it
from threats, whether they be the elements, monsters, or various catastrophes.
For now, the game is intended to be single-player only, but I may attempt to add
multi-player co-op or challenge mode in the future.

I have written a few toy games here and there but this is my first project where
I intend to make a fully playable, enjoyable game from scratch.
As such, this project is developed at a very slow rate as I am learning game
programming and design as I go along.

The game will initially be developed to only support a top-down, layered view of
a world which is generated and rendered in chunks.
The project uses the SDL2 library to provide graphics/input/etc. capability.

## Note

**THIS PROJECT IS CURRENTLY UNDERGOING A FULL RE-WRITE**.
With the deprecation of the [glium][glium] library, this project is currently
undergoing a full re-write from scratch.

# Platforms & Tool Chains

`Colonize` should be compilable on any of the major rustc tool chains (stable,
beta, or nightly).

In the long run, `Colonize` intends to support all major platforms
(Windows/Mac OS X/Linux, 32-bit+64 bit).
However, at the moment, I can only afford to prioritize one or two platforms at
a time.
As such, **currently the game is now only actively developed and tested on
64-bit Linux**.
I may setup automated builds for other platforms in the future.
Contributors from all platforms are welcome, regardless of officially stated
platform support.

If you wish to help test or debug the game on any platform, please let me know!
Your help would be greatly appreciated.

## Compiling & Running From Source

Prerequisites:

* [rust](https://www.rust-lang.org)

Steps:

1. Compile the project with `cargo build`.
2. Run the game with `cargo run`.

## Contributing

Contributions are always welcome!
If you have an idea for something to add (code, documentation, tests, examples,
etc.) feel free to give it a shot.

Please read [CONTRIBUTING.md][contributing] before you start contributing.

## License

Colonize is distributed under the terms of both the MIT license and the Apache
License (Version 2.0).

See [LICENSE-APACHE][license-apache], and [LICENSE-MIT][license-mit] for details.

## Credits

The list of contributors to this project can be found at
[CONTRIBUTORS.md][contributors].

[changelog]: https://github.com/indiv0/colonize/blob/master/CHANGELOG.md
[contributing]: https://github.com/indiv0/colonize/blob/master/CONTRIBUTING.md "Contribution guide"
[contributors]: https://github.com/indiv0/colonize/blob/master/CONTRIBUTORS.md "List of contributors"
[glium]: https://users.rust-lang.org/t/glium-post-mortem/7063 "Glium deprecation post"
[license-apache]: https://github.com/indiv0/colonize/blob/master/LICENSE-APACHE "Apache-2.0 License"
[license-mit]: https://github.com/indiv0/colonize/blob/master/LICENSE-MIT "MIT License"
