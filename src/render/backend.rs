use godot::classes::rendering_server::{
    EnvironmentAmbientSource, EnvironmentBg, EnvironmentReflectionSource, EnvironmentToneMapper,
};
use godot::classes::{Os, RenderingServer};
use godot::prelude::*;

use crate::core::{Trc, Weak};
use crate::userdata::{CFrame, Color3};

impl From<Transform3D> for CFrame {
    fn from(transform: Transform3D) -> Self {
        CFrame {
            rot_matrix: [
                [
                    transform.basis.col_a().x as f64,
                    transform.basis.col_a().y as f64,
                    transform.basis.col_a().z as f64,
                ],
                [
                    transform.basis.col_b().x as f64,
                    transform.basis.col_b().y as f64,
                    transform.basis.col_b().z as f64,
                ],
                [
                    transform.basis.col_c().x as f64,
                    transform.basis.col_c().y as f64,
                    transform.basis.col_c().z as f64,
                ],
            ],
            pos: [
                transform.origin.x as f64,
                transform.origin.y as f64,
                transform.origin.z as f64,
            ],
        }
    }
}
impl From<CFrame> for Transform3D {
    fn from(cframe: CFrame) -> Self {
        Transform3D::new(
            Basis::from_cols(
                Vector3::new(
                    cframe.rot_matrix[0][0] as f32,
                    cframe.rot_matrix[0][1] as f32,
                    cframe.rot_matrix[0][2] as f32,
                ),
                Vector3::new(
                    cframe.rot_matrix[1][0] as f32,
                    cframe.rot_matrix[1][1] as f32,
                    cframe.rot_matrix[1][2] as f32,
                ),
                Vector3::new(
                    cframe.rot_matrix[2][0] as f32,
                    cframe.rot_matrix[2][1] as f32,
                    cframe.rot_matrix[2][2] as f32,
                ),
            ),
            Vector3::new(
                cframe.pos[0] as f32,
                cframe.pos[1] as f32,
                cframe.pos[2] as f32,
            ),
        )
    }
}

impl From<Color3> for Color {
    fn from(color: Color3) -> Self {
        Color {
            r: color.r as f32,
            g: color.g as f32,
            b: color.b as f32,
            a: 1.0, // Default alpha value
        }
    }
}
impl From<Color> for Color3 {
    fn from(color: Color) -> Self {
        Color3 {
            r: color.r as f32,
            g: color.g as f32,
            b: color.b as f32,
        }
    }
}

#[derive(Debug)]
struct MultiMeshData {
    multimesh: Rid,
    instance: Rid,
    instance_count: u32,
    instance_capacity: u32,
    free_ids: Vec<u32>,
}

#[derive(Debug)]
enum InstanceOwner {
    Multimesh(Trc<MultiMeshData>),
    SingleInstance { instance: Rid, mesh: Rid },
}

#[derive(Debug)]
pub struct InstanceData {
    parent: Option<Trc<InstanceData>>,
    children: Vec<Trc<InstanceData>>,
    local_transform: Transform3D,
    world_transform: Transform3D,
    owner: InstanceOwner,
    backend: Weak<RenderBackend>,
}

#[derive(Debug)]
pub struct RenderBackend {
    primitive_multimeshes: [Trc<MultiMeshData>; 5],
    scenario: Rid,
    instances: Vec<Weak<InstanceData>>,

    /// Environments
    environment: Rid,
    environment_sky: Rid,

    fallback_environment: Rid,
}

macro_rules! execute_on_render_thread {
    ($name: ident => $code: block) => {
        RenderingServer::singleton().call_on_render_thread(&Callable::from_sync_fn(stringify!($name), move |_| {
            $code
            Ok(Variant::nil())
        }))
    }

}

impl RenderBackend {
    pub(crate) fn new() -> Self {
        assert!(
            Os::singleton().get_main_thread_id() == Os::singleton().get_thread_caller_id(),
            "RenderBackend must be created on the main thread"
        );

        let mut rendering_server = RenderingServer::singleton();
        let scenario = rendering_server.scenario_create();
        let environment = rendering_server.environment_create();
        let fallback_environment = rendering_server.environment_create();
        let mut empty_multimesh_fn = || {
            Trc::new({
                let multimesh_rid = rendering_server.multimesh_create();
                let instance_rid = rendering_server.instance_create2(multimesh_rid, scenario);
                MultiMeshData {
                    multimesh: multimesh_rid,
                    instance: instance_rid,
                    instance_count: 0,
                    instance_capacity: 0,
                    free_ids: Vec::new(),
                }
            })
        };
        let backend = RenderBackend {
            primitive_multimeshes: [
                empty_multimesh_fn(),
                empty_multimesh_fn(),
                empty_multimesh_fn(),
                empty_multimesh_fn(),
                empty_multimesh_fn(),
            ],
            scenario,
            environment,
            fallback_environment,
            environment_sky: rendering_server.sky_create(),
            instances: Vec::new(),
        };
        rendering_server.scenario_set_environment(scenario, environment);
        rendering_server.scenario_set_fallback_environment(scenario, fallback_environment);

        {
            // Fallback environment setup
            rendering_server
                .environment_set_ambient_light_ex(
                    fallback_environment,
                    Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    },
                )
                .ambient(EnvironmentAmbientSource::COLOR)
                .reflection_source(EnvironmentReflectionSource::DISABLED)
                .sky_contibution(0.0)
                .done();
        }
        {
            // Environment setup
            rendering_server.environment_glow_set_use_bicubic_upscale(true);
            let sky = backend.environment_sky;
            rendering_server.environment_set_sky(environment, sky);
            rendering_server
                .environment_set_ambient_light_ex(
                    environment,
                    Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    },
                )
                .energy(1.0)
                .ambient(EnvironmentAmbientSource::COLOR)
                .sky_contibution(0.0)
                .reflection_source(EnvironmentReflectionSource::BG)
                .done();
            rendering_server.environment_set_background(environment, EnvironmentBg::COLOR);
            rendering_server.environment_set_bg_energy(environment, 1.0, 0.0);
            rendering_server.environment_set_bg_color(
                environment,
                Color {
                    r: 0.0,
                    g: 0.9,
                    b: 1.0,
                    a: 1.0,
                },
            );
            rendering_server.environment_set_tonemap(
                environment,
                EnvironmentToneMapper::ACES,
                1.0,
                16.0,
            );
        }

        backend
    }
    pub(crate) fn scenario(&self) -> Rid {
        self.scenario
    }
    pub(crate) fn update_ambient(&self, color: Color3, brightness: f32, sky_contribution: f32) {
        let environment = self.environment;
        execute_on_render_thread!(update_ambient => {
            RenderingServer::singleton().environment_set_ambient_light_ex(environment, color.into())
                .energy(brightness)
                .sky_contibution(sky_contribution)
                .done();
        })
    }
}
