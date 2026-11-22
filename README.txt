════════════════════════════════════════════════════════════════════════
  CENTINELA UFW  ·  UFW SENTINEL  ·  versión 1.3.0
  Administrador gráfico del firewall UFW para Linux
════════════════════════════════════════════════════════════════════════

Fecha de esta versión: 21 de noviembre de 2026
Creado por: Leyter Vázquez . V
Licencia: GNU GPL v3 (GNU General Public License, versión 3)
Idiomas: Español (por defecto) e Inglés
Escritorios: GNOME · KDE Plasma · COSMIC · XFCE
Arquitectura: x86_64 (64 bits)

────────────────────────────────────────────────────────────────────────
 1. ¿QUÉ ES ESTA APP?
────────────────────────────────────────────────────────────────────────

Centinela UFW es una aplicación de escritorio escrita en Rust que permite
gestionar el firewall UFW (Uncomplicated Firewall) de forma sencilla pero
con opciones avanzadas y profesionales. Toda la gestión se hace desde una
interfaz moderna en tonos verdes, pensada para que cualquier persona
pueda proteger su equipo sin memorizar comandos, y para que un usuario
avanzado no se quede corto: políticas por defecto, límite de conexiones,
perfiles preconfigurados, perfiles de aplicación, IPv6, niveles de
registro (logging), modo simulación, informe de actividad y más.

La aplicación NUNCA necesita ejecutarse como root: los comandos
privilegiados se piden al sistema mediante PolicyKit (pkexec), que
muestra el diálogo de contraseña de tu propio escritorio.

Novedades 1.3.0: en la pestaña «Informe» hay un nuevo botón «Borrar
registro automático» que vacía el fichero que la app guarda en disco
(informe.log y su rotación informe.log.1) y la vista de la sesión, con
confirmación previa; el registro sigue funcionando desde ese momento.
Además se eliminó de raíz el falso error «ERROR: You need to be root to
run this script» que aparecía en el registro de actividad y en el
informe: es la respuesta NORMAL de ufw cuando algo consulta el estado
sin privilegios, no un fallo de la app, y ahora se detecta y se escala
por pkexec en silencio (con la contraseña bien escrita el estado se
carga sin ningún error; secciones 4, 7 y 9).

Novedades 1.2.0: el INFORME (todas las acciones y errores de la app,
exportable a .txt) añade un NIVEL DE DETALLE ajustable — «Completo»
registra todo y «Solo errores» muestra, copia y exporta únicamente los
errores. La elección se recuerda entre sesiones y el fichero automático
en disco conserva siempre el historial íntegro (sección 4 y 7).

Novedades 1.1.0: todas las acciones y errores de la app quedan guardados
en un INFORME completo, visible en su propia pestaña y exportable a un
fichero .txt con un clic (sección 4 y 7 de este documento).

────────────────────────────────────────────────────────────────────────
 2. AUTOR Y LICENCIA (GNU GPL v3)
────────────────────────────────────────────────────────────────────────

Autor: ............ Leyter Vázquez . V
Licencia: ......... GNU GPL v3
Texto completo: ... fichero LICENSE que acompaña a esta app
                  y https://www.gnu.org/licenses/gpl-3.0.html

Este programa es software libre: puedes redistribuirlo y/o modificarlo
bajo los términos de la Licencia Pública General de GNU (GNU GPL)
versión 3, publicada por la Free Software Foundation.

Este programa se distribuye con la esperanza de que sea útil, pero SIN
NINGUNA GARANTÍA; ni siquiera la garantía implícita de COMERCIALIZACIÓN
o de IDONEIDAD PARA UN PROPÓSITO PARTICULAR. Consulta la GNU GPL v3
para más detalles.

Esta información también aparece dentro de la app, en la sección
«Acerca de».

────────────────────────────────────────────────────────────────────────
 3. CARACTERÍSTICAS PRINCIPALES
