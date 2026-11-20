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

Al iniciar aparece un hangar con vista previa 3D. Elige con `1` / `2` / `3`, flechas o
clic y pulsa `Enter` o **Despegar**:

- **Carro original:** conserva su modelo, controles y propulsión.
- **Ovni / gato alien:** platillo metálico con luces verdes, cúpula transparente
  y piloto felino con antenas. Su aceleración y velocidad máxima son tres veces
  las del carro; sus partículas y estelas son verdes.
- **Destructor Estelar:** nave original inspirada en un Destructor Estelar, con
  casco triangular, ciudadela escalonada, puente, cúpulas, torretas, motores azules
  y una bahía ventral animada para la bomba planetaria.

Durante el juego, pulsa **F1** o haz clic en **Cambiar nave**, en la esquina superior
derecha, para regresar al hangar. La selección conserva la última nave elegida.
Al despegar de nuevo, el vehículo comienza desde la posición inicial y los planetas
conservan su estado; el mundo se pausa mientras estás en el hangar. Volver al hangar
detiene la radio y el audio, y cancela el sobrevuelo, los modos especiales y cualquier
secuencia de la bomba en curso. **Esc** sigue cerrando el juego fuera de los menús.

### Destructor Estelar y bomba planetaria

Selecciona **3** en el hangar. Conserva los controles de vuelo y el vuelo local
con `L`; también permite viajar escribiendo `3.14`, igual que el ovni.
Apunta la mira de la proa hacia un planeta cercano: aparecerán su nombre y
unas esquinas rojas alrededor del objetivo. El sol no es un objetivo válido.

1. **B:** fija el planeta y abre la bahía. La nave acompaña su órbita durante
   la operación; las órbitas del sistema continúan y no se cambia de escena.
2. **J, pulsar y soltar rápidamente:** cada pulsación aporta hasta 6,5 puntos
   porcentuales a la carga. Mantener la tecla o la repetición del teclado no
   sirve. El reactor pierde entre 17 y 42 puntos por segundo, según su carga:
   la parte final exige sostener aproximadamente siete pulsaciones por segundo.
   La estabilización exige **al menos 15 segundos pulsando**, incluso a un ritmo
   mayor; esperar sin pulsar no adelanta ese tiempo. Si el reactor se vacía,
   la estabilización vuelve a comenzar. A seis pulsaciones por segundo no se
   consigue terminar. A 60 FPS, siete pulsaciones por segundo completan la carga
   en unos 15,2 segundos. El panel muestra el porcentaje, las pulsaciones del
   último segundo y los segundos de estabilización pendientes.
3. El tono del reactor sube y baja con la carga real. Se intensifican la luz,
   los conductos y una vibración leve. Los sonidos se sintetizan en CPU.
4. **La pregunta aparece al llegar al 100 %**, después de estabilizar el reactor.
   La carga queda asegurada y se abre el diálogo
   **«¿Estás seguro de que quieres exterminar un planeta completo?»** con
   **«Sí, lanzar»** y **«Ay noooo»**. La cancelación está seleccionada inicialmente.
   Usa **flechas** y después **Enter**; `J` nunca confirma el lanzamiento.
5. **B / Esc** cancelan la carga o la confirmación. Perder el foco de la ventana
   también cancela esas fases. Una vez autorizado el lanzamiento, la bomba
   persigue la posición actual del planeta, aparecen grietas luminosas y una
   onda expansiva, y el planeta se convierte en escombros animados.

La destrucción retira también los objetos unidos al planeta, incluidos modelos
de superficie, atmósfera, anillos y satélites, además de sus colisiones y destinos
de viaje. Al terminar la secuencia, **F8** restaura todos los planetas destruidos
y sus objetos, conservando sus identificadores, sin reiniciar el programa.
Los demás vehículos conservan sus funciones. `Esc` fuera de un menú o de la
secuencia del arma cierra el juego.

El casco, las compuertas, la bomba, las grietas y los fragmentos se transforman
y dibujan en el pipeline de software. La nave se carga desde
`assets/star_destroyer.obj`; las piezas móviles usan `star_destroyer_door.obj`
y `star_destroyer_bomb.obj`, con materiales compartidos en `star_destroyer.mtl`.
Son recursos OBJ editables; el cargador propio conserva los colores `Kd` y las
luces `Ke` del MTL, y las piezas se animan mediante transformaciones independientes.
El modelo y los sonidos son originales,
creados para este proyecto con asistencia de Codex/IA; no se descargaron modelos
ni audio de Star Wars.

Para generar capturas del hangar, la bahía, la carga, el diálogo, el lanzamiento,
la destrucción y la cancelación, además de los WAV, sin reproducir audio:

```bash
cargo run --release -- --preview-doomsday
```

La verificación usa una ventana oculta y controles simulados sobre la lógica real;
también comprueba la restauración. Los archivos quedan en `target/previews/destroyer-*`.
Las constantes de dificultad están en `src/ship/doomsday.rs`.

El ovni está modelado mediante geometría propia en `src/ship/vehicle.rs`:
casco, aros, paneles, tornillos, consola, gato y cúpula se construyen en CPU.
La cúpula se dibuja después del piloto con transparencia y reflejos según el
ángulo de visión. Todo utiliza el rasterizador por software del proyecto.

### Persona 3: planetas y navegación cercana

Los dos mundos de esta contribución son **Ureski** y **Alfa Riddertal**.
Sustituyen las dos propuestas provisionales, sin duplicar cuerpos del catálogo.
Sus módulos están en
`src/scene/planets/exploration/` y `exploration.rs`:

