// main.rs
// Raytracer de una isla estilo Minecraft: ventana, controles y modo de medición

mod block;
mod block_types;
mod events;
mod framebuffer;
mod grid;
mod light;
mod material;
mod ray_intersect;
mod scene;
mod snell;
mod textures;

use crate::events::handle_camera_input;
use crate::framebuffer::{Framebuffer, to_rgba};
use crate::grid::World;
use crate::light::Light;
use crate::scene::{create_optimized_scene, load_textures};
use crate::snell::trace_ray_multi_light;
use crate::textures::TextureManager;
use raylib::prelude::*;
use rayon::prelude::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};
use std::time::Instant;

const SCREEN_WIDTH: usize = 400;
const SCREEN_HEIGHT: usize = 300;
const RENDER_SCALE: i32 = 2;
/// Intensidad de las luces blancas de la escena
const FILL_LIGHT: f32 = 8.0;
const TOP_LIGHT: f32 = 7.0;
const FRONT_LIGHT: f32 = 14.0;
/// Rebotes máximos de reflexión y refracción por rayo
const MAX_BOUNCES: u32 = 2;

/// Bloques, luces y texturas de la escena
struct Scene {
    world: World,
    lights: Vec<Light>,
    textures: TextureManager,
}

impl Scene {
    fn load() -> Self {
        let mut blocks = create_optimized_scene();
        let mut textures = TextureManager::default();
        load_textures(&mut textures, &mut blocks);

        // Los bloques emisivos (magma y sol) también iluminan
        let mut lights: Vec<Light> = blocks.iter().filter_map(|b| b.emission).collect();
        // Luces blancas: una de relleno detrás, una cenital y una frontal que ilumina las caras que ve la cámara
        for (position, intensity) in [
            (Vector3::new(-5.0, 6.0, 5.0), FILL_LIGHT),
            (Vector3::new(0.0, 6.0, 0.0), TOP_LIGHT),
            (Vector3::new(-4.0, 8.0, -9.0), FRONT_LIGHT),
        ] {
            lights.push(Light::new(position, Vector3::one(), intensity));
        }

        Scene {
            world: World::new(blocks),
            lights,
            textures,
        }
    }
}

#[derive(Clone)]
struct CameraConfig {
    pos: Vector3,
    forward: Vector3,
    right: Vector3,
    up: Vector3,
    fov_tan: f32,
    aspect_ratio: f32,
}