────────────────────────────────────────────────────────────────────────

  • Interfaz gráfica moderna, rápida y en color verde (tema oscuro).
  • Activar / desactivar el firewall con un solo clic (con confirmación).
  • Políticas por defecto editables: entrante, saliente y reenviado
    (deny / allow / reject).
  • Reglas: creación con asistente y previsualización del comando exacto
    que se ejecutará, listado numerado del estado real de UFW (con
    marcado de reglas IPv6) y borrado individual. Acciones: permitir
    (allow), denegar (deny), rechazar (reject) y limitar (limit, anti
    fuerza bruta).
  • Direcciones: entrante (in), saliente (out) y reenviado (route).
  • Protocolos: TCP, UDP o ambos. Puertos simples (22), listas
    (80,443) y rangos (5000:5010), con validación (1-65535).
  • Origen/destino: direcciones IP o redes (por ejemplo 192.168.1.0/24).
  • Comentarios en las reglas (se guardan junto a la regla de UFW).
  • Perfiles preconfigurados: Hogar, Oficina, Servidor web y
    Red pública, con resumen de comandos antes de aplicar.
  • Perfiles de aplicación de UFW (OpenSSH, Samba, Nginx, Apache…).
  • Nivel de registro (logging): off / low / medium / high / full.
  • IPv6: activar o desactivar (edita /etc/default/ufw y recarga).
  • Política de autenticación PolicyKit opcional: registra UFW para no
    tener que escribir la contraseña en cada acción (recuerda 5 min).
  • Recargar UFW y restablecer el firewall a fábrica (zona de riesgo
    con confirmación explícita).
  • Modo SIMULACIÓN (dry run): muestra todos los comandos en el registro
    de actividad SIN ejecutarlos nunca. Ideal para aprender o revisar.
  • Bandeja del sistema (system tray) con icono que cambia según el
    estado del firewall y menú rápido (mostrar/ocultar ventana,
    activar, desactivar y salir).
  • INFORME: pestaña «Informe» con el registro completo y persistente
    de todo lo que hace la app y de los errores que suceden, con detalle
    (comando ejecutado, código de salida, salida, respuesta del sistema).
    Se guarda automáticamente en disco, se puede EXPORTAR a un fichero
    .txt con un clic, copiar al portapapeles y BORRAR (nuevo en 1.3.0).
  • NUEVO · 1.3.0: botón «Borrar registro automático» en la pestaña
    Informe: elimina el informe.log en disco (y su rotación
    informe.log.1) y vacía la vista de la sesión, con confirmación
    previa; el registro sigue activo a partir de ese momento.
  • Selector de idioma español/inglés al instante; el idioma también
    se recuerda entre sesiones.
  • Registro de actividad con color, marcas de tiempo y borrado.
  • Detección del escritorio (GNOME, KDE, COSMIC, XFCE) y del sistema.
  • Modo demostración opcional (--demo) para probar la interfaz sin
    tocar el firewall.

────────────────────────────────────────────────────────────────────────
 4. FUNCIONES DETALLADAS (PESTAÑA POR PESTAÑA)
────────────────────────────────────────────────────────────────────────

PANEL (Dashboard)
  • Estado grande del firewall: ACTIVO / INACTIVO con punto pulsante.
  • Botón Activar / Desactivar (desactivar pide confirmación).
  • Políticas por defecto: Entrante, Saliente y Reenviado.
  • Ajustes rápidos: modo «Simulación».
  • Fichas de resumen: reglas activas, nivel de registro, estado de
    IPv6 y escritorio detectado.

REGLAS
  • Formulario con Acción, Dirección, Protocolo, Puertos, Desde, Hacia
    y Comentario.
  • Previsualización del comando en vivo, por ejemplo:
      ufw allow in proto tcp from 192.168.1.0/24 to any port 22
  • Validación: puertos correctos (1-65535), listas/rangos requieren
    protocolo, direcciones sin espacios y no se permiten reglas
    «vacías» sin puerto ni dirección.
  • Tabla del estado REAL de UFW (ufw status numbered) con el número de
    cada regla, acción coloreada, origen, marcado (v6) y comentario.
  • Botón Eliminar por regla (con confirmación).

PERFILES
  • Hogar: bloquea entrante, permite saliente y confía en la red local
    192.168.0.0/16.
  • Oficina: como Hogar + SSH limitado a 6 intentos cada 30 s.
  • Servidor web: SSH limitado y puertos 80 y 443 abiertos.
  • Red pública: máxima protección, bloquea todo lo entrante.
  • Cada perfil muestra la lista exacta de comandos antes de aplicar.
  • Perfiles de aplicación de UFW: lista «ufw app list» y botón
    Permitir por aplicación.

