This is a voxel sandbox game written using Bevy. Please use the Context7 tool to read the detailed documentation of Bevy.

Before making changes or answering project-specific questions, read the `specs/` directory first to find relevant project notes, research, and requirements.

## Project Status

### Snapshot

- Project name: `gecynd`
- Current version: `0.1.0`
- Main branch observed: `main`
- Build status verified on this workspace: `cargo check` passes on 2026-06-11
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
- bottom hotbar material selection with number keys and mouse wheel
- in-game pause menu with resume and return-to-main-menu flow
- save/load support for the default world directory
- async chunk loading and unloading around the player
- async chunk mesh rebuilding
- async chunk collider rebuilding
- lightweight in-game debug info toggle and status UI

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
  - handles voxel targeting, material selection, and edit interactions
  - marks chunks and border neighbors dirty after edits
- `src/render.rs`
  - builds chunk meshes from voxel data
  - manages chunk mesh entities, voxel highlight rendering, and crosshair UI
- `src/physics.rs`
  - builds Rapier trimesh colliders from voxel data
- `src/ui.rs`
  - shows the main menu, loading overlay, pause menu, hotbar, and optional debug info
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
  - `F1`: toggle lightweight debug info

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
- Project notes and research live in `specs/`
- There is no dedicated test suite in the repository yet
- `target/` is present locally and ignored from source control

### Known Technical Limits

- Chunk rendering does not yet cull faces across chunk boundaries
- Chunk physics still rebuilds full trimesh colliders for edited chunks
- Terrain generation is simple heightmap terrain, not caves or volumetric structures
- Building is still raw voxel placement and destruction, without higher-level tools
- There is no movable voxel-object or debris system yet

## Working Guidelines

- Preserve the current plugin-oriented structure unless there is a strong reason to change it
- Keep rendering, physics, world simulation, and player logic decoupled
- Treat voxel data as the source of truth, with meshes and colliders as derived data
- Prefer Bevy task pools for heavy background work
- When changing Bevy APIs or patterns, verify them with Context7 instead of relying on memory

## Debugging Workflow

### Windows Desktop Automation

- The `windows-desktop-e2e` skill is installed at `~/.agents/skills/windows-desktop-e2e`.
- Use it when an agent needs to launch the native Windows game window, bring it forward, send keyboard or mouse input, and capture screenshots for visual inspection.
- Prefer real desktop-region screenshots over `PrintWindow`-style capture for Bevy/Vulkan windows, because GPU-rendered windows may otherwise capture as black.
- For visual debugging, combine screenshots with BRP ECS queries instead of relying on screenshots alone.

### Bevy Remote Protocol

- Debug builds enable Bevy Remote Protocol over HTTP on `127.0.0.1:15702`.
- The BRP setup lives in `src/debug_remote.rs` and is only compiled when `debug_assertions` are enabled.
- Release builds must not expose this debug HTTP endpoint unless explicitly requested.
- Built-in BRP methods such as `world.query`, `world.list_components`, `world.get_components`, and `world.get_resources` are available.
- Project-specific methods are available:
  - `gecynd.debug.summary`
  - `gecynd.debug.player`
  - `gecynd.debug.chunks`

Example PowerShell request:

```powershell
$body = @{ jsonrpc = "2.0"; method = "gecynd.debug.summary"; id = 1 } | ConvertTo-Json -Compress
Invoke-RestMethod -Uri "http://127.0.0.1:15702/" -Method Post -ContentType "application/json" -Body $body
```

Use `gecynd.debug.summary` to quickly check:

- current `AppState`
- player and camera entity counts
- chunk entity count
- chunk mesh and physics counts
- pending render and physics task counts
- dirty render and physics refresh marker counts
- initial world generation progress

Use `gecynd.debug.player` to inspect player and camera transforms.

Use `gecynd.debug.chunks` to inspect chunk counts and a small chunk sample without serializing full voxel buffers.

When debugging rendering, prefer this loop:

1. Start the game with `cargo run` or `target/debug/gecynd.exe`.
2. Capture the game window screenshot with desktop automation.
3. Query `gecynd.debug.summary`.
4. If terrain is missing visually, compare `chunk_components`, `chunk_meshes`, `pending_render_meshes`, and `needs_render_refresh`.
5. If physics works but terrain is invisible, focus on render components, materials, camera settings, visibility, lighting, and post-processing.
6. If chunks are missing in both rendering and physics, focus on world generation, save loading, chunk lifecycle, and async task completion.

## Style Guide

Your code should be kept simple and clear, with attention paid to scalability and compatibility. Do not write temporary code. Only add comments when necessary, and use English.