- **Ureski:** desierto dorado permanente, tundra azul nocturna y un cinturón
  verde de primavera eterna. Su cara local +X permanece apuntando al sol
  mediante rotación sincronizada: orbita, pero no tiene un ciclo de día y noche.
  El crepúsculo es una región fija de su geografía. Allí crecen árboles de copas
  escalonadas y helechos con flores luminosas; saltarines verdes de antenas
  doradas y mantas violetas recorren la vegetación. Los saltos, la compresión
  del cuerpo, el aleteo, el paseo y el balanceo vegetal se animan con el tiempo.
- **Alfa Riddertal:** gigante de gas rosado con corrientes atmosféricas y un
  núcleo que aparece en tres estados superpuestos. Sus filamentos y ojos
  cambian con el tiempo y la dirección de observación. Es una representación
  visual de una entidad que custodia el pasado humano, no una simulación
  cuántica ni una conexión a un archivo real. Seis estaciones originales
  orbitan en planos y sentidos distintos. Alternan consultas cian hacia el
  planeta y respuestas rosadas hacia las estaciones, con paquetes luminosos
  en movimiento, pausas de comunicación y destellos en los puntos de enlace.

La flora, las dos especies animales y las estaciones son geometría original
construida en CPU, distinta de los modelos de Kashyyyk y Coruscant.
Las ubicaciones se generan con ruido procedural y semillas reproducibles
(`LIFE_SEED`); la vida está confinada al cinturón templado y permanece orientada
hacia fuera mientras el planeta orbita. Cada modelo tiene dos niveles de
detalle y participa en el descarte del mundo. Ureski usa un shader de biomas;
Alfa Riddertal compone el gas y las apariciones interiores analíticamente a lo
largo del rayo de la cámara en un shader de fragmentos de CPU. El framebuffer
resuelve la profundidad de los modelos y los haces, incluidos sus ocultamientos.
Para mantener el vuelo interactivo, Ureski reutiliza un mapa de biomas calculado
en CPU y descarta la vida que queda oculta tras el horizonte. Los modelos
comparten vértices dentro de sus facetas y reducen el detalle a distancia.
El núcleo de Alfa Riddertal se calcula en una caché de sombreado alineada con
la cámara, renovada en cada cuadro y muestreada durante la rasterización;
la geometría, la proyección y la profundidad siguen en el pipeline propio.

**L: activar/desactivar sobrevuelo**, disponible en los tres vehículos.
Acércate primero a un planeta (también puedes usar `3.14` con el ovni o el destructor).
Al pulsar **L**, la nave se aproxima suavemente a una altura baja y comienza a rodear
el planeta cercano, acompañando su órbita. No hace falta mantener una tecla para avanzar.
La cámara conserva la vista habitual detrás de la nave, centrada en ella y un poco
más cerca. El terreno y la curvatura se ven debajo y por delante durante el vuelo.
La cámara mantiene su distancia respecto a la nave aunque el planeta sea grande.
Al terminar la aproximación, el sobrevuelo conserva la altura.

**Espacio** acelera el sobrevuelo; al soltarlo vuelve al crucero tranquilo.
**Shift** frena hasta detenerse y mantiene el sobrevuelo pausado; **Espacio**
reanuda la marcha. **A/D** cambian el rumbo y **W/S** permiten bajar o subir con
una inclinación limitada; también cancelan la aproximación automática para tomar
el control de la altura. Al soltar **W/S**, el morro vuelve suavemente al horizonte
y la altura se estabiliza. **L** sale del modo conservando la posición y la velocidad.
El teletransporte y `achu` abandonan el sobrevuelo; si te alejas mucho del planeta,
recuperas el vuelo espacial automáticamente.

El giro y la cámara usan suavizado exponencial. La integración de navegación
se divide en pasos de como máximo 1/120 s, manteniendo una respuesta consistente
a 30, 60 y 144 FPS. Las colisiones siguen comprobando el volumen completo del
vehículo y generan rebote, explosión y sonido.

Para demostrar esta contribución: elige el ovni, salta a uno de estos planetas,
acércate para observar sus modelos, pulsa **L** para rodearlo automáticamente,
cambia la altura con **W/S**, frena y sal con **L** para alejarte. Repite en el
otro planeta. Las inspecciones con **F** describen sus nuevas identidades.

Verificación automatizada: vuelta completa sin deriva de altura, seguimiento
de un planeta en órbita, comparación del vuelo a distintos FPS, rebotes de
ambos vehículos, semillas reproducibles, hábitats, saltos reales, bloqueo
solar de Ureski durante su órbita, estaciones orbitales, comunicación en
ambos sentidos, animación del núcleo y geometría con dos niveles de detalle.
Para generar capturas de los planetas integrados sin ventana ni sonido:

```bash
cargo run --release -- --preview-persona3
```

Las imágenes quedan en `target/previews/persona3-*.png`. La atracción
gravitatoria figura como **opcional** en la planificación y no está activada
ni implementada como fuerza física; el vuelo local es una ayuda de control.
El video y los nombres de los integrantes siguen correspondiendo a la entrega
compartida. Esta ampliación se desarrolló con asistencia de Codex/IA; los modelos
de flora, fauna y estaciones son propios del proyecto, sin descargas externas.

### Teletransporte del ovni y del destructor

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

Los tres vehículos chocan con los cuerpos celestes, incluidas lunas y estrellas.
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
- `F11`: alternar pantalla completa sin bordes en el monitor actual, tanto en el hangar como durante el vuelo. La imagen conserva sus proporciones con bandas negras cuando sea necesario; al salir se restaura la ventana.
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

Para generar capturas de los tres vehículos con el renderer CPU sin abrir ventanas:

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
