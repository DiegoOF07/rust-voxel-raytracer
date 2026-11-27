// snell.rs - Módulo de raytracing optimizado y reorganizado
use crate::grid::World;
use crate::light::Light;
use crate::ray_intersect::Intersect;
use crate::textures::TextureManager;
use raylib::prelude::*;

// === CONSTANTES ===
const MAX_DISTANCE: f32 = 50.0;
const EPSILON: f32 = 1e-4;
const MIN_REFLECTION_THRESHOLD: f32 = 0.05;
const MIN_SPECULAR_THRESHOLD: f32 = 5.0;
/// Luz ambiente blanca: suaviza el contraste en las caras que ninguna luz toca
const AMBIENT: f32 = 0.3;

// === FUNCIONES DE FÍSICA ÓPTICA ===

/// Calcula la reflexión de un rayo: R = I - 2(N·I)N
#[inline]
pub fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

/// Calcula la refracción usando la ley de Snell
pub fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Vector3 {
    let mut cosi = incident.dot(*normal).clamp(-1.0, 1.0);
    let mut etai = 1.0;
    let mut etat = refractive_index;
    let mut n = *normal;

    // Determinar si entramos o salimos del material
    if cosi > 0.0 {
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    } else {
        cosi = -cosi;
    }

    let eta = etai / etat;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);

    // Reflexión interna total si k < 0
    if k < 0.0 {
        Vector3::zero()
    } else {
        *incident * eta + n * (eta * cosi - k.sqrt())
    }
}

// === FUNCIONES DE SHADING ===

/// Calcula la contribución de una luz individual
fn calculate_light_contribution<'a>(
    intersect: &Intersect<'a>,
    light: &Light,
    base_color: &Vector3,
    view_dir: &Vector3,
) -> Vector3 {
    // Verificar que el material existe
    let material = match intersect.material {
        Some(mat) => mat,
        None => return Vector3::zero(), // Sin material, sin contribución
    };

    let light_dir = (light.position - intersect.point).normalized();
    let light_distance = (light.position - intersect.point).length();

    // Atenuación cuadrática por distancia
    let attenuation = 1.0 / (1.0 + 0.01 * light_distance * light_distance);

    // Componente difusa (Lambert)
    let n_dot_l = intersect.normal.dot(light_dir).max(0.0);
    let diffuse_intensity = n_dot_l * light.intensity * attenuation;

    let mut color = *base_color * light.color * diffuse_intensity * material.albedo[0];

    // Componente especular (Blinn-Phong) solo si es significativo
    if material.specular > MIN_SPECULAR_THRESHOLD && diffuse_intensity > 0.1 {
        let view_direction = (-*view_dir).normalized();
        let half_vector = (light_dir + view_direction).normalized();
        let n_dot_h = intersect.normal.dot(half_vector).max(0.0);
        let spec = n_dot_h.powf(material.specular);

        color += light.color * spec * material.albedo[1] * attenuation;
    }

    color
}

/// Obtiene el color base del material, aplicando texturas si existen
#[inline]
fn get_material_color<'a>(intersect: &Intersect<'a>, texture_manager: &TextureManager) -> Vector3 {
    // Verificar que el material existe
    let material = match intersect.material {
        Some(mat) => mat,
        None => return Vector3::one(), // Color blanco por defecto si no hay material
    };

    let mut base_color = material.diffuse;

    // Aplicar textura si existe
    if let Some(id) = material.texture_id {
        let texture_color = texture_manager.sample(id, intersect.u, intersect.v);
        base_color *= texture_color;
    }

    base_color
}

/// Color del cielo con gradiente basado en la dirección del rayo
#[inline]
fn sky_color(dir: &Vector3) -> Vector3 {
    let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0); // Mapear [-1,1] a [0,1]

    // Gradiente de horizonte (naranja) a cenit (azul)
    let horizon_color = Vector3::new(0.98, 0.92, 0.88); // Casi blanco con tono cálido
    let zenith_color = Vector3::new(0.2, 0.4, 0.8);

    horizon_color * (1.0 - t) + zenith_color * t
}

// === FUNCIONES PRINCIPALES DE RAYTRACING ===