AVANZADO
  • Sistema: ruta de UFW, versión, pkexec, escritorio y política polkit.
  • Botón para instalar la política de autenticación (menos contraseñas).
  • Nivel de registro con selección directa.
  • IPv6: estado actual y botones activar/desactivar.
  • Zona de riesgo: Recargar UFW y Restablecer todo el firewall
    (con doble confirmación).

INFORME
  • Registro completo de todo lo que hace la aplicación: arranque,
    detección del sistema, comandos ejecutados (con su código de
    salida, stdout y stderr), operaciones, ajustes guardados, avisos y
    errores con todo el detalle.
  • Se guarda automáticamente en disco mientras usas la app, con
    rotación a los 5 MiB (se conserva el historial anterior en
    informe.log.1).
  • Botón «Exportar informe (.txt)»: vuelca TODO el historial (todas
    las sesiones) a un fichero de texto legible; la ruta de destino se
    puede editar (por defecto, tu carpeta personal con fecha y hora).
  • Botón «Copiar al portapapeles» para pegar el informe donde quieras.
  • NUEVO · 1.3.0: botón «Borrar registro automático»: tras confirmar,
    borra el fichero automático en disco (informe.log y su rotación
    informe.log.1) y vacía la vista de la sesión. El registro sigue
    activo y los eventos posteriores se escriben desde cero. La acción
    no se puede deshacer, por eso pide confirmación.
  • NIVEL DE DETALLE (1.2.0): «Completo» registra y muestra
    todas las operaciones; «Solo errores» muestra, copia y exporta
    únicamente los eventos de error. La elección se guarda entre
    sesiones; el fichero automático en disco conserva SIEMPRE el
    historial completo (el filtro afecta solo a la vista, a la copia
    y a la exportación).

ACERCA DE
  • Logo, nombre, versión y lema de la app.
  • «Creado por Leyter Vázquez . V».
  • Licencia GNU GPL v3 con resumen legal y enlace al texto completo.
  • Escritorios compatibles y detección del escritorio actual.
  • Novedades de la versión.

BANDEJA DEL SISTEMA
  • Icono verde (firewall activo) o gris (inactivo).
  • Clic izquierdo en el icono: mostrar / ocultar la ventana.
  • Menú: Mostrar/Ocultar ventana · Activar firewall · Desactivar
    firewall · Salir.

REGISTRO DE ACTIVIDAD (parte inferior)
  • Cada comando y respuesta aparece con color y marca de tiempo.
  • Botón Limpiar.

────────────────────────────────────────────────────────────────────────
 5. REQUISITOS DEL SISTEMA
────────────────────────────────────────────────────────────────────────

  • Linux x86_64 (Debian 12+, Ubuntu 22.04+, Linux Mint 21+, Pop!_OS,
    Fedora 36+, Arch, openSUSE, Manjaro, NixOS y cualquier distribución
    moderna con glibc 2.35 o superior).
  • Escritorio GNOME, KDE Plasma, COSMIC o XFCE (funciona igualmente en
    otros escritorios basados en X11/Wayland).
  • UFW instalado:  sudo apt install ufw   (Debian/Ubuntu)
                    sudo dnf install ufw   (Fedora)
                    sudo pacman -S ufw     (Arch)
  • PolicyKit/pkexec (incluido por defecto en todos los escritorios
    citados).
  • NADA MÁS: los tres paquetes (deb, AppImage y run) llevan embebidas
    en su interior las librerías gráficas que la app carga en tiempo de
    ejecución (stack de X11, xkbcommon y Wayland), así que no requieren
    instalar dependencias adicionales. Del sistema solo se usan las
    librerías básicas (glibc 2.35+), el soporte OpenGL de tu escritorio
    y, si usas Wayland, sus librerías del sistema: presentes en
    cualquier instalación de escritorio.

────────────────────────────────────────────────────────────────────────
 6. INSTALACIÓN SEGÚN EL FORMATO
────────────────────────────────────────────────────────────────────────

Los tres paquetes son PORTABLES e independientes: llevan la aplicación,
sus librerías gráficas embebidas, el lanzador, los iconos y la política
de PolicyKit. Solo necesitan ufw instalado en el sistema.

A) PAQUETE .deb  (recomendado en Debian, Ubuntu, Mint, Pop!_OS…)
     sudo apt install ./Centinela-UFW_1.3.0_amd64.deb
   o doble clic desde el gestor de paquetes de tu escritorio.
   Instala el ejecutable en /usr/bin (con las librerías embebidas en
   /usr/lib/ufw-sentinel), crea el lanzador en el menú, los iconos y la
   política de autenticación polkit.
   Desinstalar:  sudo apt remove ufw-sentinel