impl CameraConfig {
    fn new(pos: Vector3, yaw: f32, pitch: f32) -> Self {
        let forward = Vector3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalized();
        let right = forward.cross(Vector3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        Self {
            pos,
            forward,
            right,
            up,
            fov_tan: (FRAC_PI_3 / 2.0).tan(),
            aspect_ratio: SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32,
        }
    }

    #[inline]
    fn ray_direction(&self, x: usize, y: usize) -> Vector3 {
        let px = (2.0 * ((x as f32 + 0.5) / SCREEN_WIDTH as f32) - 1.0)
            * self.fov_tan
            * self.aspect_ratio;
        let py = (1.0 - 2.0 * ((y as f32 + 0.5) / SCREEN_HEIGHT as f32)) * self.fov_tan;
        (self.forward + self.right * px + self.up * py).normalized()
    }
}

/// Traza un rayo por píxel; en paralelo reparte las filas entre los núcleos con Rayon
fn render(fb: &mut Framebuffer, camera: &CameraConfig, scene: &Scene, parallel: bool) {
    let width = fb.width;
    let shade_row = |(y, row): (usize, &mut [[u8; 4]])| {
        for (x, pixel) in row.iter_mut().enumerate() {
            let color = trace_ray_multi_light(
                camera.pos,
                camera.ray_direction(x, y),
                0,
                MAX_BOUNCES,
                &scene.world,
                &scene.lights,
                &scene.textures,
            );
            *pixel = to_rgba(color);
        }
    };
    if parallel {
        fb.pixels_mut()
            .par_chunks_mut(width)
            .enumerate()
            .for_each(shade_row);
    } else {
        fb.pixels_mut()
            .chunks_mut(width)
            .enumerate()
            .for_each(shade_row);
    }
}

fn main() {
    if std::env::args().any(|a| a == "--bench") {
        bench();
        return;
    }

    let (mut rl, thread) = raylib::init()
        .size(
            SCREEN_WIDTH as i32 * RENDER_SCALE,
            SCREEN_HEIGHT as i32 * RENDER_SCALE,
        )
        .title("Raytracer de vóxeles")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    rl.set_target_fps(60);

    let scene = Scene::load();
    let mut framebuffer = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    // La cámara arranca mirando la isla de frente
    let mut camera_pos = Vector3::new(0.0, 3.0, -8.0);
    let mut camera_yaw = FRAC_PI_2;
    let mut camera_pitch = -0.25_f32;
    let mut parallel = true;

    println!("W A S D            moverse");
    println!("Flechas            girar la cámara");
    println!("Espacio / Ctrl     subir o bajar");
    println!("T                  alternar entre un hilo y todos los núcleos");
    println!("Esc                salir");

    while !rl.window_should_close() {
        handle_camera_input(&rl, &mut camera_pos, &mut camera_yaw, &mut camera_pitch);
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            parallel = !parallel;
        }

        let camera = CameraConfig::new(camera_pos, camera_yaw, camera_pitch);
        let start = Instant::now();
        render(&mut framebuffer, &camera, &scene, parallel);
        let render_ms = start.elapsed().as_secs_f64() * 1000.0;

        let fps = rl.get_fps();
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        let dest = Rectangle::new(
            0.0,
            0.0,
            (SCREEN_WIDTH as i32 * RENDER_SCALE) as f32,
            (SCREEN_HEIGHT as i32 * RENDER_SCALE) as f32,
        );
        framebuffer.present_scaled(&mut d, &thread, dest);

        let pos = format!(
            "Pos: ({:.1}, {:.1}, {:.1})",
            camera_pos.x, camera_pos.y, camera_pos.z
        );
        let mode = if parallel {
            "Modo: todos los núcleos"
        } else {
            "Modo: un hilo"
        };
        d.draw_text(&format!("FPS: {fps}"), 10, 10, 20, Color::WHITE);
        d.draw_text(&pos, 10, 35, 16, Color::WHITE);
        d.draw_text(mode, 10, 60, 16, Color::WHITE);
        d.draw_text(
            &format!("Render: {render_ms:.1} ms"),
            10,
            85,
            16,
            Color::WHITE,
        );
        d.draw_text("T - alternar hilos", 10, 110, 14, Color::LIGHTGRAY);
    }
}

/// `cargo run --release -- --bench`: mide el tiempo por cuadro en varias vistas, sin abrir ventana
fn bench() {
    const FRAMES: u32 = 10;
    let scene = Scene::load();
    let mut fb = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    let views = [
        (
            "isla completa",
            Vector3::new(0.0, 3.0, -8.0),
            FRAC_PI_2,
            -0.25,
        ),
        (
            "casa de cerca",
            Vector3::new(-1.0, 1.8, -4.0),
            FRAC_PI_2,
            -0.1,
        ),
        (
            "ventana de vidrio",
            Vector3::new(-1.0, 2.0, 1.2),
            -FRAC_PI_2,
            0.0,
        ),
        ("lago", Vector3::new(1.5, 2.5, 5.5), -FRAC_PI_2, -0.5),
        (
            "dentro del vidrio",
            Vector3::new(-1.0, 2.0, 0.0),
            -FRAC_PI_2,
            0.0,
        ),
    ];
    println!(
        "{SCREEN_WIDTH}x{SCREEN_HEIGHT}, {} hilos, promedio de {FRAMES} cuadros",
        rayon::current_num_threads()
    );
    for (name, pos, yaw, pitch) in views {
        let camera = CameraConfig::new(pos, yaw, pitch);
        for (parallel, mode) in [(true, "todos los núcleos"), (false, "un hilo")] {
            let start = Instant::now();
            for _ in 0..FRAMES {
                render(&mut fb, &camera, &scene, parallel);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;
            println!(
                "{name:<20} {mode:<18} {ms:>7.1} ms  ({:.0} FPS)",
                1000.0 / ms
            );
        }
    }
}
