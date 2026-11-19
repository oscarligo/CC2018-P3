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

Al iniciar aparece un hangar con vista previa 3D. Elige con `1` / `2`, flechas o
clic y pulsa `Enter` o **Despegar**:

- **Carro original:** conserva su modelo, controles y propulsión.
- **Ovni / gato alien:** platillo metálico con luces verdes, cúpula transparente
  y piloto felino con antenas. Su aceleración y velocidad máxima son tres veces
  las del carro; sus partículas y estelas son verdes.

El ovni está modelado mediante geometría propia en `src/ship/vehicle.rs`:
casco, aros, paneles, tornillos, consola, gato y cúpula se construyen en CPU.
La cúpula se dibuja después del piloto con transparencia y reflejos según el
ángulo de visión. Todo utiliza el rasterizador por software del proyecto.

### Teletransporte del ovni

Durante el vuelo, escribe la secuencia `3.14` (sin Enter, en menos de cinco
segundos entre caracteres). También acepta la coma decimal del teclado numérico.
Se abre un menú con los 18 destinos principales, incluidos Kashyyyk y Coruscant.
Usa flechas, rueda o clic para seleccionar y **Enter / Teletransportar** para saltar;
`Esc` cancela. Las flechas y la rueda recorren las páginas del menú.

La simulación orbital continúa mientras consultas el menú; el vuelo se suspende.
El salto utiliza la posición actual del destino, busca espacio libre fuera del
planeta y sus anillos, detiene el ovni, lo orienta hacia el planeta y reinicia
la cámara y las partículas. Las lunas y el sol principal no son destinos.
En vuelo, `Esc` cierra el programa.

### Travesuras del ovni

Escribe los comandos durante el vuelo, sin Enter. No distinguen mayúsculas y
aceptan hasta cinco segundos entre caracteres. Las interfaces del hangar,
teletransporte y modos usan azul; la energía del ovni sigue siendo verde.

- **`achu`:** carga de 1,05 segundos con temblor y compresión del platillo,
  seguida de 3,8 segundos de vuelo caótico. La velocidad llega aproximadamente
  a 25 veces la velocidad base del ovni, con nube verde expansiva, destello,
  estela luminosa y silbidos que recorren ambos canales de los audífonos.
  Al terminar recuperas el control y la velocidad vuelve al rango normal.
- **`mishmish`** (todo junto): activa o desactiva el modo de puntería. Aparecen
  dianas de una en una, orbitando al gato con una estela verde. Su movimiento
  depende del tiempo y del piloto, sin seguir al ratón. Tú diriges el vuelo.
  Mueve la mira azul con el ratón y pulsa **clic izquierdo** para disparar un
  pulso verde desde el borde del ovni. Los planetas bloquean los disparos.
  Solo acertar con el láser destruye la diana y suma un punto; después de una
  breve pausa aparece la siguiente. Pasar junto a ella no la recoge.
- **Espacio:** acelera el vehículo.
- **R:** hace girar al ovni. Mantenerla pulsada sostiene la pirueta;
  al soltarla se amortigua suavemente, sin girar la cámara ni dirigir el vuelo.
  El clic izquierdo queda dedicado al disparo.
  El carro conserva **Espacio** para acelerar.
- **Shift izquierdo:** cancela el modo especial y frena para recuperar el control.
- Los modos son exclusivos. El teletransporte reinicia modos y efectos; el menú
  suspende el vuelo y el audio especial mientras continúan las órbitas.

### Radio del carro

Coloca los archivos `.mp3` en `assets/radio/` (también acepta `.MP3`).
Con el carro seleccionado, escribe **`fm`**, sin Enter, para encender la radio
retro de la esquina inferior derecha. Repetir `fm` la oculta y suspende el audio;
al encenderla de nuevo conserva la posición y el estado de pausa.

Solo hay tres botones: **Anterior**, **Pausar/Reanudar** y **Siguiente**.
La radio elige al azar evitando repetir inmediatamente la misma canción cuando
hay otras disponibles. Cada elección nueva comienza en un punto aleatorio,
dejando al menos el último cuarto o 30 segundos por escuchar, lo que sea menor.
Al terminar una canción pasa automáticamente a otra selección aleatoria.

Anterior recorre el historial con los puntos de inicio que se eligieron para
esas canciones; Siguiente permite volver a recorrer ese historial antes de hacer
otra selección. Cambiar de canción mientras está pausada mantiene la pausa.
La carpeta se vuelve a revisar al encender la radio. Si está vacía o un archivo
no se puede reproducir, el juego continúa y muestra el estado correspondiente.

Mientras suena la radio, los sonidos del carro (motor, aceleración, freno,
vuelo e impactos) bajan suavemente al 20 % de su volumen habitual. Al pausar
o apagar la radio recuperan su volumen progresivamente.