B) APPIMAGE  (universal, sin instalación)
     chmod +x Centinela-UFW_1.3.0_x86_64.AppImage
     ./Centinela-UFW_1.3.0_x86_64.AppImage
   Ejecuta directamente. Si tu escritorio tiene AppImageLauncher, se
   integrará en el menú automáticamente. Si no tienes FUSE disponible,
   puedes extraer y ejecutar con:
     ./Centinela-UFW_1.3.0_x86_64.AppImage --appimage-extract-and-run
   Desinstalar: borrar el fichero .AppImage.

C) INSTALADOR .run  (cualquier distribución, autoextraíble)
     chmod +x Centinela-UFW_1.3.0_instalador.run
     sudo ./Centinela-UFW_1.3.0_instalador.run
   Instala en /usr/local/bin (con las librerías embebidas en
   /usr/local/lib/ufw-sentinel), los iconos, el lanzador del menú y la
   política polkit. Opciones adicionales:
     ./Centinela-UFW_1.3.0_instalador.run --user
        instalación por usuario sin root (en ~/.local)
     sudo ./Centinela-UFW_1.3.0_instalador.run --uninstall
        desinstalar la instalación del sistema
     ./Centinela-UFW_1.3.0_instalador.run --uninstall --user
        desinstalar la instalación por usuario
     ./Centinela-UFW_1.3.0_instalador.run --extract DIR
        solo extraer los ficheros a una carpeta (portable)
     ./Centinela-UFW_1.3.0_instalador.run --help
        ayuda del instalador

DESCARGAS · VERIFICACIÓN DE INTEGRIDAD:
  Junto a los tres paquetes se distribuye un fichero SHA256SUMS.txt con
  las firmas SHA-256 de cada entregable. Puedes comprobar la integridad
  de lo descargado con:
     sha256sum -c SHA256SUMS.txt

DESINSTALAR:
  • .deb:      sudo apt remove ufw-sentinel
  • .run:      sudo ./Centinela-UFW_1.3.0_instalador.run --uninstall
  • AppImage:  borrar el archivo .AppImage

────────────────────────────────────────────────────────────────────────
 7. USO RÁPIDO
────────────────────────────────────────────────────────────────────────

  1. Abre «Centinela UFW» desde el menú de aplicaciones o ejecuta
     «ufw-sentinel» en una terminal.
  2. La app comprueba el estado del firewall al arrancar. UFW exige
     privilegios de root incluso para CONSULTAR el estado (es su
     comportamiento normal), así que lo habitual es que PolicyKit pida
     la contraseña una vez al arrancar o al actualizar; si instalas la
     política de autenticación de la app (pestaña Avanzado, un clic),
     PolicyKit la recordará durante 5 minutos y no volverá a pedirla
     en cada acción.
  3. Pulsa ACTIVAR FIREWALL si estaba inactivo.
  4. Aplica el perfil que encaje contigo (Hogar, por ejemplo).
  5. Añade reglas concretas desde la pestaña Reglas.
  6. En cualquier momento, abre la pestaña INFORME para ver todo lo
     que ha hecho la app y sus errores; desde ahí puedes exportarlo a
     un .txt, copiarlo al portapapeles o borrar el registro automático
     guardado en disco.

CONSEJOS:
  • Activa el modo SIMULACIÓN antes de tocar nada para ver, sin riesgo
    (no se ejecuta nada), qué comandos se ejecutarían.
  • Si te pide la contraseña demasiado a menudo, instala la política
    de autenticación en Avanzado (un solo clic).
  • Si quieres dejar el informe en blanco, usa el botón «Borrar
    registro automático» de la pestaña Informe: borra informe.log y su
    rotación informe.log.1; la app sigue registrando desde ese momento.
  • La configuración (idioma, simulación, nivel de detalle del informe)
    se guarda en ~/.config/ufw-sentinel/config.json.
  • El informe automático se guarda en
    ~/.local/share/ufw-sentinel/informe.log.

OPCIONES DE LÍNEA DE COMANDOS:
  ufw-sentinel              Interfaz gráfica normal
  ufw-sentinel --version    Versión, autor y licencia
  ufw-sentinel --help       Ayuda
  ufw-sentinel --demo       Demo sin tocar el firewall (no requiere root)
  ufw-sentinel --autotour   Demo + recorrido automático de pestañas
                            (uso interno/pruebas)
  ufw-sentinel --simflow    Flujo completo de prueba contra ufw real o
                            simulado (uso interno/pruebas)

