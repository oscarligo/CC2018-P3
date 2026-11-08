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

Cada 120 frames se imprime en la terminal un reporte con tiempo promedio y p95 del frame, tiempos por etapa, objetos visibles y descartados, superficies actualizadas, triángulos totales y descartados, y fragmentos cubiertos, rechazados por profundidad y sombreados. Estas métricas excluyen la pausa artificial de 16 ms entre frames.

Los objetos del mundo se renderizan de adelante hacia atrás según la distancia a su superficie para maximizar los descartes tempranos del depth buffer.

Los triángulos que atraviesan el plano cercano de la cámara se recortan conservando sus colores, UV y materiales.

Los cambios de LOD usan umbrales distintos de entrada y salida para evitar oscilaciones cuando un cuerpo permanece cerca de un límite. Al cambiar de nivel, la geometría se deforma durante 0.2 segundos entre ambas superficies; solo se procesa una malla y no se agrega un pase de mezcla.

Los planetas reciben iluminación desde la posición real de la estrella primaria registrada en el mundo.

La nave usa esa misma fuente para modular su textura con iluminación ambiental y difusa según sus normales.

Cada cuerpo puede configurar semilla procedural, tinte, cobertura de nubes y velocidades de animación y rotación mediante `CelestialStyle`.

El fondo usa un catálogo estelar determinista y persistente con variaciones de brillo y temperatura de color.

El mundo separa objetos opacos y transparentes: los opacos se dibujan de adelante hacia atrás y los transparentes de atrás hacia adelante sin escribir profundidad.

El gigante gaseoso utiliza una malla anular transparente, compartida y de doble cara, con bandas y desvanecimiento radial.

Los planetas usan órbitas inclinadas. Sus atmósferas, anillos y futuros elementos se conectan mediante anclas padre-hijo; posición, rotación y escala locales se componen en jerarquía, por lo que siguen al cuerpo sin duplicar parámetros orbitales.

Los cuerpos sólidos tienen identificadores estables y colisionadores esféricos. El mundo ofrece raycast al impacto más cercano y consultas de proximidad sin asignaciones obligatorias por frame. Atmósferas y anillos permanecen fuera de las colisiones.

El sistema de interacción selecciona el primer objeto sólido bajo el centro de la cámara, muestra su acción disponible y presenta una respuesta temporal al activarlo.

El planeta rocoso incluye formaciones de baja geometría distribuidas sobre su superficie. Comparten una sola malla, heredan la transformación planetaria, reciben iluminación solar y pueden examinarse individualmente.

Su cuerpo usa una deformación estática determinista con elevaciones y depresiones. Cada LOD mantiene una versión equivalente de la forma, con normales recalculadas y transición geométrica entre niveles.

El gigante gaseoso usa un perfil achatado, con ecuador más ancho y polos comprimidos. Su atmósfera reutiliza las mismas mallas de forma para mantener un grosor visual coherente.