La lectura, el salto al punto aleatorio y la reproducción se ejecutan en un
hilo de audio independiente, para que cambiar de canción no detenga el dibujo
ni los controles. El panel muestra «SINTONIZANDO...» durante la carga.
Se conserva el MP3 comprimido actual y se decodifica por bloques; al cambiar
pueden coexistir brevemente el archivo anterior y el siguiente.
Esto permite admitir nombres con tildes en Windows sin renombrar las canciones.
La radio no está disponible en el ovni. Las inspecciones con `F` se colocan
encima del reproductor para mantener legibles ambos paneles.

Para generar una captura de la interfaz real, con ventana oculta y sin sonido:

```bash
cargo run --release -- --preview-radio
```

La captura se guarda en `target/previews/radio.png`.

Para medir lectura, decodificador y salto de cada MP3, además del trabajo
del hilo principal durante cambios asíncronos (sin sonido):

```bash
cargo run --release -- --benchmark-radio
```

### Estructura Dyson


El sol está rodeado por exactamente dos aros gruesos de sección redondeada.
Giran en sentidos opuestos: las celdas completan una vuelta en unos 2,6 segundos
y sus planos giran alrededor del centro solar en unos 8 segundos. Ambos ejes
pasan por el centro del sol, sin balanceos ni desplazamientos del centro.
Las caras interiores son espejos plateados; el exterior tiene paneles opacos,
oscuros y mates, sin emisión ni halo luminoso, para contrastar con los reflejos
internos. Los bloques de paneles permiten distinguir la rotación. Todo se calcula
en CPU.

Los aros dejan visible la mayor parte del disco solar. El radio
exterior se limita según la distancia al planeta más cercano y sus dimensiones,
dejando margen respecto a su órbita. No se modifican órbitas ni velocidades de
planetas. Los aros no añaden un colisionador esférico que bloquee el espacio vacío.

Para generar vistas de ambos aros en distintos momentos y desde el interior:

```bash
cargo run --release -- --preview-dyson
```

Las capturas quedan en `target/previews/dyson-*.png`.

### Choques

Ambos vehículos chocan con los cuerpos celestes, incluidas lunas y estrellas.
Se comprueba todo el segmento recorrido por el volumen del vehículo en cada
frame para impedir atravesar un planeta incluso durante `achu`. El impacto
produce una explosión luminosa, sonido grave, una breve sacudida de cámara y
un impulso de rebote que se amortigua. Un choque cancela el estornudo.
Las colisiones usan esferas conservadoras alrededor de los cuerpos; no son
colisiones por triángulo con edificios, árboles o anillos.

Los efectos son originales y se sintetizan al iniciar: sonido estéreo con cambios
de intensidad entre oídos, pequeños retardos, filtrado y ecos para simular movimiento
alrededor de la cabeza. Es una aproximación espacial para audífonos, sin HRTF
personalizada ni dependencia de un sistema de audio “4D”.

Para generar capturas de los modos y escuchar los efectos WAV por separado:

```bash
cargo run --release -- --preview-alien
```

Las capturas y muestras de audio quedan en `target/previews/`.

La nave recorre el espacio siguiendo su orientación y la cámara mantiene su centro a la vista, incluso durante giros completos o vuelo invertido. Los objetos conservan sus posiciones en el mundo: al acercarte aumentan de tamaño y al pasar a su lado se desplazan en pantalla.

- `S` / `W`: elevar / bajar el morro respecto a la nave.
- `A` / `D`: girar a izquierda / derecha respecto a la nave.
- `Q` / `E`: inclinarse y acompañar el giro.
- `F`: interactuar con el cuerpo apuntado desde el centro de la cámara.
- `Espacio`: acelerar.
- `R` (ovni): girar sobre su eje, de forma independiente a la aceleración.
- Clic izquierdo en `mishmish`: un disparo por pulsación.
- `Shift izquierdo`: frenar.
- Clic derecho + movimiento del ratón: orbitar la cámara alrededor de la nave.
- Rueda del ratón: zoom. Mover el cursor sin botones no dirige la nave.

Al inspeccionar con `F`, los 32 cuerpos celestes muestran una ficha propia:
nombre, sobrenombre y una descripción ligada a su aspecto, sus lunas o sus
construcciones. Incluye el sol, los planetas y todas las lunas. El panel ajusta
las líneas al ancho de la ventana y permanece visible durante ocho segundos.
La barra del ovni recuerda `achu` y `mishmish`; los aciertos aparecen únicamente
mientras está activo el modo de puntería.

```bash
cargo run --release
```

En Windows se necesitan Rust, las herramientas C++ de Visual Studio, CMake y
LLVM/libclang. Si no están disponibles en la terminal actual:

```powershell
$env:Path = "C:\Program Files\CMake\bin;$env:Path"
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
cargo run --release
```

Cierra cualquier ejecución anterior antes de recompilar: Windows bloquea el
archivo `.exe` mientras está abierto.

Para generar capturas de ambos vehículos con el renderer CPU sin abrir ventanas:

```bash
cargo run --release -- --preview-ships
```

Las imágenes se guardan en `target/previews/` (excluidas del repositorio).

Para ejecutar una comparación reproducible de 240 frames sin abrir una ventana:

```bash
cargo run --release -- --benchmark
cargo run --release -- --benchmark --objects 30
cargo run --release -- --benchmark --objects 300
```
