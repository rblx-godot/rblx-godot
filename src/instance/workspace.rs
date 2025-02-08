use r2g_mlua::prelude::*;

use crate::{
    core::{
        lua_macros::lua_getter, DynInstance, IInstance, IInstanceComponent, IObject,
        InheritanceBase, InheritanceTableBuilder, InstanceComponent, InstanceCreationMetadata, Irc,
        ManagedInstance, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
    userdata::{
        enums::{
            AnimatorRetargetingMode, AvatarUnificationMode, ClientAnimatorThrottlingMode,
            FluidForces, IKControlConstraintSupport, MeshPartHeadsAndAccessories,
            MoverConstraintRootBehaviorMode, PathfindingUseImprovedSearch, PhysicsSteppingMethod,
            PlayerCharacterDestroyBehavior, PrimalPhysicsSolver, RejectCharacterDeletions,
            RenderingCacheOptimizationMode, ReplicateInstanceDestroySetting, RollOutState,
            SandboxedInstanceMode, StreamOutBehavior, StreamingIntegrityMode,
        },
        ManagedRBXScriptSignal, RBXScriptSignal, Vector3,
    },
};

use super::{IModel, IPVInstance, ModelComponent, PVInstanceComponent};

#[derive(Debug)]
pub struct WorkspaceComponent {
    pub persistent_loaded: ManagedRBXScriptSignal,

    air_density: f64,
    allow_third_party_sales: bool,
    avatar_unification_mode: AvatarUnificationMode,
    client_animator_throttling: ClientAnimatorThrottlingMode,
    current_camera: Option<ManagedInstance>, // todo!
    distributed_game_time: f64,
    fall_height_enabled: bool,
    fallen_parts_destroy_height: f64,
    fluid_forces: FluidForces,
    global_wind: Vector3,
    gravity: f64,
    ik_control_constraint_support: IKControlConstraintSupport,
    insert_point: Vector3,
    mesh_part_heads_and_accessories: MeshPartHeadsAndAccessories,
    mover_constraint_root_behavior: MoverConstraintRootBehaviorMode,
    pathfinding_use_improved_search: PathfindingUseImprovedSearch,
    physics_stepping_method: PhysicsSteppingMethod,
    player_character_destroy_behavior: PlayerCharacterDestroyBehavior,
    primal_physics_solver: PrimalPhysicsSolver,
    reject_character_deletions: RejectCharacterDeletions,
    rendering_cache_optimizations: RenderingCacheOptimizationMode,
    replicate_instance_destroy_string: ReplicateInstanceDestroySetting,
    retargeting: AnimatorRetargetingMode,
    sandboxed_instance_mode: SandboxedInstanceMode,
    //signal_behavior inside fastflags
    stream_out_behavior: StreamOutBehavior,
    streaming_enabled: bool,
    streaming_integrity_mode: StreamingIntegrityMode,
    streaming_min_radius: f64,
    streaming_target_radius: f64,
    terrain: Option<ManagedInstance>, //todo!
    touch_events_use_collision_groups: RollOutState,
    touches_use_collision_groups: bool,
}
#[derive(Debug)]
pub struct Workspace {
    instance_component: RwLock<InstanceComponent>,
    pvinstance_component: RwLock<PVInstanceComponent>,
    model_component: RwLock<ModelComponent>,
    workspace_component: RwLock<WorkspaceComponent>,
}

impl InheritanceBase for Workspace {
    fn inheritance_table(&self) -> crate::core::InheritanceTable {
        InheritanceTableBuilder::new()
            .insert_type::<Workspace, dyn IObject>(|x| x, |x| x)
            .insert_type::<Workspace, DynInstance>(|x| x, |x| x)
            .insert_type::<Workspace, dyn IPVInstance>(|x| x, |x| x)
            .insert_type::<Workspace, dyn IModel>(|x| x, |x| x)
            .insert_type::<Workspace, Workspace>(|x| x, |x| x)
            .output()
    }
}

impl IObject for Workspace {
    fn lua_get(&self, lua: &Lua, name: String) -> LuaResult<LuaValue> {
        self.workspace_component
            .read()
            .unwrap()
            .lua_get(self, lua, &name)
            .or_else(|| self.get_model_component().lua_get(self, lua, &name))
            .or_else(|| self.get_pv_instance_component().lua_get(self, lua, &name))
            .unwrap_or_else(|| self.get_instance_component().lua_get(lua, &name))
    }

    fn get_class_name(&self) -> &'static str {
        "Workspace"
    }

    fn get_property_changed_signal(&self, property: String) -> ManagedRBXScriptSignal {
        self.get_instance_component()
            .get_property_changed_signal(property)
            .unwrap()
    }

    fn is_a(&self, class_name: &String) -> bool {
        match class_name.as_str() {
            "Workspace" => true,
            "Model" => true,
            "PVInstance" => true,
            "Instance" => true,
            "Object" => true,
            _ => false,
        }
    }

    fn get_changed_signal(&self) -> ManagedRBXScriptSignal {
        self.get_instance_component().changed.clone()
    }
}

