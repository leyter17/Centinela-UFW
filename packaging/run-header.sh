#!/bin/sh
# ══════════════════════════════════════════════════════════════════════
#  Centinela UFW (UFW Sentinel) @VER@ — INSTALADOR AUTOEXTRACTABLE (.run)
#  Creado por Leyter Vázquez . V — Licencia GNU GPL v3
# ══════════════════════════════════════════════════════════════════════
#
# Uso:
#   sudo ./Centinela-UFW_@VER@_instalador.run              Instalación en el
#       sistema (en /usr/local/bin, /usr/local/lib/ufw-sentinel y
#       /usr/share): requiere root.
#   ./Centinela-UFW_@VER@_instalador.run --user            Instalación solo
#       para tu usuario, sin root (~/.local).
#   sudo ./Centinela-UFW_@VER@_instalador.run --uninstall  Desinstalar la
#       instalación del sistema.
#   ./Centinela-UFW_@VER@_instalador.run --uninstall --user
#       Desinstalar la instalación por usuario.
#   ./Centinela-UFW_@VER@_instalador.run --extract DIR     Solo extraer los
#       ficheros a una carpeta (portable, sin instalar nada).
#
# Este fichero es un instalador autoextraíble: la cabecera es un script
# POSIX sh y, a partir de la línea @SKIP@, va el paquete tar.gz comprimido
# con la aplicación y sus librerías gráficas embebidas.
set -eu

VER="@VER@"
APP="ufw-sentinel"
SKIP="@SKIP@"

say() { printf '%s\n' "$*"; }

show_help() {
    say "Centinela UFW (UFW Sentinel) v$VER — instalador"
    say "Creado por Leyter Vázquez . V — Licencia GNU GPL v3"
    say ""
    say "Uso: $0 [opción]"
    say "  (sin opciones)   Instalar en el sistema (requiere root/sudo)"
    say "  --user           Instalar solo para tu usuario (~/.local, sin root)"
    say "  --uninstall      Desinstalar (añade --user para la instalación de usuario)"
    say "  --extract DIR    Extraer sin instalar (portable)"
    say "  --help           Esta ayuda"
}

# ---------- extraer el payload embebido --------------------------------
TMPDIR_RUN="$(mktemp -d "${TMPDIR:-/tmp}/cufw-install.XXXXXX")"
cleanup() { rm -rf "$TMPDIR_RUN"; }
trap cleanup EXIT INT TERM

tail -n +"$SKIP" "$0" | tar xzf - --no-same-owner -C "$TMPDIR_RUN" || {
    say "ERROR: no se pudo extraer el instalador (fichero corrupto o truncado)."
    exit 1
}
SRC="$TMPDIR_RUN/payload"
[ -d "$SRC" ] || { say "ERROR: paquete interno incompleto."; exit 1; }

# ---------- opciones -----------------------------------------------------
MODE=""
EXTRACT_DIR=""
UNINSTALL_USER=0
for arg in "$@"; do
    case "$arg" in
        --user) MODE="user" ;;
        --uninstall) MODE="uninstall" ;;
        --uninstall-user|--user-uninstall) MODE="uninstall"; UNINSTALL_USER=1 ;;
        --extract) ;;
        --extract=*) EXTRACT_DIR="${arg#*=}"; MODE="extract" ;;
        --help|-h) show_help; exit 0 ;;
        *) if [ -z "$EXTRACT_DIR" ] && [ "$MODE" = "extract" ]; then EXTRACT_DIR="$arg"; fi ;;
    esac
done

# «--extract DIR» con la ruta como siguiente argumento
if [ "${1:-}" = "--extract" ] && [ -n "${2:-}" ]; then MODE="extract"; EXTRACT_DIR="$2"; fi

uninstall_from_prefix() {
    P="$1"
    rm -f  "$P/bin/$APP"
    rm -rf "$P/lib/$APP"
    rm -f  "$P/share/applications/$APP.desktop"
    for size in 256x256 128x128 64x64 48x48 32x32; do
        rm -f "$P/share/icons/hicolor/$size/apps/$APP.png"
    done
    rm -rf "$P/share/doc/$APP"
    rm -rf "$P/lib/locale/ufw-sentinel"
}

refresh_caches() {
    P="$1"
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$P/share/applications" 2>/dev/null || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f "$P/share/icons/hicolor" 2>/dev/null || true
    fi
}

