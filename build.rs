//! Script de compilación.
//!
//! El std precompilado de Rust referencia de forma débil pidfd_spawn,
//! pidfd_spawnp y pidfd_getpid (glibc >= 2.39) para la API
//! CommandExt::create_pidfd, que Centinela UFW no usa. Al enlazar contra una
//! glibc reciente esas referencias débiles registran la versión GLIBC_2.39 y
//! el binario dejaría de cargar en distros con glibc anterior (Ubuntu 22.04,
//! Debian 12…). Definiendo los símbolos como absolutos (dirección 0) la
//! referencia se resuelve sin versión y std los considera no disponibles
//! (mismo comportamiento que en glibc antigua: usa posix_spawn normal).

fn main() {
    // RPATH al directorio privado de librerías del paquete: el binario en
    // <prefijo>/bin encuentra las librerías embebidas en
    // <prefijo>/lib/ufw-sentinel tanto en el .deb (/usr/bin + /usr/lib),
    // en el AppImage (usr/bin + usr/lib), en el .run (/usr/local/bin +
    // /usr/local/lib) como en la instalación por usuario (~/.local).
    // También cubre las librerías que eframe/winit cargan en tiempo de
    // ejecución mediante dlopen (X11, xkbcommon, Wayland). Se usa el RPATH
    // clásico (DT_RPATH, --disable-new-dtags) porque, a diferencia de
    // RUNPATH, también se aplica a las dependencias de las librerías
    // cargadas por dlopen desde el propio binario.
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/ufw-sentinel");
    println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");
    for sym in ["pidfd_spawn", "pidfd_spawnp", "pidfd_getpid"] {
        println!("cargo:rustc-link-arg=-Wl,--defsym,{sym}=0");
    }
}
