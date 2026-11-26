// textures.rs
// Texturas en memoria de CPU, identificadas por índice y muestreadas con interpolación bilineal

use raylib::prelude::*;
use std::collections::HashMap;

struct CpuTexture {
    width: i32,
    height: i32,
    /// Colores normalizados a [0, 1]
    pixels: Vec<Vector3>,
}

impl CpuTexture {
    fn from_image(image: &Image) -> Self {
        let pixels = image
            .get_image_data()
            .iter()
            .map(|c| Vector3::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0))
            .collect();
        Self {
            width: image.width,
            height: image.height,
            pixels,
        }
    }

    fn sample_bilinear(&self, u: f32, v: f32) -> Vector3 {
        let x = u.clamp(0.0, 1.0) * (self.width - 1) as f32;
        let y = v.clamp(0.0, 1.0) * (self.height - 1) as f32;
        let (x0, y0) = (x.floor() as i32, y.floor() as i32);
        let (x1, y1) = ((x0 + 1).min(self.width - 1), (y0 + 1).min(self.height - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);

        let c00 = self.pixel(x0, y0);
        let c10 = self.pixel(x1, y0);
        let c01 = self.pixel(x0, y1);
        let c11 = self.pixel(x1, y1);
        let c0 = c00 + (c10 - c00) * fx;
        let c1 = c01 + (c11 - c01) * fx;
        c0 + (c1 - c0) * fy
    }

    fn pixel(&self, x: i32, y: i32) -> Vector3 {
        let x = x.clamp(0, self.width - 1);
        let y = y.clamp(0, self.height - 1);
        self.pixels
            .get((y * self.width + x) as usize)
            .copied()
            .unwrap_or(Vector3::one())
    }
}

#[derive(Default)]
pub struct TextureManager {
    textures: Vec<CpuTexture>,
    ids: HashMap<String, usize>,
}

impl TextureManager {
    /// Carga la imagen una sola vez y devuelve su índice. No necesita ventana abierta.
    pub fn load(&mut self, path: &str) -> Result<usize, String> {
        if let Some(&id) = self.ids.get(path) {
            return Ok(id);
        }
        let image =
            Image::load_image(path).map_err(|_| format!("No se pudo cargar la imagen: {path}"))?;
        self.textures.push(CpuTexture::from_image(&image));
        let id = self.textures.len() - 1;
        self.ids.insert(path.to_string(), id);
        Ok(id)
    }

    /// Color en (u, v); blanco si el índice no existe
    pub fn sample(&self, id: usize, u: f32, v: f32) -> Vector3 {
        self.textures
            .get(id)
            .map_or(Vector3::one(), |t| t.sample_bilinear(u, v))
    }
}
