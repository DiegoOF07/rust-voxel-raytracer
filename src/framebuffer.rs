// framebuffer.rs
// Imagen del cuadro en CPU y su presentación escalada en la ventana

use raylib::prelude::*;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pixels: Vec<[u8; 4]>,
    texture: Option<Texture2D>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0, 255]; width * height],
            texture: None,
        }
    }

    pub fn pixels_mut(&mut self) -> &mut [[u8; 4]] {
        &mut self.pixels
    }

    /// Sube el cuadro a una textura que se reutiliza y la dibuja escalada en `dest`
    pub fn present_scaled(
        &mut self,
        d: &mut RaylibDrawHandle,
        thread: &RaylibThread,
        dest: Rectangle,
    ) {
        if self.texture.is_none() {
            let image = Image::gen_image_color(self.width as i32, self.height as i32, Color::BLACK);
            self.texture = d.load_texture_from_image(thread, &image).ok();
        }
        let Some(texture) = self.texture.as_mut() else {
            return;
        };
        if texture.update_texture(self.pixels.as_flattened()).is_err() {
            return;
        }
        let source = Rectangle::new(0.0, 0.0, self.width as f32, self.height as f32);
        d.draw_texture_pro(&*texture, source, dest, Vector2::zero(), 0.0, Color::WHITE);
    }
}

/// Color en [0, 1] a bytes RGBA
#[inline]
pub fn to_rgba(c: Vector3) -> [u8; 4] {
    [
        (c.x * 255.0).clamp(0.0, 255.0) as u8,
        (c.y * 255.0).clamp(0.0, 255.0) as u8,
        (c.z * 255.0).clamp(0.0, 255.0) as u8,
        255,
    ]
}
