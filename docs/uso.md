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

| Mando | Teclado | Qué hace |
|---|---|---|
| Pulsar la rueda grande | — | Abre y cierra la vista de loops |
| Girar la rueda (con la vista abierta) | `1`–`6` | Elige pista |
| Grabar ● | `Espacio` | Empieza y para la grabación de la pista elegida |
| OK | `Intro` | Sigue o para todos los loops |
| Pulsar la perilla izquierda (vista abierta) | — | Silencia o devuelve el sonido de la pista elegida |
| Pulsar la perilla central (vista abierta), dos veces | Mantener `1`–`6` 1,5 s | Borra la pista elegida |
| — | `Retroceso` | Deshace la última grabación |

La primera grabación marca la duración de la vuelta (si suena un ritmo, se redondea a compases); las
demás graban una vuelta y paran solas. Para borrar hay que pulsar dos veces la perilla central: la
primera la pista se pone en rojo con "¿Borrar?", y la segunda (antes de 2 s) la borra.

Al abrir la app vuelve la última canción con sus loops, en pausa; para empezar de cero, Mis
canciones › Nueva.

## Canciones (`F5`)

Una canción nueva no se guarda hasta que se le da a **Guardar** en el menú `F1` (pide el nombre).
Desde entonces se guarda sola cada vez que cambia el looper, y la app la abre al arrancar.

En la pantalla Mis canciones: la rueda elige, OK (o `Espacio`) abre una canción o crea una
"Nueva", y abajo se exporta a MP3, OGG o WAV. Lo exportado va a
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
también se graba en el looper.

- **Elegir el sample de un atajo:** mantén el atajo y pulsa **OK**. Sale la lista de samples de la
  carpeta `Música/Mapachsynthesizer/Samples`: la rueda grande la recorre (y da la vuelta), cada
  sample suena al pasar por él, la **perilla central** lo marca como favorito ★ (los favoritos
  salen arriba) y **OK** lo pone en el atajo. Pulsar la rueda o un botón de color cierra la lista.
- **Vaciar un atajo:** mantenlo pulsado y pulsa la perilla central.
- También se puede desde el Taller: con un sonido elegido, mantén amarillo y pulsa el atajo.

En la carpeta `Samples` se pueden dejar directamente ficheros WAV, MP3, OGG o FLAC descargados de
internet: la app los lee tal cual.

## Presets

`F9`–`F12`: pulsar carga y mantener 1,5 s guarda sonido, efectos, tonalidad, modo y ritmo.

## Menú (`F1`)

`F1` abre el menú; dentro, cada opción va con una tecla y `Esc` lo cierra. Arriba se ve el audio, el
mando y si la canción está guardada.

| Tecla | Qué hace |
|---|---|
| `F2` | Canción nueva (se pierde lo que no esté guardado) |
| `F3` | Poner nombre o renombrar la canción (si ya está guardada, también su carpeta) |
| `F4` | Copiar la canción: la guarda con otro nombre y sigues trabajando en la copia |
| `F5` | Guardar (si aún no tiene nombre, lo pide) |
| `F6` / `F7` | Volumen máximo − / + (de −24 a −3 dBFS; −12 por defecto): el techo que nada puede pasar |
| `F8` | Colores por nota sí o no |
| `F9` | Arrancar al encender sí o no |
| `F10` | Aprender mando |

Los nombres se escriben con el teclado: `Intro` acepta, `Esc` cancela. Fuera del menú, `Esc`
cierra el programa. (Los nombres de acordes se activan en el menú gris, Tono › Nombres.)

## Ficheros

| Qué | Dónde |
|---|---|
| Ajustes y presets | `~/.config/mapachsynthesizer/ajustes.ron` |
| Mando aprendido | `~/.config/mapachsynthesizer/mando.ron` |
| Canciones | `~/Música/Mapachsynthesizer/Canciones/` |
| Exportadas | `~/Música/Mapachsynthesizer/Exportadas/` |
| Samples | `~/Música/Mapachsynthesizer/Samples/` |
| Para importar | `~/Música/Mapachsynthesizer/Importar/` |
