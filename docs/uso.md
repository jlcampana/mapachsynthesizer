# Cómo se usa Mapachsynthesizer

Guía rápida. El mapeo completo del mando está en [mando.md](mando.md).

## Tocar

| Teclado | Mando | Qué hace |
|---|---|---|
| `A S D F G H J` | Teclas 1–7 | Los 7 acordes de colores (`Q`…`U`: una octava más agudos) |
| Flechas | Teclas 9–12 | Con un acorde pulsado, cambian su color (dos a la vez: diagonales) |
| Rueda del ratón | Rueda grande (derecha) | Cursor, como en un iPod (ver abajo) |
| `RePág` / `AvPág` | Perilla izquierda | Volumen (pulsarla: silencio) |
| `-` / `+` | Perilla del centro | Tempo (`B`: tap tempo) |
| `Intro` | OK (verde) | El botón central del iPod en menús, Canciones y Taller; si no, play / pausa |
| `V N M ,` | Atajos (fila 3) | Disparan los samples que les hayas puesto |

### La rueda grande

- **Mantén un botón de abajo y gira:** Tono → tonalidad, Sonido → sonido, Modo → modo, Play → pista
  del looper.
- **Con un menú abierto:** girar mueve el cursor; Play entra a cambiar el valor y Play otra vez sale.
- **Con un acorde pulsado:** recorre sus colores.
- **En Canciones y en el Taller:** mueve la selección y Play elige.

## Los tres menús

Pulsar y soltar abre su página (rueda o ↑↓ eligen; Play o ←→ cambian). Si se mantienen pulsados,
hacen atajos:

| Menú | Mantener + ←→ o rueda | Mantener + ↑↓ | Mantener + volumen | Mantener + tempo |
|---|---|---|---|---|
| Gris `Z` (tono) | Tonalidad | Octava | Ataque | Octava |
| Amarillo `X` (sonido) | Sonido | Efecto | Release | Cantidad de efecto |
| Rojo `C` (modo) | Modo | Tempo | Brillo | Patrón / variación / kit |

Mantener un menú y pulsar Grabar (`Espacio`) pone valores al azar 🎲.

En la página gris están también la escala, las tablas de flechas (Básico, Jazz, Prestados), el bajo,
los nombres de acordes y **Mis canciones**. En la roja, el **juego de oído**.

## Modos (menú rojo)

Tocar, Rasgueo, Arpegio, Repetir, Zumbido (sigue sonando al soltar; `Intro` lo para), Melodía (una
nota por tecla), Batería (las teclas tocan golpes), Ritmos (un ritmo suena solo y encima se tocan
acordes; `Intro` para y arranca) y Secuencia (16 pasos: en el mando, las 16 teclas; en el teclado,
`1`–`8` y `Q`–`I`; la rueda elige instrumento, y la última fila, "Salir", vuelve a Ritmos con "Mi
ritmo").

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

Grabar (`Espacio`) graba con el micro (hasta 10 s) y la rueda elige sonido (y suena). Para
destrozarlo, mantén OK, gira la rueda hasta el botón que quieras y suelta OK; pulsar OK sin girar
repite el último. Los botones de colores: **gris** sale, **rojo** deshace y
**amarillo** usa el sonido en los acordes ("Mi sample"). Mantener el amarillo y pulsar una tecla de
acorde pone el sonido en ese golpe de la batería (otra vez lo mismo: vuelve el golpe de serie). Para traer sonidos de fuera, déjalos en
`Música/Mapachsynthesizer/Importar` (WAV, MP3, OGG o FLAC) y abre el taller.

## Juegos (menú rojo)

- **Juego de oído**: suena un acorde y hay que pulsar su tecla de color.
- **Cópiame**: suena una serie de acordes que se iluminan, y hay que repetirla. Cada vez es más larga.
- **Sigue la canción**: brilla la siguiente tecla de Estrellita, el Himno de la alegría, Cumpleaños
  feliz o Martinillo. Se va al ritmo que se quiera; "↑" quiere decir la fila de arriba (`Q`…`U`).

Play (`Intro`) repite y cualquier botón de color sale. Equivocarse nunca castiga: se vuelve a
intentar. Los juegos dan **estrellas** ⭐ y con 10 y 25 se desbloquean dos sonidos secretos.

## Atajos a samples

La fila 3 del mando (en el teclado, `V N M ,`) son 4 atajos: cada uno dispara un sample entero, que
también se graba en el looper. Para ponerle un sonido a un atajo: en el Taller, elige el sonido con
la rueda, mantén el botón amarillo y pulsa el atajo. Hacerlo otra vez con el mismo sonido lo vacía.

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
- `Ctrl+Alt+P` vuelve a tocar. `Esc` (en cualquier momento) cierra el programa.

## Ficheros

| Qué | Dónde |
|---|---|
| Ajustes y presets | `~/.config/mapachsynthesizer/ajustes.ron` |
| Mando aprendido | `~/.config/mapachsynthesizer/mando.ron` |
| Canciones | `~/Música/Mapachsynthesizer/Canciones/` |
| Exportadas | `~/Música/Mapachsynthesizer/Exportadas/` |
| Samples | `~/Música/Mapachsynthesizer/Samples/` |
| Para importar | `~/Música/Mapachsynthesizer/Importar/` |