────────────────────────────────────────────────────────────────────────
 8. NOTAS POR ESCRITORIO
────────────────────────────────────────────────────────────────────────

  GNOME:    Todo funciona. La bandeja del sistema requiere tener
            activada la extensión «AppIndicator and KStatusNotifierItem
            Support» (Ubuntu la trae de serie).
  KDE:      Todo funciona de serie, incluida la bandeja.
  COSMIC:   Todo funciona; la bandeja aparece en el «Área de estado»
            del panel de COSMIC.
  XFCE:     Todo funciona de serie, incluida la bandeja
            (elemento «Área de notificación» del panel).

Si la bandeja no está disponible, la app avisa en el registro de
actividad y en el informe, y sigue funcionando con normalidad (el
cierre de la ventana finalizará la aplicación).

────────────────────────────────────────────────────────────────────────
 9. SEGURIDAD Y PERMISOS
────────────────────────────────────────────────────────────────────────

  • La app se ejecuta SIEMPRE como tu usuario, nunca como root.
  • Cada cambio del firewall se ejecuta con «pkexec ufw …», que pide
    autenticación con el diálogo nativo de tu escritorio.
  • Las consultas de estado: UFW exige root incluso para consultar el
    estado y responde «You need to be root to run this script» cuando
    se consulta sin privilegios. Esa respuesta es el comportamiento
    NORMAL de ufw, no un error de la app: Centinela UFW la detecta y
    repite la consulta mediante pkexec SIN registrar ningún error
    (desde la 1.3.0; en versiones anteriores esa respuesta aparecía
    por error como ERROR en el registro y en el informe).
  • Opcionalmente puedes instalar la política de autenticación de
    Centinela UFW (fichero
    /usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy): PolicyKit
    recordará tu contraseña 5 minutos entre acciones, igual que otras
    herramientas del sistema.
  • El modo SIMULACIÓN no ejecuta nada: solo muestra comandos.
  • El informe se guarda solo en tu equipo (nunca sale de él); la app
    no tiene red y nunca envía información a internet.

────────────────────────────────────────────────────────────────────────
 10. REGISTRO DE CAMBIOS (CHANGELOG)
────────────────────────────────────────────────────────────────────────
(Esta sección se actualiza con cada nueva versión de la app.)

v1.3.0 — 21/11/2026 — Borrado del registro automático y depuración a fondo
  • NUEVO: botón «Borrar registro automático» en la pestaña «Informe».
    Borra el fichero automático que la app guarda en disco (informe.log
    y su rotación informe.log.1) y vacía la vista de la sesión, con
    confirmación previa. El registro sigue activo: los eventos
    posteriores se escriben desde cero.
  • CORREGIDO: desaparecieron los falsos «ERROR: You need to be root to
    run this script» del registro de actividad y del informe. La causa
    era que las consultas de estado se intentan primero sin
    privilegios y ufw SIEMPRE responde eso (necesita root incluso para
    consultar); la respuesta se registraba como error aunque luego la
    contraseña se escribiera bien. Ahora se detecta como comportamiento
    esperado, se escala por pkexec en silencio y solo queda una nota de
    detalle en el informe (nunca como ERROR).
  • CORREGIDO: con el modo simulación activado, las consultas de estado
    escaladas por pkexec ya no quedaban vacías (la simulación solo
    afecta a comandos que modifican el firewall).
  • CORREGIDO: el tamaño del registro automático en disco se muestra
    correctamente también en inglés (antes salía «KiB en disco» fijo).
  • Empaquetado renovado: .deb, .AppImage y .run portables con el stack
    gráfico embebido (X11/xkbcommon/Wayland, cargado por dlopen) y
    RPATH $ORIGIN; solo requieren ufw, glibc 2.35+ y el OpenGL del
    escritorio. Se distribuye SHA256SUMS.txt con las firmas de los tres
    entregables (el formato tar.gz se retira; el .run con --extract lo
    sustituye).
  • Mejorado: 0 avisos de clippy; limpieza de código muerto de
    empaquetado (scripts del antiguo tar.gz) y revisión completa del
    motor de ejecución.
  • Verificado con simulaciones completas: recorrido demo de las 6
    pestañas, flujo funcional de 16 pasos (activar, reglas v4+v6,
    perfil Oficina, logging, error intencional, exportación completa y
    de solo errores, borrado del registro automático, borrado real de
    regla, modo simulación) y regresión específica del bug de root como
    usuario normal con contraseña correcta: cero errores en el registro
    y en el informe. Ver Simulacion-Centinela-UFW-1.3.0.txt.

