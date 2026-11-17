#!/usr/bin/env python3
"""Programa y prueba el macropad de 16 teclas y 3 perillas (USB 514c:8850).

Sin dependencias: habla directamente con /dev/hidraw* y /dev/input/event*.
Protocolo tomado de https://github.com/elmitash/macropad-16key-3knob (MIT).

    python3 herramientas/mando.py leer [-o copia.json]   # copia de seguridad de las 3 capas
    python3 herramientas/mando.py escribir               # graba el mapeo de Mapachsynthesizer en las 3 capas
    python3 herramientas/mando.py probar                 # muestra en vivo lo que manda cada control

Necesita permisos sobre el mando: instalar config/70-mapachsynthesizer-mando.rules (ver docs/mando.md).
"""

import argparse
import fcntl
import glob
import json
import os
import select
import struct
import sys
import time

VID, PID = "514c", "8850"
REPORT_ID = 0x03
SIZE = 64
SLOTS = 25  # 16 teclas + 3 perillas × (izquierda, pulsar, derecha)

KIND_TECLADO, KIND_MULTIMEDIA, KIND_RATON = 1, 2, 3

# Códigos de uso HID (página de teclado) del mapeo de Mapachsynthesizer.
F13 = 0x68  # F13..F24 = 0x68..0x73
KP = {"/": 0x54, "*": 0x55, "-": 0x56, "1": 0x59, "2": 0x5A, "3": 0x5B, "4": 0x5C,
      "5": 0x5D, "6": 0x5E, "7": 0x5F, "8": 0x60, "9": 0x61, "0": 0x62}

# Ranura del firmware -> (nombre del control, código HID).
# Con las perillas arriba, el firmware numera las teclas por columnas y de abajo arriba:
#   ranura  4  8 12 16      tecla  1  2  3  4
#           3  7 11 15   =         5  6  7  8
#           2  6 10 14             9 10 11 12
#           1  5  9 13            13 14 15 16
# (comprobado con el mando real el 2026-11-17). Ranuras 17-19 perilla 1, 20-22 perilla 2,
# 23-25 perilla 3 (de izquierda a derecha); dentro de cada perilla: girar a la DERECHA,
# pulsar, girar a la IZQUIERDA (también comprobado).
CODIGO_TECLA = [F13 + i for i in range(12)] + [KP["1"], KP["2"], KP["3"], KP["4"]]
MAPEO = {}
for tecla in range(1, 17):
    fila, col = divmod(tecla - 1, 4)
    MAPEO[col * 4 + (4 - fila)] = (f"Tecla {tecla}", CODIGO_TECLA[tecla - 1])
for n, (izq, der, pulsar) in enumerate([("5", "6", "7"), ("8", "9", "0"), ("/", "*", "-")]):
    base = 17 + n * 3
    MAPEO[base] = (f"P{n + 1} derecha", KP[der])
    MAPEO[base + 1] = (f"P{n + 1} pulsar", KP[pulsar])
    MAPEO[base + 2] = (f"P{n + 1} izquierda", KP[izq])

# Códigos evdev (linux/input-event-codes.h) de esos mismos HID, para `probar`.
EVDEV = {183 + i: f"F{13 + i}" for i in range(12)}
EVDEV.update({79: "Num 1", 80: "Num 2", 81: "Num 3", 75: "Num 4", 76: "Num 5", 77: "Num 6",
              71: "Num 7", 72: "Num 8", 73: "Num 9", 82: "Num 0", 98: "Num /", 55: "Num *",
              74: "Num -", 114: "Bajar volumen", 115: "Subir volumen", 113: "Silencio",
              164: "Play/pausa", 163: "Siguiente", 165: "Anterior"})
HID_A_NOMBRE = {F13 + i: f"F{13 + i}" for i in range(12)}
HID_A_NOMBRE.update({v: f"Num {k}" for k, v in KP.items()})


def fallo(msg):
    print(f"Error: {msg}", file=sys.stderr)
    sys.exit(1)


# --- HID ------------------------------------------------------------------

def buscar_hidraw():
    """Devuelve /dev/hidrawN de la interfaz 0 (la de configuración) del mando."""
    for sys_path in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        dev = os.path.realpath(os.path.join(sys_path, "device"))
        # .../1-6.3:1.0/0003:514C:8850.0009
        nombre = os.path.basename(dev).upper()
        interfaz = os.path.basename(os.path.dirname(dev))
        if f":{VID.upper()}:{PID.upper()}." in nombre and interfaz.endswith(":1.0"):
            return "/dev/" + os.path.basename(sys_path)
    fallo("no encuentro el mando 514c:8850. ¿Está conectado por cable?")


def abrir_hid():
    ruta = buscar_hidraw()
    try:
        return os.open(ruta, os.O_RDWR)
    except PermissionError:
        fallo(f"sin permiso para abrir {ruta}. Instala config/70-mapachsynthesizer-mando.rules "
              "(ver docs/mando.md) o ejecuta con sudo.")


def enviar(fd, datos):
    paquete = bytes([REPORT_ID]) + bytes(datos).ljust(SIZE, b"\0")
    os.write(fd, paquete)


def recibir(fd, espera=0.5):
    listo, _, _ = select.select([fd], [], [], espera)
    if not listo:
        return None
    datos = os.read(fd, SIZE + 1)
    return datos[1:] if datos and datos[0] == REPORT_ID else datos


def vaciar(fd):
    while recibir(fd, 0.05) is not None:
        pass


