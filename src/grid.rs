// grid.rs
// Rejilla de vóxeles: cada rayo visita solo las celdas que cruza en lugar de probar todos los bloques

use crate::block::Block;
use crate::ray_intersect::{Intersect, RayIntersect};
use raylib::prelude::Vector3;

/// Los bloques de la escena y la rejilla que los indexa
pub struct World {
    pub blocks: Vec<Block>,
    /// Celda mínima de la rejilla, en coordenadas enteras
    min: [i32; 3],
    dims: [i32; 3],
    /// Índice del bloque + 1 en cada celda; 0 si está vacía
    cells: Vec<u32>,
    /// Bloques que no ocupan exactamente una celda (como el sol, de tamaño 2)
    loose: Vec<usize>,
}

/// Un bloque entra a la rejilla si es un cubo unitario centrado en coordenadas enteras
fn grid_cell(block: &Block) -> Option<[i32; 3]> {
    let p = block.position;
    let aligned = block.size == 1.0 && [p.x, p.y, p.z].iter().all(|c| c.fract() == 0.0);
    aligned.then_some([p.x as i32, p.y as i32, p.z as i32])
}

/// Si el punto está dentro del bloque (o en su superficie)
fn contains(block: &Block, p: Vector3) -> bool {
    let half = block.size * 0.5;
    (p.x - block.position.x).abs() <= half
        && (p.y - block.position.y).abs() <= half
        && (p.z - block.position.z).abs() <= half
}

impl World {
    pub fn new(blocks: Vec<Block>) -> Self {
        let cells_of: Vec<_> = blocks.iter().map(grid_cell).collect();
        let mut min = [i32::MAX; 3];
        let mut max = [i32::MIN; 3];
        for c in cells_of.iter().flatten() {
            for a in 0..3 {
                min[a] = min[a].min(c[a]);
                max[a] = max[a].max(c[a]);
            }
        }
        let dims = if min[0] == i32::MAX {
            [0; 3]
        } else {
            [
                max[0] - min[0] + 1,
                max[1] - min[1] + 1,
                max[2] - min[2] + 1,
            ]
        };

        let mut world = World {
            cells: vec![0; (dims[0] * dims[1] * dims[2]) as usize],
            loose: Vec::new(),
            blocks,
            min,
            dims,
        };
        for (i, cell) in cells_of.iter().enumerate() {
            match cell {
                Some(c) => {
                    let index = world.index(*c).expect("la celda está dentro de la rejilla");
                    // Si dos bloques comparten celda, gana el primero, como en la búsqueda exhaustiva
                    if world.cells[index] == 0 {
                        world.cells[index] = i as u32 + 1;
                    }
                }
                None => world.loose.push(i),
            }
        }
        world
    }

    fn index(&self, c: [i32; 3]) -> Option<usize> {
        let l = [c[0] - self.min[0], c[1] - self.min[1], c[2] - self.min[2]];
        if (0..3).any(|a| l[a] < 0 || l[a] >= self.dims[a]) {
            return None;
        }
        Some(((l[2] * self.dims[1] + l[1]) * self.dims[0] + l[0]) as usize)
    }

