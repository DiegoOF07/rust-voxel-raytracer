// material.rs
use raylib::prelude::Vector3;

/// Define las propiedades físicas y visuales de un material.
/// Se usa para calcular cómo interactúa la luz con la superficie.
#[derive(Debug, Clone)]
pub struct Material {
    /// Color base difuso (en espacio RGB normalizado 0.0–1.0).
    pub diffuse: Vector3,

    /// Coeficientes de mezcla entre el color propio y la luz:
    /// [albedo_difuso, albedo_especular]
    pub albedo: [f32; 2],

    /// Intensidad de brillo especular (mayor = más brillante).
    pub specular: f32,

    /// Reflectividad de la superficie:
    /// 0.0 = no refleja, 1.0 = espejo perfecto.
    pub reflectivity: f32,

    /// Transparencia de la superficie:
    /// 0.0 = opaco, 1.0 = perfectamente transparente.
    pub transparency: f32,

    /// Índice de refracción (ej: aire ≈ 1.0, agua ≈ 1.33, vidrio ≈ 1.5).
    pub refractive_index: f32,

    /// Ruta opcional a la textura difusa.
    pub texture: Option<&'static str>,

    /// Índice de la textura ya cargada; se asigna al cargar la escena.
    pub texture_id: Option<usize>,

    /// Ruta opcional para el color del halo de luz
    pub emission_color: Option<Vector3>,

    /// Intensidad de la luz emitida
    pub emission_strength: f32,
}
