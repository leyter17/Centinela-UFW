#!/bin/bash
# ══════════════════════════════════════════════════════════════════════
#  build-all.sh — genera los tres paquetes portables de Centinela UFW:
#    · Centinela-UFW_<ver>_amd64.deb      (Debian/Ubuntu/Mint/Pop!_OS…)
#    · Centinela-UFW_<ver>_x86_64.AppImage (universal, sin instalación)
#    · Centinela-UFW_<ver>_instalador.run  (cualquier distro, con root o --user)
#  Requisitos: binario compilado en target/release (cargo build --release),
#  dpkg-deb, mksquashfs (squashfs-tools) y appimagetool para el AppImage.
#  Uso:  packaging/build-all.sh [carpeta-de-salida]
# ══════════════════════════════════════════════════════════════════════
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKGS="$ROOT/packaging"
VER="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
BIN="$ROOT/target/release/ufw-sentinel"
DEB_ARCH="amd64"
AI_ARCH="x86_64"
OUT="${1:-$ROOT/dist}"
# Rutas alternativas donde buscar la herramienta appimagetool extraída
AI_TOOL_DIR="${AI_TOOL_DIR:-$ROOT/../appimagetool-extracted}"

[ -x "$BIN" ] || { echo "ERROR: falta el binario $BIN (ejecuta: cargo build --release)"; exit 1; }
NAME="Centinela-UFW_${VER}"
echo "== Centinela UFW v$VER — empaquetado portable"

# ─── payload compartido: bin/ + lib/ + share/ ───────────────────────────
PAY="$OUT/payload"
rm -rf "$PAY"
install -D -m 755 "$BIN" "$PAY/bin/ufw-sentinel"
# librerías gráficas embebidas (X11/xkbcommon/Wayland: cargadas por dlopen)
rm -rf "$PAY/lib/ufw-sentinel"
mkdir -p "$PAY/lib/ufw-sentinel"
cp -a "$PKGS/libs/." "$PAY/lib/ufw-sentinel/"
# lanzador del menú
install -D -m 644 "$PKGS/ufw-sentinel.desktop" "$PAY/share/applications/ufw-sentinel.desktop"
# iconos
for size in 256x256 128x128 64x64 48x48 32x32; do
    base="${size%%x*}"
    install -D -m 644 "$ROOT/assets/icon_${base}.png" \
        "$PAY/share/icons/hicolor/$size/apps/ufw-sentinel.png"
done
# política PolicyKit
install -D -m 644 "$PKGS/polkit/com.ufwsentinel.ufw.policy" \
    "$PAY/share/polkit-1/actions/com.ufwsentinel.ufw.policy"
# documentación
install -D -m 644 "$ROOT/README.txt" "$PAY/share/doc/ufw-sentinel/README.txt"
install -D -m 644 "$PKGS/LICENSE" "$PAY/share/doc/ufw-sentinel/LICENSE"
echo "   payload listo ($(du -sh "$PAY" | cut -f1))"

# ─── 1) paquete .deb ─────────────────────────────────────────────────────
DEBROOT="$OUT/deb-root"
rm -rf "$DEBROOT"
mkdir -p "$DEBROOT/DEBIAN" "$DEBROOT/usr"
cp -a "$PAY/." "$DEBROOT/usr/"
SIZE_KB="$(du -sk "$DEBROOT/usr" | cut -f1)"
sed -e "s/@INSTALLED_SIZE@/$SIZE_KB/" -e "s/@VER@/$VER/" "$PKGS/DEBIAN/control" > "$DEBROOT/DEBIAN/control"
sed -e "s/@VER@/$VER/" "$PKGS/DEBIAN/postinst" > "$DEBROOT/DEBIAN/postinst"
cp "$PKGS/DEBIAN/postrm" "$DEBROOT/DEBIAN/postrm"
chmod 755 "$DEBROOT/DEBIAN/postinst" "$DEBROOT/DEBIAN/postrm"
chmod 644 "$DEBROOT/DEBIAN/control"
(cd "$OUT" && dpkg-deb --root-owner-group --build deb-root "${NAME}_${DEB_ARCH}.deb" >/dev/null)
rm -rf "$DEBROOT"
echo "   .deb generado: ${NAME}_${DEB_ARCH}.deb"

# ─── 2) AppImage ──────────────────────────────────────────────────────────
APPDIR="$OUT/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr"
cp -a "$PAY/." "$APPDIR/usr/"
cp "$PKGS/ufw-sentinel.desktop" "$APPDIR/ufw-sentinel.desktop"
install -m 755 "$PKGS/AppRun" "$APPDIR/AppRun"
install -m 644 "$ROOT/assets/icon_256.png" "$APPDIR/ufw-sentinel.png"
ln -sf ufw-sentinel.png "$APPDIR/.DirIcon"
if command -v appimagetool >/dev/null 2>&1; then
    AI_TOOL="appimagetool"
elif [ -x "$AI_TOOL_DIR/AppRun" ]; then
    AI_TOOL="$AI_TOOL_DIR/AppRun"
else
    AI_TOOL=""
fi
if [ -n "$AI_TOOL" ]; then
    "$AI_TOOL" --no-appstream "$APPDIR" "$OUT/${NAME}_${AI_ARCH}.AppImage" >/dev/null 2>&1 \
        || "$AI_TOOL" --no-appstream "$APPDIR" "$OUT/${NAME}_${AI_ARCH}.AppImage"
else
    # construcción manual: mksquashfs + runtime de AppImageKit
    [ -n "${RUNTIME_FILE:-}" ] && [ -f "$RUNTIME_FILE" ] || {
        echo "ERROR: no hay appimagetool ni RUNTIME_FILE (runtime-x86_64) para el AppImage"; exit 1; }
    mksquashfs "$APPDIR" "$OUT/${NAME}_${AI_ARCH}.AppImage" -comp zstd -noappend -all-root >/dev/null
    cat "$RUNTIME_FILE" >> "$OUT/${NAME}_${AI_ARCH}.AppImage"
    chmod +x "$OUT/${NAME}_${AI_ARCH}.AppImage"
fi
rm -rf "$APPDIR"
echo "   AppImage generado: ${NAME}_${AI_ARCH}.AppImage"

# ─── 3) instalador .run autoextraíble ─────────────────────────────────────
HDR="$OUT/run-header.tmp"
sed -e "s/@VER@/$VER/" "$PKGS/run-header.sh" > "$HDR"
chmod 755 "$HDR"
TGZ="$OUT/payload.tar.gz"
tar -C "$OUT" -czf "$TGZ" "$(basename "$PAY")"
SKIP=$(( $(wc -l < "$HDR") + 1 ))
sed -i "s/@SKIP@/$SKIP/" "$HDR"
cat "$HDR" "$TGZ" > "$OUT/${NAME}_instalador.run"
chmod 755 "$OUT/${NAME}_instalador.run"
rm -f "$HDR" "$TGZ"
echo "   .run generado: ${NAME}_instalador.run"

# ─── comprobación de integridad de los tres paquetes ──────────────────────
rm -rf "$PAY"
cd "$OUT"
sha256sum "${NAME}_${DEB_ARCH}.deb" "${NAME}_${AI_ARCH}.AppImage" "${NAME}_instalador.run" > SHA256SUMS.txt
echo "== SHA256SUMS.txt generado:"
cat SHA256SUMS.txt
echo "== Empaquetado completado en: $OUT"