v1.2.0 — 20/11/2026 — Nivel de detalle del informe y repaso profundo
  • NUEVO: nivel de detalle del informe ajustable en la pestaña
    «Informe»: «Solo errores» muestra, copia y exporta únicamente los
    eventos de error; «Completo» registra todo con detalle. La
    elección se recuerda entre sesiones y el registro automático en
    disco conserva siempre el historial íntegro.
  • CORREGIDO: la escritura del registro automático podía perder
    líneas si el fichero estaba ocupado (p. ej. durante la rotación de
    5 MiB); ahora cada evento se escribe siempre.
  • CORREGIDO: los errores de comando aparecían duplicados en el
    «Registro de actividad» y en el informe; ahora aparecen una sola
    vez.
  • CORREGIDO: en los scripts privilegiados (IPv6, política polkit),
    stdout y stderr se concatenaban sin separador, lo que podía fusionar
    palabras y afectar a la detección de «autenticación cancelada».
  • CORREGIDO: el contador del informe usaba plural incorrecto con un
    solo elemento («1 errores» → «1 error» / «1 evento»).
  • Mejorado: 0 avisos de clippy (6 corregidos) y limpieza de código
    muerto (patrones sin usar en validación y en la tabla de reglas).
  • Verificado con simulaciones completas: recorrido demo de las 6
    pestañas y flujo funcional de 15 pasos. Ver
    Simulacion-Centinela-UFW-1.2.0.txt.

v1.1.0 — 20/11/2026 — Repaso integral, informe y correcciones
  • NUEVO: pestaña «Informe» con registro completo y persistente de
    todo lo que hace la app y sus errores (comando, código de salida,
    salida y respuesta del sistema), con exportación a .txt, copia al
    portapapeles y guardado automático en disco con rotación.
  • CORREGIDO: el estado de la app quedaba bloqueado en «Trabajando…»
    y no reflejaba el estado real del firewall.
  • CORREGIDO: el modo simulación volvía a ejecutar comandos reales al
    añadir reglas o aplicar perfiles; ahora nunca ejecuta nada.
  • CORREGIDO: el idioma elegido no se guardaba al cambiarlo desde la
    barra superior; ahora se conserva entre sesiones.
  • CORREGIDO: la instalación de la política polkit no daba ningún
    mensaje de confirmación; ahora avisa al terminar correctamente.
  • CORREGIDO: el parseo de la salida de ufw dependía del idioma del
    sistema; ahora se fuerza salida en inglés (LC_ALL=C) y funciona
    en cualquier idioma.
  • Mejorado: el motor no se detiene ante un error interno (se recupera
    y lo registra en el informe).
  • Mejorado: validación de puertos (1-65535, rangos coherentes) y de
    direcciones; mensajes de error más claros.
  • Mejorado: la tabla de reglas marca las reglas IPv6 como «(v6)».
  • Mejorado: lista de perfiles de aplicación con mensaje de resultado.
  • Eliminada la opción «Minimizar a la bandeja del sistema» (y todo
    su código): minimizar y cerrar la ventana son acciones normales;
    la bandeja se conserva con sus acciones rápidas.
  • Empaquetado portable: los paquetes llevan las librerías gráficas
    embebidas (solo requieren ufw y glibc 2.35+).
  • Código revisado a fondo: eliminado el código muerto y los avisos
    de compilación.

v1.0.0 — 19/11/2026 — Primera versión pública
  • Interfaz gráfica completa en verde con tema oscuro y 5 secciones:
    Panel, Reglas, Perfiles, Avanzado y Acerca de.
  • Gestión completa de UFW: activar/desactivar, políticas por defecto,
    reglas con asistente y borrado, limit (anti fuerza bruta), logging,
    IPv6, recarga y restablecimiento.
  • Perfiles Hogar / Oficina / Servidor web / Red pública.
  • Perfiles de aplicación de UFW.
  • Modo simulación (dry run) con registro de actividad coloreado.
  • Bandeja del sistema con icono dinámico y menú.
  • Idiomas español (por defecto) e inglés, con cambio instantáneo.
  • Instalación de política PolicyKit desde la propia app.
  • Detección de escritorio y adaptación de mensajes.
  • Licencia GNU GPL v3 publicada junto a la app.