/// Raytracer principal con múltiples luces, reflexiones y transparencia + fake glow
pub fn trace_ray_multi_light(
    origin: Vector3,
    dir: Vector3,
    depth: u32,
    max_depth: u32,
    world: &World,
    lights: &[Light],
    texture_manager: &TextureManager,
) -> Vector3 {
    if depth > max_depth {
        return sky_color(&dir);
    }

    let intersect = match world.closest_hit(origin, dir, MAX_DISTANCE) {
        Some(hit) => hit,
        None => return sky_color(&dir),
    };

    let material = match intersect.material {
        Some(mat) => mat,
        None => return sky_color(&dir),
    };

    let base_color = get_material_color(&intersect, texture_manager);

    // === iluminación directa ===
    let mut final_color = Vector3::zero();
    for light in lights {
        final_color += calculate_light_contribution(&intersect, light, &base_color, &dir);
    }
    if !lights.is_empty() {
        final_color /= lights.len() as f32;
    }

    // === Emisión basada en textura o diffuse ===
    if material.emission_strength > 0.0 {
        // Si el bloque tiene textura, úsala como "emission base"
        let emission_base = if material.texture.is_some() {
            base_color // viene de get_material_color(), ya con textura aplicada
        } else if let Some(emission) = &material.emission_color {
            *emission
        } else {
            material.diffuse
        };

        // Emisión directa
        final_color += emission_base * material.emission_strength;

        // --- Fake glow extra ---
        let glow_strength = material.emission_strength;
        let view_dir = -dir.normalized();
        let angle_factor = intersect.normal.dot(view_dir).clamp(0.0, 1.0).powf(2.0);
        let dist = (intersect.point - origin).length();
        let dist_factor = 1.0 / (1.0 + 0.15 * dist);

        final_color += emission_base * glow_strength * angle_factor * dist_factor * 2.0;
    }

    final_color += base_color * AMBIENT;

    // === reflexión y refracción ===
    let mut reflection_color = Vector3::zero();
    let mut refraction_color = Vector3::zero();
    let mut fresnel = 0.0;

    // Reflexión
    if material.reflectivity > MIN_REFLECTION_THRESHOLD && depth < max_depth {
        let reflected_dir = reflect(&dir, &intersect.normal).normalized();
        let reflect_origin = intersect.point + intersect.normal * EPSILON;
        reflection_color = trace_ray_multi_light(
            reflect_origin,
            reflected_dir,
            depth + 1,
            max_depth,
            world,
            lights,
            texture_manager,
        );
    }

    // Refracción
    if material.transparency > 0.01 && depth < max_depth {
        let refracted_dir = refract(&dir, &intersect.normal, material.refractive_index);
        if refracted_dir.dot(refracted_dir) > 1e-6 {
            let refract_origin = if dir.dot(intersect.normal) < 0.0 {
                intersect.point - intersect.normal * EPSILON
            } else {
                intersect.point + intersect.normal * EPSILON
            };
            refraction_color = trace_ray_multi_light(
                refract_origin,
                refracted_dir.normalized(),
                depth + 1,
                max_depth,
                world,
                lights,
                texture_manager,
            );

            // Fresnel (Schlick)
            let cos_i = (-dir.dot(intersect.normal)).abs().clamp(0.0, 1.0);
            fresnel = calculate_fresnel(cos_i, material.refractive_index);
        } else {
            fresnel = 1.0; // reflexión interna total
        }
    }

    // === combinación final ===
    if material.transparency > 0.01 && material.reflectivity > MIN_REFLECTION_THRESHOLD {
        // Caso 3: Material con transparencia + reflectividad (vidrio espejado)
        let direct = final_color * (1.0 - material.transparency) * (1.0 - material.reflectivity);
        let reflect = reflection_color * fresnel * material.reflectivity;
        let refract = refraction_color * material.transparency * (1.0 - fresnel);
        final_color = direct + reflect + refract;
    } else if material.transparency > 0.01 {
        // Caso 1: Solo transparente
        let direct = final_color * (1.0 - material.transparency);
        let reflect = reflection_color * fresnel;
        let refract = refraction_color * material.transparency * (1.0 - fresnel);
        final_color = direct + reflect + refract;
    } else if material.reflectivity > MIN_REFLECTION_THRESHOLD {
        // Caso 2: Solo reflectivo
        final_color =
            final_color * (1.0 - material.reflectivity) + reflection_color * material.reflectivity;
    }

    Vector3::new(
        final_color.x.clamp(0.0, 1.0),
        final_color.y.clamp(0.0, 1.0),
        final_color.z.clamp(0.0, 1.0),
    )
}

/// Calcula el coeficiente de reflexión de Fresnel
fn calculate_fresnel(cos_i: f32, refractive_index: f32) -> f32 {
    let n1 = 1.0;
    let n2 = refractive_index;
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos_i).powi(5) // Schlick
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn reflection_mirrors_the_ray_around_the_normal() {
        let incident = Vector3::new(1.0, -1.0, 0.0).normalized();
        let normal = Vector3::new(0.0, 1.0, 0.0);
        assert!(close(
            reflect(&incident, &normal),
            Vector3::new(1.0, 1.0, 0.0).normalized()
        ));
    }

    #[test]
    fn a_ray_hitting_glass_head_on_goes_straight_through() {
        let down = Vector3::new(0.0, -1.0, 0.0);
        let refracted = refract(&down, &Vector3::new(0.0, 1.0, 0.0), 1.5);
        assert!(close(refracted.normalized(), down), "{refracted:?}");
    }

    #[test]
    fn refraction_follows_snells_law() {
        // Entra del aire (n = 1) al vidrio (n = 1.5) a 45 grados
        let incident = Vector3::new(1.0, -1.0, 0.0).normalized();
        let refracted = refract(&incident, &Vector3::new(0.0, 1.0, 0.0), 1.5).normalized();
        let sin_in = incident.x.abs();
        let sin_out = refracted.x.abs();
        assert!(
            (sin_in - 1.5 * sin_out).abs() < 1e-4,
            "sen θ1 = {sin_in}, sen θ2 = {sin_out}"
        );
        assert!(refracted.y < 0.0, "debe seguir hacia dentro del vidrio");
    }

    #[test]
    fn a_grazing_ray_leaving_glass_is_totally_reflected() {
        // Sale del vidrio casi rasante: no hay refracción posible
        let incident = Vector3::new(0.95, 0.312, 0.0).normalized();
        let outward_normal = Vector3::new(0.0, 1.0, 0.0);
        assert_eq!(refract(&incident, &outward_normal, 1.5), Vector3::zero());
    }

    #[test]
    fn fresnel_grows_toward_grazing_angles() {
        let head_on = calculate_fresnel(1.0, 1.5);
        let grazing = calculate_fresnel(0.05, 1.5);
        assert!((head_on - 0.04).abs() < 1e-3, "{head_on}");
        assert!(grazing > 0.7 && grazing <= 1.0, "{grazing}");
    }
}
