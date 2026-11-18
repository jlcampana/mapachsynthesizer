# Investigación

Fecha: 2026-11-06 (sección 6: 2026-11-17)

Este documento recoge lo que hacen los dos dispositivos de referencia y qué partes nos
interesan para un sintetizador de escritorio en Rust, pensado para un niño y controlado con
teclado y perilla.

---

## 1. HiChord (referencia principal de la interfaz)

Fuente: [manual oficial, firmware Rev 3.0](https://updater.hichord.shop/manual/)

Es un instrumento de bolsillo con carcasa **amarilla**, pantalla OLED pequeña y muy pocos
controles. Su gracia es que **cualquier botón que pulses suena bien**: los 7 botones tocan los 7
acordes de la tonalidad.

### Controles físicos

| Control | Función |
|---|---|
| 7 botones de acorde | Tocan los acordes diatónicos I–vii° (en Do: C, Dm, Em, F, G, Am, Bdim) |
| Joystick 8 direcciones + click | Mientras mantienes un acorde, cambia su "color" (7ª, sus4, menor…). También navega por los menús y arma el looper |
| Botón **gris** | Tonalidad y ajustes (←/→ cambia la tonalidad, ↑/↓ la octava) |
| Botón **amarillo** | Sonidos y efectos (←/→ cambia el sonido, ↑ abre efectos) |
| Botón **rojo** | Modos y tempo (←/→ cambia el modo, ↑ abre BPM) |
| Rueda de volumen | Volumen. Con gris/amarillo/rojo pulsado: attack / release / cutoff |
| Pantalla OLED 64×32 | Nombre del acorde, menús, estado del looper |

### Funciones que copiamos

- **Acordes por tonalidad**: 12 tonalidades y 10 escalas (mayor, menor, pentatónica, blues, dórica…).
- **Modificadores con joystick** (modo por defecto):
  ↑ mayor/menor · ↗ 7ª dominante · → 7ª · ↘ 9ª · ↓ sus4 · ← disminuido · ↖ aumentado.
  Tiene además los modos *Extended*, *Chromatic* y *Borrow* (acordes prestados).
- **Inversiones** y *voice leading* automático (elige la inversión que menos se mueve).
- **Chord lock**: fija un acorde modificado a un botón.
- **28 sonidos** (analógico, FM, samples, híbridos, ruido).
- **Efectos**: reverb, delay sincronizado al BPM, chorus, flanger, drive, tape, tremolo, vibrato,
  filtro y vocoder.
- **Modos de toque**: Play, Strum (rasgueo), Lead (una nota), Drone, Arp (6 patrones),
  Repeat, Sequencer (16 pasos de acordes), Drums (7 kits), Drum Loops (56 ritmos), Auto-drum.
- **Tempo** de 40 a 300 BPM, con *tap tempo*. Controla arpegiador, delay, tremolo, ritmos y metrónomo.
- **Looper de 6 pistas** de unos 20 s. La pista 1 marca la duración y las demás se sincronizan.
  Al salir del secuenciador o de los ritmos, el patrón se graba solo en una pista libre.
- **Sampleo por micro** (~3 s) con detección de tono y transposición a Do.
- **4 presets** (P1–P4) que lo guardan todo.
- **Aleatorio**: menú + click del joystick llena ese menú con valores al azar. A los niños les encanta.
- **Juegos**: *Chord Hiro* (ritmo) y *Ear Trainer* (entrenar el oído).

---

## 2. M-VAVE FM-1

Fuentes: [Synthtopia](https://www.synthtopia.com/content/2026/07/13/m-vave-introduces-fm-1-fm-pocket-synthesizer/),
[review en Synth Anatomy](https://synthanatomy.com/2026/07/m-vave-fm-1-review-low-budget-pocket-fm-ynthesizer-with-iconic-sounds.html),
[firmware abierto FM1 Open](https://synthtopia.com/content/2026/10/20/m-vave-officially-opens-fm-1-firmware-to-developers-with-new-workbench-sdk),
[sampler lo-fi de la comunidad](https://www.matrixsynth.com/2026/11/he-made-fair-sampler-for-fm-1-and-it.html).
(La página del producto en m-vave.com solo muestra la cabecera, sin especificaciones.)

Es un sinte FM pequeño al estilo Yamaha DX7, con teclado de silicona.

- Motor **FM de 6 operadores**, 32 algoritmos, 12 voces, 128 presets y compatible con SysEx del DX7
  (según un comentario de lector, basado en Dexed/MiniDexed).
- **Arpegiador**: direcciones, octavas, rate, gate, swing, latch.
- **Secuenciador de 16 pasos**, mono o polifónico, con 16 patrones.
- **Multiefectos**: filtro (LP/HP/BP), chorus, phaser, distorsión, delay y reverb (room/hall/plate).
- Firmware abierto (FM1 Open) y un sampler lo-fi hecho por la comunidad.

### Qué cogemos del FM-1

- Un **motor FM** con pocos algoritmos fáciles (2 y 4 operadores) para campanas, pianos
  eléctricos y bajos. 6 operadores no compensan para un niño.
- **Secuenciador de 16 pasos con patrones** guardables.
- **Arpegiador** con swing y latch.
- **Distorsión, phaser y filtro multimodo**.
- La idea del **sampler lo-fi**: samples cortos, bitcrush, reducción de frecuencia de muestreo.

### Qué no cogemos

- La edición de operadores y envolventes de DX7 es demasiado compleja para un niño. Como mucho,
  la dejamos en un "modo papá".

---

## 3. Lo que tú pediste además

| Requisito | Cómo lo resolvemos |
|---|---|
| Crear música | Acordes + modos + batería + secuenciador + looper |
| Guardarla | Proyectos con guardado automático y nombres divertidos generados solos |
| Convertir a OGG/MP3 | Render del proyecto a WAV, OGG Vorbis y MP3 |
| Gestionar samples | Biblioteca de samples: importar, grabar con micro, recortar, renombrar, borrar |
| Distorsionarlos | "Taller de samples": distorsión, bitcrush, al revés, más agudo/grave, eco, robot… |
| Interfaz tipo HiChord | Pantalla central oscura tipo OLED, carcasa amarilla/naranja, 7 botones de colores |
| Colores vivos | Paleta amarillo, naranja, rojo, verde, azul, morado (ver bocetos) |
| Control por teclado | Mapa de teclas que copia los botones del HiChord |
| Perilla (teclado de AliExpress) | Lectura directa con `evdev`. La perilla controla volumen, tempo y parámetros como la rueda del HiChord |

---

## 4. La perilla de los teclados de AliExpress

La mayoría de teclados y macropads con perilla (encoder rotatorio) envían por defecto
**teclas multimedia**: `KEY_VOLUMEUP`, `KEY_VOLUMEDOWN` y `KEY_MUTE` al pulsarla.
Eso trae dos problemas:

1. El escritorio (GNOME/KDE) intercepta esas teclas y cambia el volumen del sistema.
2. Las ventanas no siempre reciben esos eventos.

**Solución propuesta**, en orden de preferencia:

1. **Leer el dispositivo con `evdev`** desde Rust y "agarrarlo" en exclusiva (`EVIOCGRAB`)
   mientras la app está abierta. Así el sistema no ve los giros y la app los recibe todos.
   Hace falta que el usuario esté en el grupo `input` o añadir una regla `udev` (la documentamos).
2. Si el teclado es compatible con **QMK/VIA**, reasignar la perilla a `F13`/`F14`/`F15`,
   que ninguna aplicación usa. Así funciona incluso sin `evdev`.
3. Como alternativa sin hardware: `Re Pág`/`Av Pág` o la rueda del ratón hacen de perilla.

**Consejo de compra**: elegir un modelo que diga **"QMK" o "VIA"** en la descripción. Un macropad
con **3 perillas** (hay varios de 16 teclas + 3 encoders) daría volumen, tempo y "parámetro"
independientes, con más juego que una sola perilla. Antes de comprar, revisad que no necesite
software solo para Windows.

---

## 5. Elección de tecnologías (Rust)

| Necesidad | Crate | Por qué |
|---|---|---|
| Salida/entrada de audio | `cpal` | Estándar en Rust. Funciona con PipeWire/ALSA/Pulse |
| Comunicación UI ↔ audio sin bloqueos | `rtrb` (ring buffer) + `triple_buffer` | El hilo de audio no puede esperar ni reservar memoria |
| DSP | Propio + `fundsp` para prototipos | Osciladores, filtros SVF, envolventes, efectos. Código propio para tenerlo controlado y testeable |
| Interfaz | `eframe`/`egui` con dibujo propio | Pantalla completa, se dibuja todo a mano (estilo HiChord), rápido de iterar |
| Teclado | eventos de `egui`/`winit` | Teclas normales |
| Perilla | `evdev` | Lectura directa y exclusiva del encoder |
| Leer samples (wav/mp3/ogg/flac) | `symphonia` | Decodificador en Rust puro |
| Cambiar frecuencia de muestreo | `rubato` | Remuestreo de calidad al importar |
| Escribir WAV | `hound` | Simple y fiable |
| Exportar OGG | `vorbis_rs` | Bindings a libvorbis, buena calidad |
| Exportar MP3 | `mp3lame-encoder` | Bindings a LAME |
| Guardar proyectos | `serde` + `ron` | Legible a mano y fácil de versionar |
| MIDI (opcional, futuro) | `midir` | Por si algún día conectáis un teclado MIDI o un HiChord de verdad |
| Detección de tono (sampleo por micro) | `pitch-detection` (YIN/McLeod) | Para transponer samples a Do, como hace HiChord |

Alternativa a los encoders: llamar a `ffmpeg` si está instalado. Lo dejamos como plan B porque
añade una dependencia externa.

---

## 6. Firmwares de la comunidad del FM-1

Fecha: 2026-11-17. Enlaces de partida en [FUENTES.md](../FUENTES.md).

Desde que M-VAVE abrió el firmware del FM-1, la comunidad ha publicado decenas de firmwares
alternativos ([The Firmware Hub](https://thefirmwarehub.com/) lista más de 30). Casi todos parten de
[Felucca](https://github.com/hugelton/Felucca) y son **GPL-3.0**. Dos están muy cerca de lo que
queremos hacer: **fm1-chord**, que convierte el FM-1 en un HiChord, y **Rainbow mode**, un juguete
musical para niños pequeños.

> **Licencias:** si copiamos código de estos proyectos, Mapachsynthesizer tendría que ser GPL-3.0. Las tablas
> musicales (escalas, intervalos, qué hace cada dirección) son datos que podemos reescribir en Rust.
> fm1-chord avisa de que el fabricante del HiChord dice tener una **patente en trámite** sobre la
> asignación de acordes. Para un proyecto privado no importa. Si algún día publicamos Mapachsynthesizer,
> conviene usar nombres propios y citar los precedentes: Omnichord, Suzuki QChord y Casio Chordana.

### 6.1 fm1-chord: un HiChord dentro del FM-1

Fuente: [math0ne/fm1-chord](https://github.com/math0ne/fm1-chord) (GPL-3.0). Lo más útil está en
`spec/docs/02-chord-machine-reference.md` y `spec/docs/04-chord-engine-tables.md`.

Es una reimplementación del HiChord hecha solo a partir de manuales públicos y del protocolo de su
app. Trae las **tablas completas del motor de acordes**, y eso es justo lo que necesitamos para la
fase 1 de Mapachsynthesizer:

- **10 escalas** con sus grados, y el acorde por defecto de cada tecla en cada escala. En la mayor:
  I mayor, ii menor, iii menor, IV mayor, V mayor, vi menor y vii disminuido. Con las pentatónicas
  y la de blues, las teclas siguen las notas de la escala y pasan a la octava siguiente cuando se
  acaban.
- **29 tipos de acorde** con sus intervalos: mayor, menor, 7ª, maj7, 9ª, sus2, sus4, 6ª, add9,
  disminuido, aumentado, m7♭5, 13ª…
- **Modificadores: qué hace cada flecha**, en 4 tablas. La tabla básica depende de si el acorde
  es mayor o menor:

  | Flecha | Acorde mayor | Acorde menor | Disminuido (vii) |
  |---|---|---|---|
  | ↑ | menor | mayor | menor |
  | ↗ | 7ª dominante | 7ª dominante | 7ª dominante |
  | → | maj7 | m7 | m7♭5 |
  | ↘ | maj9 | m9 | m7♭5 |
  | ↓ | sus4 | sus4 | sus4 |
  | ↙ | 6ª | sus2 | sus2 |
  | ← | disminuido | disminuido | menor |
  | ↖ | aumentado | aumentado | aumentado |

  Las otras tablas son *Extended*, *Chromatic* (←/→ sube o baja la tonalidad un semitono) y
  *Borrow*: dominante secundaria, sustituto tritonal, napolitano, acordes paralelos…
- **Voicing en 6 voces:** raíz, 3ª, 5ª, bajo (dos octavas abajo) y dos extensiones. Si están
  activadas, cada voz lleva una copia desafinada de 5 a 15 cents y panoramizada al lado contrario,
  para dar un sonido estéreo ancho.
- **Inversiones** (posición fundamental, 1ª y 2ª) y un **algoritmo de voice leading** sencillo:
  prueba 3 inversiones × 3 octavas y se queda con la que mueve menos las notas respecto al acorde
  anterior.
- **Arpegios** como patrones de 8 pasos que se refieren a "raíz, 3ª, 5ª, 7ª" y no a notas
  concretas, así que funcionan con cualquier acorde. Ritmos: de 1/1 a 1/32, tresillos y swing.
- **Rasgueo:** 200, 80 o 40 ms entre nota y nota.
- **7 envolventes predefinidas** (LONG, SHORT, SWELL, PLUCK, TOUCH, SUSTAIN, KEYS), con sus valores
  ADSR en milisegundos.
- **Batería:** cada ritmo es un patrón de 16 pasos, con un bit por instrumento (bombo, caja,
  charles, tom, charles abierto, platillo). Hay 7 estilos × 8 variaciones.

Ideas suyas que no teníamos:

- **Acorde con bajo distinto** (*slash chord*): la primera tecla que se mantiene da el bajo y la
  segunda el acorde (Em/C).
- **Arpa de rasgueo** al estilo Omnichord: unas teclas tocan una a una las notas del último acorde,
  y al pasar el dedo por encima se rasguea.
- **En reposo, la pantalla muestra qué hace cada tecla.**

### 6.2 Rainbow mode: un juguete musical para niños pequeños

Fuente: [joshbartlettband-web/felucca-rainbow](https://github.com/joshbartlettband-web/felucca-rainbow)
(GPL-3.0). Todo el diseño está explicado en `RAINBOW.md`, y el código en `firmware/src/kid.c`.

Está diseñado para niños que todavía no leen, y de él hay mucho que aprender:

- **Las perillas dicen lo que hacen con imágenes:** *día → noche* (el sonido se oscurece y sale la
  luna), *eco* y *meneo* (vibrato).
- **Efectos de un toque, con nombre gracioso:** HIPO (tartamudeo), AL REVÉS, DORMIDO (el sonido se
  frena como un tocadiscos), CHILLÓN (una octava arriba), GIGANTE (una octava abajo) y CONGELAR.
  Se activan y se desactivan con la misma tecla, y vuelven a empezar con cada nota.
- **Modos de juego:**
  - *ADIVINA:* suena una nota y hay que encontrarla. Empieza con Do y Sol y va añadiendo notas.
  - *SIGUE:* se ilumina la tecla de la siguiente nota de una canción infantil. El niño va a su ritmo
    y si se equivoca no pasa nada.
  - *ACORDES:* lecciones de 4 acordes.
  - *CÓPIAME:* un juego de memoria tipo Simón.
  - *ESCRIBE:* un pentagrama para componer al estilo del Mario Paint.
- **Estrellas y secretos:** cada juego ganado da una estrella. Con 10, 25 y 50 estrellas se
  desbloquean sorpresas secretas.
- **Pentagrama en directo:** las notas que se tocan aparecen escritas con su color, y su duración
  se ve crecer mientras se mantiene la tecla.
- **Colores de las notas**, los de los tubos y campanas de las clases de música: Do rojo, Re
  naranja, Mi amarillo, Fa verde, Sol turquesa, La morado, Si rosa. Los sostenidos llevan el color
  intermedio.
- **Seguridad del volumen:**
  - El adulto fija el volumen máximo en 8 pasos de unos 3 dB.
  - Un **limitador** sigue a ese máximo, para que ningún efecto suba el volumen por encima de lo
    que ya se oía.
  - Los agudos se suavizan unos 4 dB por encima de unos 3,3 kHz, que es la zona que molesta en
    altavoces pequeños y auriculares.
- **Volumen igualado de oído:** midieron la sonoridad percibida de cada sonido con el método de
  radiodifusión (ITU-R BS.1770, ponderación K) y la ajustaron. Pasaron de 12 dB de diferencia entre
  sonidos a unos 3,5. Un acorde suena tan fuerte como una nota sola.
- **Ajustes de adulto escondidos:** se mantiene una tecla 1 s y se gira una perilla. Así se
  cambian el volumen máximo, el nombre del niño en el saludo, si las notas se llaman Do-Re-Mi o
  C-D-E, y si se ve el pentagrama.
- **Saludo con el nombre del niño** al encender, con letras de arcoíris y confeti.
- **Nunca sobrescribe el trabajo de los adultos:** el modo niño tiene su propio guardado.

### 6.3 Otros enlaces

- **[fm1-emu](https://github.com/joakimeriksson/fm1-emu)** (MIT): un emulador del FM-1 escrito en
  **Rust**, que corre en el navegador (WebAssembly), como plugin (VST3/CLAP) y como app de
  escritorio. Es buena referencia para el audio en tiempo real en Rust. Usa el teclado del
  ordenador como piano: dos filas de teclas para las notas, Z/X para la octava y espacio para Play.
- **[fm1-dock](https://github.com/joakimeriksson/fm1-dock)** (GPL-3.0): varios motores de sonido
  intercambiables, con ajustes comunes como la ecualización y el MIDI. Si un motor falla varias
  veces o va lento, se salta. Es buena idea para que un sonido roto no tumbe toda la app.
- **[FM1-Animatek](https://github.com/animatek/FM1-Animatek)** (GPL-3.0): el firmware ATEK303 con el
  motor **Open303**, un sonido "ácido" tipo Roland TB-303. Lleva hoja de trucos y demo en el
  navegador.
- **[fm1_open](https://github.com/m-vave/fm1_open)** (oficial): el motor de sonido viene compilado
  y no se puede reutilizar. Tiene una idea buena: **grabar por pasos**, donde cada acorde que
  sueltas avanza un paso.
- **[The Firmware Hub](https://thefirmwarehub.com/):** otros firmwares que dan ideas:
  - *FoMni*, un arpa de acordes.
  - *Polly*, una banda que sigue tus acordes.
  - *X0X*, cajas de ritmos 808 y 909.
  - *ETCH*, para dibujar tu propia forma de onda. Encajaría con los niños.
  - *Fart Machine One*, que hace exactamente lo que dice su nombre. Seguramente sería el más
    popular entre los niños.

### 6.4 Qué podríamos llevar a Mapachsynthesizer

**Ya, en la fase 1**, de fm1-chord:

1. **Usar sus tablas como especificación** del motor de acordes: escalas, tipos de acorde, las 4
   tablas de flechas, voicing, inversiones y voice leading. Las reescribimos en Rust con tests.
   Así nos ahorramos adivinar cómo se comporta el HiChord.
2. **Pantalla en reposo con la leyenda de las teclas**: qué hace cada una cuando no se está tocando.

**Al final, en la fase 10 (Pulido)**, las ideas de Rainbow mode:

3. **Colores por nota** (Do rojo, Re naranja, Mi amarillo, Fa verde, Sol turquesa, La morado,
   Si rosa) en vez del arcoíris por grados, para que el color de cada nota sea siempre el mismo.
   En Do mayor los dos sistemas casi coinciden, porque I–vii caen en Do–Si. Solo cambian al
   cambiar de tonalidad: en Sol mayor, la tecla I sería turquesa.
4. **Seguridad del volumen**: volumen máximo para el adulto, un limitador que lo siga, agudos
   suavizados y los sonidos igualados por sonoridad, con un test que lo compruebe. El limitador
   fijo de la fase 5 ya cubre lo básico hasta entonces.
5. ~~**Efectos de un toque con nombre de niño**: hipo, al revés, dormido, chillón, gigante y congelar.~~
   Descartado (18-11-2026).
6. **Juegos**: *Sigue la canción* y *Cópiame*, que son más sencillos que el *Chord Hiro*, y
   estrellas que desbloquean cosas.
7. **Una tarjeta corta que el adulto pueda leer en voz alta** al cambiar de modo.

Descartado: los personajes por sonido de Rainbow mode.