    /// El impacto más cercano a menos de `max_distance`
    pub fn closest_hit(
        &self,
        origin: Vector3,
        dir: Vector3,
        max_distance: f32,
    ) -> Option<Intersect<'_>> {
        let mut best: Option<Intersect> = None;
        let mut limit = max_distance;
        for &i in &self.loose {
            let hit = self.blocks[i].ray_intersect(&origin, &dir);
            if hit.is_intersecting && hit.distance < limit {
                limit = hit.distance;
                best = Some(hit);
            }
        }
        self.traverse(origin, dir, limit).or(best)
    }

    /// Recorre las celdas en el orden en que el rayo las cruza (Amanatides y Woo).
    /// Como cada bloque llena su celda, el primero que pega es el más cercano.
    fn traverse(&self, origin: Vector3, dir: Vector3, limit: f32) -> Option<Intersect<'_>> {
        if self.cells.is_empty() {
            return None;
        }
        let o = [origin.x, origin.y, origin.z];
        let d = [dir.x, dir.y, dir.z];
        let lo: [f32; 3] = std::array::from_fn(|a| self.min[a] as f32 - 0.5);
        let hi: [f32; 3] = std::array::from_fn(|a| lo[a] + self.dims[a] as f32);

        // Tramo del rayo dentro de la caja de la rejilla
        let (mut t_enter, mut t_exit) = (0.0f32, limit);
        for a in 0..3 {
            if d[a].abs() < 1e-12 {
                if o[a] < lo[a] || o[a] > hi[a] {
                    return None;
                }
            } else {
                let (t1, t2) = ((lo[a] - o[a]) / d[a], (hi[a] - o[a]) / d[a]);
                t_enter = t_enter.max(t1.min(t2));
                t_exit = t_exit.min(t1.max(t2));
            }
        }
        if t_enter > t_exit {
            return None;
        }

        // Celda de entrada; el punto se toma un poco adentro para no caer justo en un borde
        let start = (t_enter + t_exit).min(t_enter + 1e-4).min(t_exit);
        let mut cell: [i32; 3] = std::array::from_fn(|a| {
            let c = (o[a] + d[a] * start + 0.5).floor() as i32;
            c.clamp(self.min[a], self.min[a] + self.dims[a] - 1)
        });
        let step: [i32; 3] = std::array::from_fn(|a| {
            if d[a] > 0.0 {
                1
            } else if d[a] < 0.0 {
                -1
            } else {
                0
            }
        });
        let t_delta: [f32; 3] = std::array::from_fn(|a| {
            if step[a] == 0 {
                f32::INFINITY
            } else {
                1.0 / d[a].abs()
            }
        });
        let mut t_next: [f32; 3] = std::array::from_fn(|a| {
            if step[a] == 0 {
                f32::INFINITY
            } else {
                let boundary = cell[a] as f32 + 0.5 * step[a] as f32;
                (boundary - o[a]) / d[a]
            }
        });

        loop {
            let slot = self.cells[self.index(cell)?];
            if slot != 0 {
                let hit = self.blocks[slot as usize - 1].ray_intersect(&origin, &dir);
                if hit.is_intersecting && hit.distance < limit {
                    let block = &self.blocks[slot as usize - 1];
                    // Si el rayo sale del bloque donde nació y al otro lado hay un bloque pegado,
                    // lo que se ve es la cara de ese bloque (por ejemplo, piedra vista desde dentro del vidrio)
                    if contains(block, origin) {
                        let axis = (0..3)
                            .min_by(|&a, &b| t_next[a].total_cmp(&t_next[b]))
                            .unwrap_or(0);
                        let mut beyond = cell;
                        beyond[axis] += step[axis];
                        if let Some(next) = self
                            .index(beyond)
                            .map(|i| self.cells[i])
                            .filter(|&n| n != 0)
                        {
                            let touching =
                                self.blocks[next as usize - 1].ray_intersect(&origin, &dir);
                            if touching.is_intersecting
                                && (touching.distance - hit.distance).abs() < 1e-3
                            {
                                return Some(touching);
                            }
                        }
                    }
                    return Some(hit);
                }
            }
            let axis = (0..3)
                .min_by(|&a, &b| t_next[a].total_cmp(&t_next[b]))
                .unwrap_or(0);
            if t_next[axis] > t_exit {
                return None;
            }
            cell[axis] += step[axis];
            t_next[axis] += t_delta[axis];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ray_intersect::RayIntersect;
    use crate::scene::create_optimized_scene;
    use raylib::prelude::Vector3;

    /// Búsqueda exhaustiva: el bloque más cercano probando todos
    fn brute_force(blocks: &[Block], origin: Vector3, dir: Vector3, max: f32) -> Option<f32> {
        blocks
            .iter()
            .map(|b| b.ray_intersect(&origin, &dir))
            .filter(|h| h.is_intersecting && h.distance < max)
            .map(|h| h.distance)
            .min_by(f32::total_cmp)
    }

    /// Números pseudoaleatorios reproducibles (xorshift)
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }
        fn range(&mut self, lo: f32, hi: f32) -> f32 {
            lo + (hi - lo) * self.next()
        }
    }

    #[test]
    fn the_grid_finds_the_same_hit_as_testing_every_block() {
        let world = World::new(create_optimized_scene());
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut hits = 0;
        for _ in 0..20_000 {
            let origin = Vector3::new(
                rng.range(-7.0, 7.0),
                rng.range(-3.0, 9.0),
                rng.range(-7.0, 7.0),
            );
            // Apunta a un punto al azar de la isla, con algo de desviación
            let target = Vector3::new(
                rng.range(-4.5, 4.5),
                rng.range(-1.0, 6.0),
                rng.range(-4.5, 5.0),
            );
            let jitter = Vector3::new(
                rng.range(-0.3, 0.3),
                rng.range(-0.3, 0.3),
                rng.range(-0.3, 0.3),
            );
            let dir = target - origin + jitter;
            if dir.length() < 1e-3 {
                continue;
            }
            let dir = dir.normalized();
            let expected = brute_force(&world.blocks, origin, dir, 50.0);
            let got = world.closest_hit(origin, dir, 50.0).map(|h| h.distance);
            match (expected, got) {
                (None, None) => {}
                (Some(e), Some(g)) => {
                    hits += 1;
                    assert!(
                        (e - g).abs() < 1e-4,
                        "origen {origin:?} dirección {dir:?}: esperado {e}, obtenido {g}"
                    );
                }
                _ => panic!(
                    "origen {origin:?} dirección {dir:?}: esperado {expected:?}, obtenido {got:?}"
                ),
            }
        }
        // La prueba no vale si casi ningún rayo pega en algo
        assert!(hits > 5_000, "solo {hits} impactos");
    }

    #[test]
    fn axis_aligned_rays_are_traversed_too() {
        let world = World::new(create_optimized_scene());
        // Desde arriba, recto hacia abajo, sobre el centro del césped
        let hit = world.closest_hit(
            Vector3::new(0.3, 20.0, 0.3),
            Vector3::new(0.0, -1.0, 0.0),
            50.0,
        );
        let expected = brute_force(
            &world.blocks,
            Vector3::new(0.3, 20.0, 0.3),
            Vector3::new(0.0, -1.0, 0.0),
            50.0,
        );
        assert_eq!(hit.map(|h| h.distance), expected);
        assert!(expected.is_some());
    }

    #[test]
    fn the_sun_outside_the_grid_is_still_hit() {
        let world = World::new(create_optimized_scene());
        // El sol es un bloque de tamaño 2 en (8, 10, -8)
        let origin = Vector3::new(8.0, 10.0, 0.0);
        let hit = world
            .closest_hit(origin, Vector3::new(0.0, 0.0, -1.0), 50.0)
            .expect("debe pegar en el sol");
        assert!((hit.distance - 7.0).abs() < 1e-4, "{}", hit.distance);
    }

    #[test]
    fn leaving_glass_into_a_touching_block_hits_that_block() {
        let world = World::new(create_optimized_scene());
        // Desde dentro del vidrio de la ventana sur, hacia abajo: debajo hay piedra pegada
        let hit = world
            .closest_hit(
                Vector3::new(-1.0, 2.0, 0.0),
                Vector3::new(0.0, -1.0, 0.0),
                50.0,
            )
            .expect("debe pegar");
        let material = hit.material.expect("con material");
        assert_eq!(
            material.transparency, 0.0,
            "pegó en el vidrio en vez de la piedra"
        );
        assert!((hit.distance - 0.5).abs() < 1e-4);
        assert!(
            hit.normal.y > 0.9,
            "debe ser la cara superior de la piedra: {:?}",
            hit.normal
        );
    }

    #[test]
    fn leaving_glass_into_air_still_hits_the_glass_exit() {
        let world = World::new(create_optimized_scene());
        // Desde dentro del mismo vidrio hacia dentro de la casa (aire)
        let hit = world
            .closest_hit(
                Vector3::new(-1.0, 2.0, 0.0),
                Vector3::new(0.0, 0.0, -1.0),
                50.0,
            )
            .expect("debe pegar");
        assert!((hit.distance - 0.5).abs() < 1e-4, "{}", hit.distance);
        assert!(hit.material.expect("con material").transparency > 0.0);
    }

    #[test]
    fn hits_beyond_the_maximum_distance_are_ignored() {
        let world = World::new(create_optimized_scene());
        let origin = Vector3::new(0.3, 20.0, 0.3);
        assert!(
            world
                .closest_hit(origin, Vector3::new(0.0, -1.0, 0.0), 5.0)
                .is_none()
        );
    }
}
