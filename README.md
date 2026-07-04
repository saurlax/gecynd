# Gecynd

Gecynd is a playable voxel sandbox prototype built with Rust and Bevy.

## Features

- Procedural terrain generation
- Chunk-based world loading and unloading
- Player movement, sprinting, jumping, and mouse look
- Voxel selection with block breaking and placement
- Hotbar material selection
- Main menu, pause menu, and save/load support
- Async chunk mesh and collider rebuilding
- Optional in-game debug info

## Requirements

- Rust stable toolchain
- A GPU and driver capable of running Bevy 3D applications

## Getting Started

Clone the repository and run the game:

```bash
cargo run
```

For a quick compile check:

```bash
cargo check
```

## Development

The project uses Bevy with a plugin-oriented structure. Source code lives in `src/`, and project notes live in `specs/`.

Useful commands:

```bash
cargo fmt
cargo clippy
cargo check
```