install_files() {
    DEST="$1"
    say "== Centinela UFW v$VER: instalando en $DEST ..."
    install -D -m 755 "$SRC/bin/$APP" "$DEST/bin/$APP"
    # librerías gráficas embebidas
    rm -rf "$DEST/lib/$APP"
    mkdir -p "$DEST/lib/$APP"
    for f in "$SRC"/lib/$APP/*; do
        base="$(basename "$f")"
        if [ -L "$f" ]; then
            ln -sf "$(readlink "$f")" "$DEST/lib/$APP/$base"
        else
            install -m 644 "$f" "$DEST/lib/$APP/$base"
        fi
    done
    # lanzador del menú (con ruta absoluta al ejecutable)
    install -D -m 644 "$SRC/share/applications/$APP.desktop" \
        "$DEST/share/applications/$APP.desktop"
    sed -i "s|^Exec=.*|Exec=$DEST/bin/$APP|" "$DEST/share/applications/$APP.desktop"
    sed -i "s|^TryExec=.*|TryExec=$DEST/bin/$APP|" "$DEST/share/applications/$APP.desktop" 2>/dev/null || true
    # iconos
    for size in 256x256 128x128 64x64 48x48 32x32; do
        base="${size%%x*}"
        [ -f "$SRC/share/icons/hicolor/$size/apps/$APP.png" ] && \
        install -D -m 644 "$SRC/share/icons/hicolor/$size/apps/$APP.png" \
            "$DEST/share/icons/hicolor/$size/apps/$APP.png"
    done
    # documentación
    mkdir -p "$DEST/share/doc/$APP"
    for f in README.txt LICENSE; do
        [ -f "$SRC/share/doc/$APP/$f" ] && install -m 644 "$SRC/share/doc/$APP/$f" "$DEST/share/doc/$APP/$f"
    done
    # política de PolicyKit (solo si se puede escribir; en --user se omite)
    if [ -w /usr/share/polkit-1 ] || [ "$(id -u)" = "0" ]; then
        install -D -m 644 "$SRC/share/polkit-1/actions/com.ufwsentinel.ufw.policy" \
            /usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy 2>/dev/null || true
    fi
}

# ---------- desinstalar ---------------------------------------------------
if [ "$MODE" = "uninstall" ]; then
    if [ "$UNINSTALL_USER" = "1" ]; then
        say "== Desinstalando la instalación por usuario (~/.local) ..."
        uninstall_from_prefix "$HOME/.local"
        refresh_caches "$HOME/.local"
        say "Hecho."
        exit 0
    fi
    if [ "$(id -u)" != "0" ]; then
        say "ERROR: la desinstalación del sistema requiere root. Usa: sudo $0 --uninstall"
        say "       (o «$0 --uninstall --user» para la instalación por usuario)."
        exit 1
    fi
    say "== Desinstalando la instalación del sistema (/usr/local y /usr/share) ..."
    uninstall_from_prefix /usr/local
    rm -f /usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy
    rm -f /usr/share/applications/$APP.desktop
    for size in 256x256 128x128 64x64 48x48 32x32; do
        rm -f /usr/share/icons/hicolor/$size/apps/$APP.png
    done
    refresh_caches /usr/share
    say "Hecho. (Tu configuración y tu informe en ~/.config/ufw-sentinel/ y"
    say "~/.local/share/ufw-sentinel/ se conservan.)"
    exit 0
fi

# ---------- extraer sin instalar ------------------------------------------
if [ "$MODE" = "extract" ]; then
    [ -n "$EXTRACT_DIR" ] || { say "Uso: $0 --extract DIR"; exit 2; }
    mkdir -p "$EXTRACT_DIR"
    cp -a "$SRC/." "$EXTRACT_DIR/"
    say "Ficheros extraídos en: $EXTRACT_DIR"
    say "Para ejecutar:  $EXTRACT_DIR/bin/$APP"
    exit 0
fi

# ---------- instalar -------------------------------------------------------
if [ "$MODE" = "user" ]; then
    install_files "$HOME/.local"
    refresh_caches "$HOME/.local"
    say ""
    say "Instalación por usuario completada en \$HOME/.local."
    say "  · Ejecutable: \$HOME/.local/bin/$APP"
    say "  · Lanzador:   «Centinela UFW» en el menú (puede requerir cerrar sesión)"
    say "  · Nota: la política PolicyKit es del sistema y no se instala en modo"
    say "    usuario; instálala desde la propia app (pestaña Avanzado) si quieres"
    say "    menos contraseñas."
    say "Desinstalar:  $0 --uninstall --user"
    exit 0
fi

if [ "$(id -u)" != "0" ]; then
    say "ERROR: la instalación del sistema requiere root."
    say ""
    say "Usa una de estas opciones:"
    say "  sudo $0            instala en el sistema"
    say "  $0 --user          instala solo para tu usuario (sin root)"
    say "  $0 --extract DIR   solo extraer los ficheros"
    exit 1
fi

install_files /usr/local
refresh_caches /usr/share
say ""
say "Instalación del sistema completada."
say "  · Ejecutable: /usr/local/bin/$APP"
say "  · Librerías:  /usr/local/lib/ufw-sentinel (embebidas, sin dependencias)"
say "  · Lanzador:   «Centinela UFW» en el menú"
say "  · Política:   /usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy"
say "Desinstalar:  sudo $0 --uninstall"
exit 0