impl IInstance for Workspace {
    fn get_instance_component(&self) -> RwLockReadGuard<InstanceComponent> {
        self.instance_component.read().unwrap()
    }

    fn get_instance_component_mut(&self) -> RwLockWriteGuard<InstanceComponent> {
        self.instance_component.write().unwrap()
    }

    fn lua_set(&self, lua: &Lua, name: String, val: LuaValue) -> LuaResult<()> {
        self.workspace_component
            .write()
            .unwrap()
            .lua_set(self, lua, &name, &val)
            .or_else(|| {
                self.get_model_component_mut()
                    .lua_set(self, lua, &name, &val)
            })
            .or_else(|| {
                self.get_pv_instance_component_mut()
                    .lua_set(self, lua, &name, &val)
            })
            .unwrap_or_else(|| self.get_instance_component_mut().lua_set(lua, &name, val))
    }

    fn clone_instance(&self, _: &Lua) -> LuaResult<ManagedInstance> {
        Err(LuaError::RuntimeError("Cannot clone Workspace".to_string()))
    }
}

impl IPVInstance for Workspace {
    fn get_pv_instance_component(&self) -> RwLockReadGuard<'_, PVInstanceComponent> {
        self.pvinstance_component.read().unwrap()
    }

    fn get_pv_instance_component_mut(&self) -> RwLockWriteGuard<'_, PVInstanceComponent> {
        self.pvinstance_component.write().unwrap()
    }
}

impl IModel for Workspace {
    fn get_model_component(&self) -> RwLockReadGuard<'_, ModelComponent> {
        self.model_component.read().unwrap()
    }

    fn get_model_component_mut(&self) -> RwLockWriteGuard<'_, ModelComponent> {
        self.model_component.write().unwrap()
    }
}

impl IInstanceComponent for WorkspaceComponent {
    fn lua_get(
        self: &mut RwLockReadGuard<'_, Self>,
        _: &DynInstance,
        lua: &Lua,
        key: &String,
    ) -> Option<LuaResult<LuaValue>> {
        match key.as_str() {
            "AirDensity" => Some(lua_getter!(lua, self.air_density)),
            "AllowThirdPartySales" => Some(lua_getter!(lua, self.allow_third_party_sales)),
            "ClientAnimatorThrottling" => Some(lua_getter!(lua, self.client_animator_throttling)),
            "CurrentCamera" => Some(lua_getter!(clone, lua, self.current_camera)),
            "DistributedGameTime" => Some(lua_getter!(lua, self.distributed_game_time)),
            "FallHeightEnabled" => Some(lua_getter!(lua, self.fall_height_enabled)), // todo! PluginSecurity
            "FallenPartsDestroyHeight" => Some(lua_getter!(lua, self.fallen_parts_destroy_height)), // todo! PluginSecurity
            "GlobalWind" => Some(lua_getter!(lua, self.global_wind)),
            "Gravity" => Some(lua_getter!(lua, self.gravity)),
            "InsertPoint" => Some(lua_getter!(lua, self.insert_point)),
            "Retargeting" => Some(lua_getter!(lua, self.retargeting)),
            "StreamingEnabled" => Some(lua_getter!(lua, self.streaming_enabled)), // todo!
            "Terrain" => Some(lua_getter!(clone, lua, self.terrain)),

            "PersistentLoaded" => Some(lua_getter!(clone, lua, self.persistent_loaded)),
            _ => None,
        }
    }

