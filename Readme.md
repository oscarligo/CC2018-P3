# CC2018 - P3

## Estructura de `src`

- `celestial_shaders/`: apariencia procedural de estrellas y planetas.
- `objects/`: cuerpos y elementos renderizables del sistema solar.
- `rendering/`: framebuffer, geometría, proyección, shaders y rasterización.
- `scene/`: mundo, catálogo de planetas, sistema solar y fondo estelar.
- `ship/`: controles, movimiento, cámara, efectos y audio de la nave.
- `main.rs`: ciclo de ejecución y benchmark.
- `noise.rs` y `performance.rs`: utilidades transversales.

## Ejecución

La nave recorre el espacio siguiendo su orientación y la cámara mantiene su centro a la vista, incluso durante giros completos o vuelo invertido. Los objetos conservan sus posiciones en el mundo: al acercarte aumentan de tamaño y al pasar a su lado se desplazan en pantalla.

- `S` / `W`: elevar / bajar el morro respecto a la nave.
- `A` / `D`: girar a izquierda / derecha respecto a la nave.
- `Q` / `E`: inclinarse y acompañar el giro.
- `F`: interactuar con el cuerpo apuntado desde el centro de la cámara.
- `Espacio`: acelerar; al soltarlo se pierde velocidad gradualmente.
- `Shift izquierdo`: frenar.
- Clic derecho + movimiento del ratón: orbitar la cámara alrededor de la nave.
- Rueda del ratón: zoom. Mover el cursor sin botones no dirige la nave.

```bash
cargo run --release
```

Para ejecutar una comparación reproducible de 240 frames sin abrir una ventana:

```bash
cargo run --release -- --benchmark
cargo run --release -- --benchmark --objects 30
cargo run --release -- --benchmark --objects 300
```