def decodificar(r):
    """Paquete de lectura (0xFA ...) -> dict legible."""
    slot, capa, kind = r[1], r[2], r[3]
    out = {"ranura": slot, "capa": capa, "tipo": kind, "crudo": r[:24].hex()}
    if kind == KIND_TECLADO:
        n = r[5]
        codes = [r[6 + i * 3 + 2] for i in range(min(n, 18))]
        out["codigos"] = [c for c in codes if c]
        out["nombres"] = [HID_A_NOMBRE.get(c, hex(c)) for c in out["codigos"]]
    elif kind == KIND_MULTIMEDIA:
        out["multimedia"] = hex(r[8] | (r[11] << 8))
    elif kind == KIND_RATON:
        out["raton"] = {"botones": r[11], "rueda": r[20] - 256 if r[20] > 127 else r[20]}
    return out


def leer_capa(fd, capa):
    vaciar(fd)
    enviar(fd, [0xFA, SLOTS, 0x00, capa])
    filas = []
    for _ in range(SLOTS):
        r = recibir(fd)
        if r is None:
            break
        if r[0] == 0xFA:
            filas.append(decodificar(r))
    return filas


def paquete_tecla(slot, capa, codigo):
    p = [0] * SIZE
    p[0], p[1], p[2], p[3] = 0xFD, slot, capa, KIND_TECLADO
    p[4], p[5] = 0, 1          # una sola tecla, sin modificadores
    p[6], p[7], p[8] = 0, 0, codigo  # retardo 0 ms + código
    return p


# --- Comandos ---------------------------------------------------------------

def cmd_leer(args):
    fd = abrir_hid()
    config = {str(c): leer_capa(fd, c) for c in (1, 2, 3)}
    os.close(fd)
    for capa, filas in config.items():
        print(f"Capa {capa}: {len(filas)} ranuras")
        for f in filas:
            valor = f.get("nombres") or f.get("multimedia") or f.get("raton")
            nombre = MAPEO.get(f["ranura"], ("?",))[0]
            print(f"  {f['ranura']:2} {nombre:14} tipo {f['tipo']}  {valor}")
    if args.salida:
        with open(args.salida, "w", encoding="utf-8") as fh:
            json.dump(config, fh, indent=2, ensure_ascii=False)
        print(f"Guardado en {args.salida}")


def cmd_escribir(args):
    fd = abrir_hid()
    for capa in (1, 2, 3):
        print(f"Escribiendo capa {capa}…")
        for slot in range(1, SLOTS + 1):
            enviar(fd, paquete_tecla(slot, capa, MAPEO[slot][1]))
            time.sleep(0.04)
        enviar(fd, [0xFD, 0xFE, 0xFF])  # confirmar y guardar en la memoria del mando
        time.sleep(0.3)

    print("Comprobando…")
    errores = 0
    for capa in (1, 2, 3):
        leidas = {f["ranura"]: f for f in leer_capa(fd, capa)}
        for slot, (nombre, codigo) in MAPEO.items():
            f = leidas.get(slot)
            if not f or f["tipo"] != KIND_TECLADO or f.get("codigos") != [codigo]:
                errores += 1
                print(f"  capa {capa} {nombre}: esperaba {HID_A_NOMBRE[codigo]}, leído {f}")
    os.close(fd)
    if errores:
        fallo(f"{errores} ranuras no coinciden")
    print("Hecho: las 3 capas tienen el mapeo de Mapachsynthesizer.")


def cmd_probar(args):
    rutas = sorted(glob.glob(f"/dev/input/by-id/usb-{VID}_*-event-*"))
    if not rutas:
        fallo("no encuentro los dispositivos de entrada del mando")
    fds = {}
    for r in rutas:
        try:
            fd = os.open(r, os.O_RDONLY)
        except PermissionError:
            fallo(f"sin permiso para leer {r}. Instala config/70-mapachsynthesizer-mando.rules o usa sudo.")
        fcntl.ioctl(fd, 0x40044590, 1)  # EVIOCGRAB: que no llegue al escritorio
        fds[fd] = r
    print("Pulsa teclas y gira o pulsa perillas. Ctrl+C para salir.")
    fmt = "llHHi"
    tam = struct.calcsize(fmt)
    try:
        while True:
            listos, _, _ = select.select(list(fds), [], [])
            for fd in listos:
                datos = os.read(fd, tam * 64)
                for i in range(0, len(datos), tam):
                    _, _, tipo, codigo, valor = struct.unpack(fmt, datos[i:i + tam])
                    if tipo == 1 and valor == 1:  # EV_KEY, pulsada
                        print(f"  {EVDEV.get(codigo, f'código {codigo}')}")
                    elif tipo == 2:  # EV_REL (rueda del ratón)
                        print(f"  ratón rel {codigo} {valor:+}")
    except KeyboardInterrupt:
        print()


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest="cmd", required=True)
    leer = sub.add_parser("leer", help="lee y muestra la configuración de las 3 capas")
    leer.add_argument("-o", "--salida", help="guardar también en JSON")
    sub.add_parser("escribir", help="graba el mapeo de Mapachsynthesizer en las 3 capas")
    sub.add_parser("probar", help="muestra en vivo lo que manda cada control")
    args = p.parse_args()
    {"leer": cmd_leer, "escribir": cmd_escribir, "probar": cmd_probar}[args.cmd](args)


if __name__ == "__main__":
    main()
