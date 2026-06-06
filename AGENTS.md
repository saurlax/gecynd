This is a voxel sandbox game written using Bevy. Please use the Context7 tool to read the detailed documentation of Bevy.

## Project Status

### Snapshot

- Project name: `gecynd`
- Current version: `0.1.0`
- Main branch observed: `main`
- Build status verified on this workspace: `cargo check` passes on 2026-06-06
- Engine stack:
  - `bevy = 0.18.1`
  - `bevy_rapier3d = 0.34.0`
  - `noise = 0.9.0`

### Current Gameplay State

The project is currently a playable voxel sandbox prototype with:

- procedural terrain generation
- player spawning, movement, sprinting, jumping, and mouse look
- voxel selection with 3D DDA traversal
- left click block destruction
- right click block placement
- async chunk loading and unloading around the player
- async chunk mesh rebuilding
- async chunk collider rebuilding
- simple in-game debug and status UI

### Current Architecture

- `src/main.rs`
  - creates the Bevy app
  - installs default plugins
  - registers the game plugins
- `src/world.rs`
  - owns chunk lifecycle and world resources
  - runs initial world generation
  - loads and unloads nearby chunks
  - stores chunks in a `HashMap<ChunkCoord, Entity>`
- `src/terrain.rs`
  - generates chunk voxel data from Perlin noise
  - currently produces layered grass, dirt, and stone terrain
- `src/player.rs`
  - controls player movement and camera look
  - handles voxel targeting and edit interactions
  - marks chunks and border neighbors dirty after edits
- `src/render.rs`
  - builds chunk meshes from voxel data
  - manages chunk mesh entities, highlight rendering, crosshair, and debug AABBs
- `src/physics.rs`
  - builds Rapier trimesh colliders from voxel data
  - syncs physics debug rendering
- `src/ui.rs`
  - shows player position, selected voxel information, loading overlay, and controls
- `src/voxel.rs`
  - defines voxel size, voxel types, and voxel face helpers

### World Data Model

- Chunks use linear voxel storage: `Vec<Voxel>`
- Chunk dimensions:
  - horizontal size: `32 x 32`
  - height: `256`
- Voxel density:
  - `VOXELS_PER_METER = 16.0`
  - `VOXEL_SIZE = 1.0 / 16.0`
- Each chunk tracks a `revision` counter so async render and physics results can ignore stale work

### Async Work Model

- Initial world generation uses `AsyncComputeTaskPool`
- Runtime chunk generation uses `AsyncComputeTaskPool`
- Chunk mesh generation uses `AsyncComputeTaskPool`
- Chunk collider generation uses `AsyncComputeTaskPool`
- Completed tasks are polled on the main world and only applied if the chunk revision still matches

This matches current Bevy guidance: frame logic should stay in `Update`, deterministic movement can live in `FixedUpdate`, and heavy non-frame-critical CPU work should use `AsyncComputeTaskPool` instead of ad hoc OS thread spawning.

### Rendering and Physics Notes

- Rendering and physics are already separated into different plugins
- Chunk rendering currently builds face meshes from solid voxels
- Face culling currently only checks neighbors inside the same chunk
- Physics currently uses Rapier fixed-body trimesh colliders per chunk
- Voxel highlight and crosshair are implemented
- Debug toggles:
  - `F1`: chunk AABB debug
  - `F2`: render wireframe
  - `F3`: physics wireframe

### Current Terrain and Loading Behavior

- The initial spawn position is above the world at `initial_player_spawn_position()`
- Initial terrain generation preloads a circular chunk area around the spawn chunk
- Runtime chunk loading is based on player position
- Runtime chunk unloading removes far chunks with a small buffer beyond render distance
- Terrain shape is generated from Perlin height noise with a grass, dirt, and stone layering model

### Build and Release Workflow

- Local dev profile is configured with:
  - crate opt-level `1`
  - dependency opt-level `3`
- Release workflow exists at `.github/workflows/release.yml`
- Current release CI targets:
  - Windows x86_64 zip
  - macOS tar.gz
- Linux release packaging is currently commented out in the workflow

### Repository Contents Observed

- Source code lives in `src/`
- Research notes live in `docs/voxel-research.md`
- There is no dedicated test suite in the repository yet
- `target/` is present locally and ignored from source control

### Known Technical Limits

- Chunk rendering does not yet cull faces across chunk boundaries
- Chunk physics still rebuilds full trimesh colliders for edited chunks
- Terrain generation is simple heightmap terrain, not caves or volumetric structures
- Building is still raw voxel placement and destruction, without higher-level tools
- There is no save/load pipeline yet
- There is no movable voxel-object or debris system yet

## Working Guidelines

- Preserve the current plugin-oriented structure unless there is a strong reason to change it
- Keep rendering, physics, world simulation, and player logic decoupled
- Treat voxel data as the source of truth, with meshes and colliders as derived data
- Prefer Bevy task pools for heavy background work
- When changing Bevy APIs or patterns, verify them with Context7 instead of relying on memory

## Style Guide

Your code should be kept simple and clear, with attention paid to scalability and compatibility. Do not write temporary code. Only add comments when necessary, and use English.