    fn lua_set(
        self: &mut RwLockWriteGuard<'_, Self>,
        ptr: &DynInstance,
        lua: &Lua,
        key: &String,
        value: &LuaValue,
    ) -> Option<LuaResult<()>> {
        todo!()
    }

    fn clone(
        self: &RwLockReadGuard<'_, Self>,
        _: &Lua,
        _: &InstanceCreationMetadata,
    ) -> LuaResult<Self> {
        Err(LuaError::RuntimeError(
            "Cannot clone WorkspaceComponent".to_string(),
        ))
    }

    fn new(metadata: &InstanceCreationMetadata) -> Self {
        WorkspaceComponent {
            persistent_loaded: RBXScriptSignal::new(metadata),

            air_density: 0.0,
            allow_third_party_sales: false,
            avatar_unification_mode: AvatarUnificationMode::Default,
            client_animator_throttling: ClientAnimatorThrottlingMode::Default,
            current_camera: None,
            distributed_game_time: 0.0,
            fall_height_enabled: false,
            fallen_parts_destroy_height: 0.0,
            fluid_forces: FluidForces::Default,
            global_wind: Vector3::ZERO,
            gravity: 98.0,
            ik_control_constraint_support: IKControlConstraintSupport::Default,
            insert_point: Vector3::new(0.0, 10.0, 0.0),
            mesh_part_heads_and_accessories: MeshPartHeadsAndAccessories::Default,
            mover_constraint_root_behavior: MoverConstraintRootBehaviorMode::Default,
            pathfinding_use_improved_search: PathfindingUseImprovedSearch::Default,
            physics_stepping_method: PhysicsSteppingMethod::Default,
            player_character_destroy_behavior: PlayerCharacterDestroyBehavior::Default,
            primal_physics_solver: PrimalPhysicsSolver::Default,
            reject_character_deletions: RejectCharacterDeletions::Default,
            rendering_cache_optimizations: RenderingCacheOptimizationMode::Default,
            replicate_instance_destroy_string: ReplicateInstanceDestroySetting::Default,
            retargeting: AnimatorRetargetingMode::Default,
            sandboxed_instance_mode: SandboxedInstanceMode::Default,
            stream_out_behavior: StreamOutBehavior::Default,
            streaming_enabled: false,
            streaming_integrity_mode: StreamingIntegrityMode::Default,
            streaming_min_radius: 0.0,
            streaming_target_radius: 0.0,
            terrain: None,
            touch_events_use_collision_groups: RollOutState::Default,
            touches_use_collision_groups: false,
        }
    }
}

impl Workspace {
    pub fn new() -> Irc<Workspace> {
        let inst = Irc::new_cyclic(|x| {
            let metadata = InstanceCreationMetadata::new("Workspace", x.cast_to_instance());
            let mut r = Workspace {
                instance_component: RwLock::new_with_flag_auto(InstanceComponent::new(&metadata)),
                pvinstance_component: RwLock::new_with_flag_auto(PVInstanceComponent::new(
                    &metadata,
                )),
                model_component: RwLock::new_with_flag_auto(ModelComponent::new(&metadata)),
                workspace_component: RwLock::new_with_flag_auto(WorkspaceComponent::new(&metadata)),
            };
            DynInstance::submit_metadata(&mut r, metadata);
            r
        });
        DynInstance::set_name(&*inst, "Workspace".into()).unwrap();
        inst
    }
}
