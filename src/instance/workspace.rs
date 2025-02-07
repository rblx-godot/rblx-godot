use crate::{
    core::RwLock,
    userdata::{ManagedRBXScriptSignal, Vector3},
};

use super::InstanceComponent;

pub struct WorkspaceComponent {
    pub persistent_loaded: ManagedRBXScriptSignal,

    air_density: f64,
    allow_third_party_sales: bool,
    avatar_unification_mode: (),    //todo!
    client_animator_throttling: (), //todo!
    current_camera: (),             //todo!
    distributed_game_time: f64,
    fall_height_enabled: bool,
    fallen_parts_destroy_height: f64,
    fluid_forces: (), //todo!
    global_wind: Vector3,
    gravity: f64,
    ik_control_constraint_support: (), //todo!
    insert_point: Vector3,
    mesh_part_heads_and_accessories: (),   //todo!
    mover_constraint_root_behavior: (),    //todo!
    pathfinding_use_improved_search: (),   //todo!
    physics_stepping_method: (),           //todo!
    player_character_destroy_behavior: (), //todo!
    primal_physics_solver: (),             //todo!
    reject_character_deletions: (),        //todo!
    rendering_cache_optimizations: (),     //todo!
    replicate_instance_destroy_string: (), //todo!
    retargeting: (),                       //todo!
    sandboxed_instance_mode: (),           //todo!
    //signal_behavior inside fastflags
    stream_out_behavior: (), //todo!
    streaming_enabled: bool,
    streaming_integrity_mode: (), //todo!
    streaming_min_radius: f64,
    streaming_target_radius: f64,
    terrain: (),                           //todo!
    touch_events_use_collision_groups: (), //todo!
    touches_use_collision_groups: bool,
}

pub struct Workspace {
    instance_component: RwLock<InstanceComponent>,
    workspace_component: RwLock<WorkspaceComponent>,
}
