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
cargo run --release
```

Para ejecutar una comparación reproducible de 240 frames sin abrir una ventana:

```bash
cargo run --release -- --benchmark
cargo run --release -- --benchmark --objects 30
cargo run --release -- --benchmark --objects 300
```

Cada 120 frames se imprime en la terminal un reporte con tiempo promedio y p95 del frame, tiempos por etapa, objetos visibles y descartados, superficies actualizadas, triángulos totales y descartados, y fragmentos cubiertos, rechazados por profundidad y sombreados. Estas métricas excluyen la pausa artificial de 16 ms entre frames.

Los objetos del mundo se renderizan de adelante hacia atrás según la distancia a su superficie para maximizar los descartes tempranos del depth buffer.

Los triángulos que atraviesan el plano cercano de la cámara se recortan conservando sus colores, UV y materiales.

Los cambios de LOD usan umbrales distintos de entrada y salida para evitar oscilaciones cuando un cuerpo permanece cerca de un límite.
