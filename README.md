# Raytracer de vóxeles en Rust

> Raytracer en CPU de una isla flotante de bloques, con reflexión, refracción, texturas y bloques que emiten luz

[![CI](https://github.com/DiegoOF07/rust-voxel-raytracer/actions/workflows/ci.yml/badge.svg)](https://github.com/DiegoOF07/rust-voxel-raytracer/actions/workflows/ci.yml)

[![Video del raytracer en ejecución](https://img.youtube.com/vi/k_zkKDIqTmM/maxresdefault.jpg)](https://youtu.be/k_zkKDIqTmM)

*Clic en la imagen para ver el video.*

Cada píxel lanza un rayo desde la cámara y lo sigue por la escena: dónde choca, cómo se ilumina ese punto y, si el material lo pide, a dónde rebota o cómo atraviesa. Todo se calcula en Rust y en CPU; Raylib solo abre la ventana y muestra la imagen.

## Qué hace

- **Una isla de 117 bloques** con una casa de piedra con ventanas de vidrio, un árbol de cerezo, un lago y un sol.
- **Materiales con su propia óptica:**
  - el vidrio refracta según la ley de Snell y mezcla reflexión y refracción con la aproximación de Schlick (Fresnel);
  - el agua del lago es un espejo;
  - el magma y el sol emiten luz e iluminan la escena;
  - el resto (césped, piedra, troncos, hojas, arena) se ilumina con Lambert y Blinn-Phong, con tres luces blancas y luz ambiente.
- **Texturas** con interpolación bilineal.
- **Cámara libre** y un modo de un solo hilo para comparar contra todos los núcleos.

## Stack

Rust (edición 2024) · Raylib 5.5 (ventana y presentación) · Rayon (paralelismo)

## Cómo funciona

```
grid.rs                     rejilla de vóxeles y recorrido de rayos por celdas
block.rs, ray_intersect.rs  intersección rayo-cubo (método de planos)
snell.rs                    trazado, iluminación, reflexión, refracción y Fresnel
textures.rs                 texturas en memoria, muestreo bilineal
scene.rs, block_types.rs    la isla y los materiales de cada bloque
main.rs                     cámara, controles y modo de medición
```

La escena está en una **rejilla de vóxeles**. En lugar de probar cada rayo contra los 117 bloques, el rayo avanza celda por celda en el orden en que las cruza (el algoritmo de Amanatides y Woo), y el primer bloque que encuentra es el más cercano. Un rayo típico visita unas pocas celdas.

Las filas de la imagen se reparten entre los núcleos con Rayon.

## Cómo correrlo

Requiere Rust estable y las dependencias de compilación de Raylib (CMake y un compilador de C). Se corre desde la raíz del repo, porque las texturas se cargan de `textures/`.

```bash
cargo run --release
```

| Tecla | Acción |
|---|---|
| W A S D | moverse |
| Flechas | girar la cámara |
| Espacio / Ctrl | subir o bajar |
| T | alternar entre un hilo y todos los núcleos |
| Esc | salir |

## Rendimiento

`cargo run --release -- --bench` renderiza varias vistas a 400×300, sin abrir ventana, y reporta el tiempo por cuadro. Con todos los núcleos de un Intel i5-10400 (12 hilos):

| Vista | Antes | Ahora |
|---|---|---|
| Isla completa | 27–34 ms | 7–9 ms |
| Casa de cerca | 30–32 ms | 11 ms |
| Lago | 29–35 ms | 12–13 ms |
| Ventana de vidrio | 57–61 ms | 20–25 ms |

Antes cada rayo se probaba contra todos los bloques, las texturas se buscaban por nombre en cada muestra y se creaban hilos nuevos en cada cuadro. Las vistas con vidrio siguen siendo las más caras, porque cada rebote vuelve a refractar y cada impacto suma la luz de 10 fuentes.

## Decisiones y aprendizajes

- **Una rejilla en vez de una lista.** Como todos los bloques ocupan una celda entera, recorrer la rejilla da el mismo impacto que probar todos los bloques, y una prueba lo compara en 20 000 rayos al azar. El sol, que mide dos bloques, se prueba aparte.
- **Caras compartidas.** Cuando un rayo sale del vidrio justo donde empieza un bloque de piedra, lo que se ve es la cara de la piedra. Antes eso dependía del orden en que se crearon los bloques y una pared de la casa se veía oscura; ahora es una regla explícita.
- **Luz blanca y ambiente suave.** Las luces de color y un ambiente de 8 % dejaban casi negras las caras que no daban a ninguna luz. Con luces blancas, una frontal que ilumina lo que ve la cámara y 30 % de ambiente, el contraste se suaviza sin perder la diferencia entre la cara superior y los lados de cada bloque.
- **Texturas solo en CPU.** El raytracer nunca usa la GPU para muestrear, así que las texturas se cargan como arreglos de colores y se identifican por índice.

Proyecto 2 del curso Gráficas por Computadora, UVG, 2025.