────────────────────────────────────────────────────────────────────────
 11. SOLUCIÓN DE PROBLEMAS (FAQ)
────────────────────────────────────────────────────────────────────────

P: La app dice «UFW no está instalado».
R: Instálalo con «sudo apt install ufw» (o el gestor de tu distro) y
   vuelve a abrir la app.

P: Antes veía «ERROR: You need to be root to run this script» en el
   registro aunque pusiera bien la contraseña. ¿Sigue pasando?
R: No: era un error de las versiones 1.1.0 y 1.2.0, corregido en la
   1.3.0. Ese mensaje es la respuesta NORMAL de ufw cuando se consulta
   el estado sin privilegios (ufw exige root incluso para leer el
   estado); la app lo detecta, repite la consulta mediante pkexec y no
   lo registra como error. Si el estado se carga, no verás ningún
   error; si PolicyKit no puede autenticarte (por ejemplo, si cancelas
   el diálogo), verás un aviso de «autenticación cancelada», no un
   error de root.

P: ¿Por qué me pide la contraseña al arrancar o al actualizar el
   estado?
R: UFW exige privilegios de root incluso para CONSULTAR el estado; es
   su comportamiento normal. Instala la política de autenticación de
   la app (pestaña Avanzado, un solo clic) y PolicyKit recordará la
   contraseña durante 5 minutos entre acciones.

P: ¿Cómo borro el registro automático que la app guarda en disco?
R: En la pestaña «Informe», pulsa «Borrar registro automático» y
   confirma. Se borran informe.log y su rotación informe.log.1 (el
   historial completo de todas las sesiones) y se vacía la vista de la
   sesión. La app sigue registrando las operaciones posteriores desde
   cero. La acción no se puede deshacer.

P: ¿Dónde está el informe y qué contiene?
R: En la pestaña «Informe», y guardado automáticamente en
   ~/.local/share/ufw-sentinel/informe.log. Contiene cada operación
   con marca de tiempo, nivel, categoría y detalle (comando exacto,
   código de salida, salida y errores del sistema). El botón
   «Exportar informe (.txt)» vuelca el historial completo a un fichero
   de texto; «Copiar al portapapeles» copia los eventos de la sesión.
   El nivel de detalle (solo errores / completo) afecta a la vista, a
   la copia y a la exportación; el fichero automático en disco
   conserva siempre el historial completo hasta que lo borres con el
   nuevo botón.

P: El AppImage no arranca.
R: Prueba «./Centinela-UFW_1.3.0_x86_64.AppImage --appimage-extract-and-run».
   Asegura tener libX11/Wayland y OpenGL (cualquier escritorio los
   trae).

P: Uso Wayland y el icono de la ventana se ve genérico.
R: Es una limitación de algunos compositores con Wayland; el icono del
   menú y de la bandeja sí es el correcto.

P: ¿Puedo usar la app como root (sudo ufw-sentinel)?
R: No es necesario y no se recomienda. La app ya eleva los comandos
   con pkexec de forma segura.

P: ¿Cómo compruebo que la descarga es correcta?
R: Descarga también SHA256SUMS.txt y ejecuta en la misma carpeta:
   sha256sum -c SHA256SUMS.txt

P: ¿Dónde se guarda mi configuración?
R: En ~/.config/ufw-sentinel/config.json (idioma, modo simulación y
   nivel de detalle del informe). Las reglas del firewall las gestiona
   UFW en /etc/ufw como siempre, y el informe en
   ~/.local/share/ufw-sentinel/.

────────────────────────────────────────────────────────────────────────
 12. CRÉDITOS
────────────────────────────────────────────────────────────────────────

  • Aplicación, diseño e iconos: Leyter Vázquez . V
  • Interfaz gráfica: egui / eframe (MIT/Apache-2.0)
  • Bandeja del sistema: ksni (Unlicense)
  • Firewall: UFW (Uncomplicated Firewall) de Canonical
  • Licencia de esta app: GNU GPL v3

  «Software libre: tu seguridad, bajo tu control.»
════════════════════════════════════════════════════════════════════════
