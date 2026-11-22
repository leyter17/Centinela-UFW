════════════════════════════════════════════════════════════════════════
  EMPAQUETADO DE CENTINELA UFW (carpeta packaging/)
════════════════════════════════════════════════════════════════════════

Esta carpeta contiene todo lo necesario para generar los tres paquetes
portables de Centinela UFW: .deb, .AppImage y .run. Los tres llevan
embebidas las librerías gráficas que la app carga en tiempo de ejecución
(stack de X11, xkbcommon y Wayland), así que los paquetes solo necesitan
ufw y las librerías básicas del escritorio (glibc 2.35+, OpenGL).

CONTENIDO
  build-all.sh              script maestro: genera los tres paquetes y el
                            fichero SHA256SUMS.txt con sus firmas.
  run-header.sh             cabecera del instalador autoextraíble .run
                            (instalación sistema / --user / --uninstall /
                            --extract).
  AppRun                    lanzador interno del AppImage (prepara
                            LD_LIBRARY_PATH y ejecuta el binario).
  DEBIAN/control            metadatos del paquete .deb.
  DEBIAN/postinst, postrm   scripts de instalación/limpieza del .deb.
  ufw-sentinel.desktop      lanzador del menú de aplicaciones.
  polkit/com.ufwsentinel.ufw.policy   política de autenticación.
  libs/                     librerías gráficas embebidas (x86_64, extraídas
                            de Debian 13 «trixie»: X11, xkbcommon, Wayland,
                            xcb y libffi) con sus enlaces .so sin versión,
                            que son los que buscan x11-dl y xkbcommon-dl al
                            cargar por dlopen.
  LICENSE                   texto completo de la GNU GPL v3.

CÓMO COMPILAR Y EMPAQUETAR
  1. cargo build --release        (build.rs añade el RPATH $ORIGIN/../lib/
                                   ufw-sentinel y neutraliza los símbolos
                                   pidfd_* para glibc 2.35+)
  2. packaging/build-all.sh [salida]
     Requiere: dpkg-deb, mksquashfs (squashfs-tools) y appimagetool
     (AppImageKit; si no está en el PATH, apunta la variable AI_TOOL_DIR a
     la carpeta extraída del AppImage de appimagetool, o define
     RUNTIME_FILE con el runtime-x86_64 para la construcción manual).

DISPOSICIÓN INSTALADA (compartida por los tres formatos)
  bin/ufw-sentinel                  ejecutable (RPATH $ORIGIN/../lib/…)
  lib/ufw-sentinel/*.so*            librerías embebidas
  share/applications/….desktop      lanzador del menú
  share/icons/hicolor/*/apps/….png  iconos (256/128/64/48/32)
  share/polkit-1/actions/….policy   política de autenticación
  share/doc/ufw-sentinel/           README.txt y LICENSE

Prefijos según el formato:
  .deb       → /usr (bin, lib, share)
  .AppImage  → autónomo (usr/ dentro de la imagen; AppRun ajusta todo)
  .run       → /usr/local (sistema) o ~/.local (--user)

NOTA DE COMPATIBILIDAD
  El binario se enlaza dinámicamente solo contra glibc (libc/libm/
  libgcc_s): las librerías de X11/Wayland/GL se cargan en tiempo de
  ejecución con dlopen. Las del stack X11/xkbcommon/Wayland van embebidas
  en lib/ufw-sentinel; el OpenGL lo aporta siempre el escritorio.
════════════════════════════════════════════════════════════════════════
