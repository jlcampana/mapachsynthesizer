#!/usr/bin/env bash
# Compila Mapachsynthesizer.
#
#   ./build.sh            compila en modo release
#   ./build.sh --run      compila y lo abre a pantalla completa
#   ./build.sh --ventana  compila y lo abre en una ventana
#   ./build.sh --test     pasa los tests
#   ./build.sh --deb      genera el paquete .deb (necesita cargo-deb)
#   ./build.sh --rpm      genera el paquete .rpm (necesita cargo-generate-rpm)
#
# El binario queda en target/release/mapachsynthesizer.

set -euo pipefail
cd "$(dirname "$0")"

error() {
    echo "❌ $*" >&2
    exit 1
}

# --- Comprobaciones ---------------------------------------------------------

command -v cargo >/dev/null || error "Falta Rust. Instálalo con: sudo dnf install cargo  (o rustup)"

if ! pkg-config --exists alsa 2>/dev/null; then
    if command -v dnf >/dev/null; then
        error "Faltan las cabeceras de ALSA. Instálalas con: sudo dnf install alsa-lib-devel"
    else
        error "Faltan las cabeceras de ALSA. Instálalas con: sudo apt install libasound2-dev"
    fi
fi

# --- Qué hacer ----------------------------------------------------------------

accion="${1:-}"

case "$accion" in
    "")
        echo "🦝 Compilando Mapachsynthesizer (release)…"
        cargo build --release -p mapach-app
        echo "✅ Listo: target/release/mapachsynthesizer"
        ;;
    --run)
        cargo run --release -p mapach-app
        ;;
    --ventana)
        cargo run --release -p mapach-app -- --ventana
        ;;
    --test)
        cargo test --all
        ;;
    --deb)
        command -v cargo-deb >/dev/null || error "Falta cargo-deb: cargo install cargo-deb"
        cargo deb -p mapach-app
        echo "✅ Paquete en target/debian/"
        ;;
    --rpm)
        command -v cargo-generate-rpm >/dev/null ||
            error "Falta cargo-generate-rpm: cargo install cargo-generate-rpm"
        cargo build --release -p mapach-app
        cargo generate-rpm -p crates/mapach-app
        echo "✅ Paquete en target/generate-rpm/"
        ;;
    -h | --help)
        sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
        ;;
    *)
        error "Opción desconocida: $accion (usa --help)"
        ;;
esac
