# Mapachsynthesizer 🦝🎹

Sintetizador para niños, escrito en Rust, para Linux. Se toca con el teclado del ordenador
y una perilla, y su interfaz de colores vivos está inspirada en el
[HiChord](https://updater.hichord.shop/manual/) y en el M-VAVE FM-1.

![Pantalla principal](docs/bocetos/01-principal.svg)

Con él se puede:

- Tocar **acordes que siempre suenan bien** con 7 teclas y cambiarles el "color" con las flechas.
- Elegir entre muchos **sonidos** (analógico, FM, samples) y **efectos** (eco, distorsión, robot…).
- Hacer **ritmos** y **arpegios** con el tempo controlado por la perilla.
- Grabar capas en un **looper de 6 pistas**.
- Grabar su voz y **destrozar samples** en el taller.
- **Guardar** canciones y **exportarlas a MP3, OGG o WAV**.

## Estado

📝 **Diseño terminado.** Todavía no hay código. El mando ya está programado y probado.

### Siguiente paso: fase 0, el esqueleto

Según el [plan](docs/plan.md#5-fases-y-entregables):

1. Crear el workspace de Rust con el crate `mapach-app`.
2. Abrir una ventana `egui` a pantalla completa.
3. Sacar sonido con `cpal`: al pulsar `A` suena una onda senoidal.
4. Medir la latencia en este ordenador y, más adelante, en el Ubuntu de destino.

**Terminada cuando:** al pulsar `A`, suena enseguida y sin cortes.

Después viene la **fase 1 (acordes)**, con `mapach-theory` hecho a partir de las tablas de
fm1-chord ([investigación §6.1](docs/investigacion.md#61-fm1-chord-un-hichord-dentro-del-fm-1)).
Además, adelantaremos la lectura del mando por `evdev` de la fase 2 en cuanto haya sonido.

Ordenador de destino: ThinkCentre con Ryzen 5 PRO, 8 GB y la penúltima versión de Ubuntu (ver
[plan, pregunta 3](docs/plan.md#7-preguntas-abiertas)).

### Documentos

- [Investigación](docs/investigacion.md): qué hacen el HiChord y el FM-1, la perilla de AliExpress y las crates de Rust, y los firmwares de la comunidad del FM-1 (fm1-chord, Rainbow mode).
- [Plan](docs/plan.md): principios, mapa de teclas, funciones, arquitectura, fases y preguntas abiertas.
- [Mando](docs/mando.md): mapeo del macropad de 16 teclas y 3 perillas. Ya está programado con [`herramientas/mando.py`](herramientas/mando.py).
- [Bocetos](docs/bocetos/README.md): todas las pantallas.
