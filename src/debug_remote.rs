use bevy::prelude::*;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::{BrpResult, RemotePlugin};
use serde_json::json;

use crate::AppState;
use crate::physics::{ChunkPhysics, PendingPhysicsCollider};
use crate::player::{NeedsPhysicsRefresh, NeedsRenderRefresh, Player, PlayerCamera};
use crate::render::{ChunkMesh, PendingRenderMesh};
use crate::world::{Chunk, InitialWorldGeneration, World as GameWorld, chunk_world_size};

const BRP_HOST: [u8; 4] = [127, 0, 0, 1];
const BRP_PORT: u16 = 15702;

pub struct DebugRemotePlugin;

impl Plugin for DebugRemotePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RemotePlugin::default()
                .with_method("gecynd.debug.summary", debug_summary)
                .with_method("gecynd.debug.player", debug_player)
                .with_method("gecynd.debug.chunks", debug_chunks),
            RemoteHttpPlugin::default()
                .with_address(BRP_HOST)
                .with_port(BRP_PORT),
        ));
    }
}

fn debug_summary(
    _: In<Option<serde_json::Value>>,
    app_state: Res<State<AppState>>,
    game_world: Res<GameWorld>,
    generation: Res<InitialWorldGeneration>,
    players: Query<(), With<Player>>,
    cameras: Query<(), With<PlayerCamera>>,
    chunks: Query<(), With<Chunk>>,
    chunk_meshes: Query<(), With<ChunkMesh>>,
    chunk_physics: Query<(), With<ChunkPhysics>>,
    pending_render: Query<(), With<PendingRenderMesh>>,
    pending_physics: Query<(), With<PendingPhysicsCollider>>,
    needs_render: Query<(), With<NeedsRenderRefresh>>,
    needs_physics: Query<(), With<NeedsPhysicsRefresh>>,
) -> BrpResult {
    Ok(json!({
        "app_state": format!("{:?}", app_state.get()),
        "brp": {
            "host": BRP_HOST,
            "port": BRP_PORT,
        },
        "world": {
            "registered_chunks": game_world.chunks.len(),
            "pending_generation_chunks": game_world.pending_chunks.len(),
            "chunk_world_size": chunk_world_size(),
        },
        "initial_generation": {
            "started": generation.started,
            "finished": generation.finished,
            "completed_chunks": generation.completed_chunks,
            "total_chunks": generation.total_chunks,
        },
        "entities": {
            "players": players.iter().count(),
            "player_cameras": cameras.iter().count(),
            "chunk_components": chunks.iter().count(),
            "chunk_meshes": chunk_meshes.iter().count(),
            "chunk_physics": chunk_physics.iter().count(),
            "pending_render_meshes": pending_render.iter().count(),
            "pending_physics_colliders": pending_physics.iter().count(),
            "needs_render_refresh": needs_render.iter().count(),
            "needs_physics_refresh": needs_physics.iter().count(),
        }
    }))
}

fn debug_player(
    _: In<Option<serde_json::Value>>,
    player_query: Query<(&Transform, Option<&GlobalTransform>), With<Player>>,
    camera_query: Query<&GlobalTransform, With<PlayerCamera>>,
) -> BrpResult {
    let players = player_query
        .iter()
        .map(|(transform, global_transform)| {
            json!({
                "translation": vec3(transform.translation),
                "global_translation": global_transform.map(|global| vec3(global.translation())),
            })
        })
        .collect::<Vec<_>>();

    let cameras = camera_query
        .iter()
        .map(|transform| {
            json!({
                "global_translation": vec3(transform.translation()),
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "players": players,
        "cameras": cameras,
    }))
}

fn debug_chunks(
    _: In<Option<serde_json::Value>>,
    game_world: Res<GameWorld>,
    chunks: Query<&Chunk>,
    chunk_meshes: Query<(), With<ChunkMesh>>,
    chunk_physics: Query<(), With<ChunkPhysics>>,
    pending_render: Query<(), With<PendingRenderMesh>>,
    pending_physics: Query<(), With<PendingPhysicsCollider>>,
    needs_render: Query<(), With<NeedsRenderRefresh>>,
    needs_physics: Query<(), With<NeedsPhysicsRefresh>>,
) -> BrpResult {
    let mut sample = chunks
        .iter()
        .take(16)
        .map(|chunk| {
            json!({
                "coord": { "x": chunk.coord.x, "z": chunk.coord.z },
                "revision": chunk.revision,
                "modified": chunk.modified,
            })
        })
        .collect::<Vec<_>>();

    sample.sort_by_key(|value| {
        let coord = &value["coord"];
        (
            coord["x"].as_i64().unwrap_or_default(),
            coord["z"].as_i64().unwrap_or_default(),
        )
    });

    Ok(json!({
        "registered_chunks": game_world.chunks.len(),
        "pending_generation_chunks": game_world.pending_chunks.len(),
        "chunk_components": chunks.iter().count(),
        "chunk_meshes": chunk_meshes.iter().count(),
        "chunk_physics": chunk_physics.iter().count(),
        "pending_render_meshes": pending_render.iter().count(),
        "pending_physics_colliders": pending_physics.iter().count(),
        "needs_render_refresh": needs_render.iter().count(),
        "needs_physics_refresh": needs_physics.iter().count(),
        "sample": sample,
    }))
}

fn vec3(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}
