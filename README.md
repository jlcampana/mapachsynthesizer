# Mapachsynthesizer 🦝🎹

Sintetizador para niños, escrito en Rust, para Linux. Se toca con un mando de 16 teclas y 3
perillas (o con el teclado del ordenador). La forma de tocar está inspirada en el
[HiChord](https://updater.hichord.shop/manual/) y en el M-VAVE FM-1.

![Pantalla principal](docs/capturas/principal.png)

Con él se puede:

- Tocar **acordes que siempre suenan bien** con 7 teclas y cambiarles el "color" con las flechas.
- Elegir entre muchos **sonidos** (analógico, FM, samples) y **efectos** (eco, distorsión, robot…).
- Hacer **ritmos** y **arpegios** con el tempo controlado por la perilla.
- Grabar capas en un **looper de 6 pistas**.
- Grabar su voz y **destrozar samples** en el taller.
- **Guardar** canciones y **exportarlas a MP3, OGG o WAV**.

## Estado

🎹 **Ya se puede tocar.** Están hechas las fases 0 a 9 del [plan](docs/plan.md#5-fases-y-entregables):

- Los 7 acordes de colores con voice leading, las flechas y la rueda grande para cambiarles el
  color, y la fila `Q`…`U` una octava más aguda.
- Menús gris (tono), amarillo (sonido) y rojo (modo). Se abren al pulsar y soltar; si se mantienen,
  hacen atajos con las flechas y las perillas, y con `Espacio` ponen valores al azar 🎲.
- 20 sonidos (analógico, FM, órgano, ruido y voces) y efectos (sala, eco, coro, distorsión, robot…).
- Modos Tocar, Rasgueo, Arpegio (6 patrones), Repetir, Zumbido y Melodía, con tempo y tap tempo (`B`).
- **Batería** sintetizada (4 kits), **56 ritmos** y **secuenciador de 16 pasos**.
- **Looper de 6 pistas**: `Espacio` graba, `1`–`6` eligen pista, mantener una pista 1,5 s la
  borra y `Retroceso` deshace.
- **Mis canciones** (`F5`): se guardan con su nombre (y luego solas) y se exportan a **MP3, OGG o
  WAV** en `Música/Mapachsynthesizer/Exportadas`.
- El mando de 16 teclas y 3 perillas, leído por `evdev` en exclusiva.
- Limitador a −12 dBFS al final de todo y ajustes que se guardan solos.
- `Esc` cierra el programa. Menú `F1`: canción nueva, renombrar, copiar, guardar y ajustes.

- **Taller de samples** (`Tab`): grabar con el micro, importar de `Música/Mapachsynthesizer/Importar`,
  10 botones de destrozar con deshacer, y usar el sonido en los acordes ("Mi sample").
- Presets `F9`–`F12` (pulsar carga, mantener guarda).

- Juego de oído (menú rojo), colores fijados en las teclas 🔒 y asistente "Aprender mando".

**Falta:** el juego de ritmo, animaciones y más pulido. El micro no se ha probado con hardware real.

## Compilar y tocar

Hace falta Rust (1.88 o más nuevo) y las cabeceras de ALSA:

```bash
sudo dnf install alsa-lib-devel          # Fedora
sudo apt install libasound2-dev          # Ubuntu / Debian

./build.sh             # compila (target/release/mapachsynthesizer)
./build.sh --run       # compila y abre a pantalla completa
./build.sh --ventana   # compila y abre en una ventana
./build.sh --help      # más opciones: --test, --deb, --rpm
```

| Tecla | Qué hace |
|---|---|
| `A S D F G H J` | Acordes (`Q`…`U`, una octava más aguda) |
| Flechas | Mientras suena un acorde, cambian su color |
| `Z` `X` `C` | Menús de tono, sonido y modo |
| `RePág` / `AvPág` | Volumen (perilla de la izquierda) |
| Rueda del ratón | Rueda grande del mando (cursor, como en un iPod) |
| `-` / `+` | Tempo (perilla del centro) |
| `B` | Tap tempo |
| `Espacio` | Grabar en el looper (con un menú pulsado: 🎲 al azar) |
| `1`–`6` | Elegir pista del looper (mantener: borrarla) |
| `Intro` | OK: elegir en menús; si no, play / pausa |
| `V N M ,` | Atajos a samples (fila 3 del mando) |
| `F5` | Mis canciones |
| `Esc` | Cerrar el programa |
| `F1` | Menú: nueva, renombrar, copiar, guardar, ajustes (`Esc` lo cierra) |

### Instalar en el ordenador del niño

Cada versión publica en [Releases](https://github.com/jlcampana/mapachsynthesizer/releases) un
`.deb` (Ubuntu/Debian) y un `.rpm` (Fedora). Instalan el programa, su icono en el menú y la regla
udev del mando (después hay que desenchufar y volver a enchufar el mando).

```bash
sudo apt install ./mapachsynthesizer_*.deb    # Ubuntu
sudo dnf install ./mapachsynthesizer-*.rpm    # Fedora
```

Para generar una versión: `git tag v0.1.0 && git push origin v0.1.0`.

Para depurar sin sonido: `mapachsynthesizer --ventana --captura salida.ppm --demo 1` guarda una captura
de la pantalla (escenas 0–12).

### Documentos

- [Cómo se usa](docs/uso.md): teclas, menús, modos, looper, canciones, taller y menú `F1`.
- [Investigación](docs/investigacion.md): qué hacen el HiChord y el FM-1, la perilla de AliExpress y las crates de Rust, y los firmwares de la comunidad del FM-1 (fm1-chord, Rainbow mode).
- [Plan](docs/plan.md): principios, mapa de teclas, funciones, arquitectura, fases y preguntas abiertas.
- [Mando](docs/mando.md): mapeo del macropad de 16 teclas y 3 perillas. Ya está programado con [`herramientas/mando.py`](herramientas/mando.py).
- [Bocetos](docs/bocetos/README.md): todas las pantallas.
