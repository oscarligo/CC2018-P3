# CC2018 - P3

## Ejecución

La nave recorre el espacio siguiendo su orientación y la cámara mantiene su centro a la vista, incluso durante giros completos o vuelo invertido. Los objetos conservan sus posiciones en el mundo: al acercarte aumentan de tamaño y al pasar a su lado se desplazan en pantalla.

- `S` / `W`: elevar / bajar el morro respecto a la nave.
- `A` / `D`: girar a izquierda / derecha respecto a la nave.
- `Q` / `E`: inclinarse y acompañar el giro.
- `Espacio`: acelerar; al soltarlo se pierde velocidad gradualmente.
- `Shift izquierdo`: frenar.
- Clic derecho + movimiento del ratón: orbitar la cámara alrededor de la nave.
- Rueda del ratón: zoom. Mover el cursor sin botones no dirige la nave.

```bash
# Relleno baricéntrico, modo predeterminado
cargo run --release

# Relleno por scanlines
cargo run --release -- --scanline

# Solo contorno
cargo run --release -- --wireframe

# Comparación de rendimiento sin abrir la ventana
cargo run --release -- --benchmark-triangles
