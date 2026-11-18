# Cómo se usa Mapachsynthesizer

Guía rápida. El mapeo completo del mando está en [mando.md](mando.md).

## Tocar

| Teclado | Mando | Qué hace |
|---|---|---|
| `A S D F G H J` | Teclas 1–7 | Los 7 acordes de colores (`Q`…`U`: una octava más agudos) |
| Flechas | Teclas 9–12 | Con un acorde pulsado, cambian su color (dos a la vez: diagonales) |
| Rueda del ratón | Perilla 2 (girar) | Con un acorde pulsado, recorre sus colores |
| — | Perilla 2 (pulsar) | Con un acorde y un color: lo deja fijo en esa tecla 🔒 |
| `RePág` / `AvPág` | Perilla 1 | Volumen (pulsarla: silencio) |
| `-` / `+` | Perilla 3 | Tempo (pulsarla, o `B`: tap tempo) |

## Los tres menús

Pulsar y soltar abre su página (↑↓ eligen, ←→ cambian). Si se mantienen pulsados, hacen atajos:

| Menú | Mantener + ←→ | Mantener + ↑↓ | Mantener + perilla 1 | Mantener + perilla 3 |
|---|---|---|---|---|
| Gris `Z` (tono) | Tonalidad | Octava | Ataque | Octava |
| Amarillo `X` (sonido) | Sonido | Efecto | Release | Cantidad de efecto |
| Rojo `C` (modo) | Modo | Tempo | Brillo | Patrón / variación / kit |

Mantener un menú y pulsar `Espacio` (o pulsar la perilla 2) pone valores al azar 🎲.

En la página gris están también la escala, las tablas de flechas (Básico, Jazz, Prestados), el bajo,
los nombres de acordes y **Mis canciones**. En la roja, el **juego de oído**.

## Modos (menú rojo)

Tocar, Rasgueo, Arpegio, Repetir, Zumbido (sigue sonando al soltar; `Intro` lo para), Melodía (una
nota por tecla), Batería (las teclas tocan golpes), Ritmos (un ritmo suena solo y encima se tocan
acordes; `Intro` para y arranca) y Secuencia (16 pasos: en el mando, las 16 teclas; en el teclado,
`1`–`8` y `Q`–`I`; ↑↓ eligen instrumento; perilla 2 sale con "Mi ritmo").

Al cambiar de modo sale unos segundos una frase corta que lo explica, para leerla en voz alta.

## Looper

`Espacio` (Grabar en el mando) graba la pista elegida y la para. La primera grabación marca la
duración (si suena un ritmo, se redondea a compases). `1`–`6` eligen pista (en el mando: Grabar +
acorde 1–6), mantener una pista 1,5 s la borra y `Retroceso` deshace la última grabación. `Intro`
para y sigue.

## Canciones (`F5`)

Las canciones se guardan solas y la app abre la última al arrancar. En la pantalla: ←→ eligen,
`Espacio` abre (o crea una "Nueva"), ↓ baja a exportar MP3, OGG o WAV. Lo exportado va a
`Música/Mapachsynthesizer/Exportadas`.

## Taller de samples (`Tab`)

`Espacio` graba con el micro (hasta 10 s), `Intro` escucha, ↑↓ eligen sonido y ←→ (o perilla 2)
eligen un botón de destrozar. Los botones de colores: **gris** destroza, **rojo** deshace y
**amarillo** usa el sonido en los acordes ("Mi sample"). Mantener el amarillo y pulsar una tecla de
acorde pone el sonido en ese golpe de la batería (otra vez lo mismo: vuelve el golpe de serie). Para traer sonidos de fuera, déjalos en
`Música/Mapachsynthesizer/Importar` (WAV, MP3, OGG o FLAC) y abre el taller.

## Juegos (menú rojo)

- **Juego de oído**: suena un acorde y hay que pulsar su tecla de color.
- **Cópiame**: suena una serie de acordes que se iluminan, y hay que repetirla. Cada vez es más larga.
- **Sigue la canción**: brilla la siguiente tecla de Estrellita, el Himno de la alegría, Cumpleaños
  feliz o Martinillo. Se va al ritmo que se quiera; "↑" quiere decir la fila de arriba (`Q`…`U`).

`Intro` repite y la perilla mágica o cualquier menú salen. Equivocarse nunca castiga: se vuelve a
intentar. Los juegos dan **estrellas** ⭐ y con 10 y 25 se desbloquean dos sonidos secretos.

## Presets

`F1`–`F4`: pulsar carga y mantener 1,5 s guarda sonido, efectos, tonalidad, modo y ritmo.

## Modo papá (`Ctrl+Alt+P`)

Información del audio y del mando, y:

- **Volumen máximo** (−24 a −3 dBFS, −12 por defecto): el techo que nada puede pasar.
- **Nombres de acordes**: enseñar "Do", "Lam7"… en vez de números.
- **Colores por nota**: Do rojo, Re naranja, Mi amarillo, Fa verde, Sol turquesa, La morado, Si rosa
  (los colores de los tubos y campanas de clase). Viene activado; si se quita, cada tecla lleva su
  color del arcoíris aunque cambie la tonalidad.
- **Arrancar al encender**: abre Mapachsynthesizer al iniciar sesión.
- **Aprender mando** y **Mando de serie**.
- `Q` sale del programa; `Esc` vuelve.

## Ficheros

| Qué | Dónde |
|---|---|
| Ajustes y presets | `~/.config/mapachsynthesizer/ajustes.ron` |
| Mando aprendido | `~/.config/mapachsynthesizer/mando.ron` |
| Canciones | `~/Música/Mapachsynthesizer/Canciones/` |
| Exportadas | `~/Música/Mapachsynthesizer/Exportadas/` |
| Samples | `~/Música/Mapachsynthesizer/Samples/` |
| Para importar | `~/Música/Mapachsynthesizer/Importar/` |
