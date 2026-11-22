//! Informe de la aplicación: registro completo y persistente de todo lo que
//! hace Centinela UFW, incluidos los errores con todo el detalle.
//!
//! - Cada evento se guarda en memoria (pestaña «Informe») y en disco
//!   (~/.local/share/ufw-sentinel/informe.log), de forma automática.
//! - El fichero en disco acumula todas las sesiones (historial completo).
//! - Se puede exportar todo el historial a un fichero .txt legible.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::ReportDetail;
use crate::i18n::{tr, trf, Lang, K};

// ------------------------------------------------------------------ niveles

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Err,
    Cmd,
    Detail,
}

impl Level {
    pub fn tag(self) -> &'static str {
        match self {
            Level::Info => "INFO ",
            Level::Ok => "OK   ",
            Level::Warn => "AVISO",
            Level::Err => "ERROR",
            Level::Cmd => "CMD  ",
            Level::Detail => "DETAL",
        }
    }
}

// ------------------------------------------------------------------ eventos

#[derive(Clone)]
pub struct Entry {
    /// Segundos desde el epoch (UTC).
    pub ts: u64,
    pub level: Level,
    /// Categoría corta: inicio, ufw, config, bandeja, informe, demo…
    pub cat: String,
    pub msg: String,
}

impl Entry {
    /// Línea «2026-11-20 14:03:22  AVISO  [ufw] mensaje» (multilínea sangrado).
    fn line(&self) -> String {
        let (y, mo, d, h, mi, s) = epoch_to_datetime(self.ts);
        let header = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  {}  [{}] ",
            y,
            mo,
            d,
            h,
            mi,
            s,
            self.level.tag(),
            self.cat
        );
        if self.msg.is_empty() {
            return format!("{header}\n");
        }
        let mut out = String::new();
        for (i, l) in self.msg.lines().enumerate() {
            if i == 0 {
                out.push_str(&header);
            } else if !l.is_empty() {
                out.push_str("        ");
            }
            out.push_str(l);
            out.push('\n');
        }
        out
    }
}

// ------------------------------------------------------------------ informe

/// Límite de rotación del fichero automático (5 MiB).
const MAX_BYTES: u64 = 5 * 1024 * 1024;

pub struct Report {
    entries: Mutex<Vec<Entry>>,
    file: Mutex<Option<File>>,
    pub path: PathBuf,
    bytes: AtomicU64,
}

static REPORT: OnceLock<Report> = OnceLock::new();

/// Inicializa el informe global (idempotente). Crea el directorio de datos
/// y abre el fichero de registro en modo añadido.
pub fn init() {
    let _ = REPORT.get_or_init(|| {
        let dir = dirs::data_dir()
            .or_else(dirs::config_dir)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("ufw-sentinel");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("informe.log");
        let file = open_append(&path);
        Report {
            entries: Mutex::new(Vec::new()),
            file: Mutex::new(file),
            bytes: AtomicU64::new(std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)),
            path,
        }
    });
}

fn open_append(path: &PathBuf) -> Option<File> {
    OpenOptions::new().create(true).append(true).open(path).ok()
}

/// Registro global; None si no se inicializó (CLI --version/--help).
pub fn global() -> Option<&'static Report> {
    REPORT.get()
}

/// Registra un evento en el informe global (no hace nada si no hay informe).
pub fn event(level: Level, cat: &str, msg: impl Into<String>) {
    if let Some(r) = global() {
        r.add(level, cat, msg.into());
    }
}

impl Report {
    /// Añade un evento: memoria + fichero automático (con rotación).
    pub fn add(&self, level: Level, cat: &str, msg: String) {
        let e = Entry {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            level,
            cat: cat.to_string(),
            msg,
        };
        let line = e.line();
        {
            let mut v = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            v.push(e);
            let n = v.len();
            if n > 4000 {
                v.drain(0..n - 4000);
            }
        }
        // fichero automático
        if self.bytes.load(Ordering::Relaxed) > MAX_BYTES {
            self.rotate();
        }
        // Bloqueo normal (no try_lock): durante la rotación u otra escritura
        // simultánea, try_lock descartaría la línea y el historial en disco
        // ya no sería completo. Esperar garantiza que cada evento quede.
        let mut guard = self.file.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
            self.bytes.fetch_add(line.len() as u64, Ordering::Relaxed);
        }
    }

    fn rotate(&self) {
        let bak = self.path.with_extension("log.1");
        let _ = std::fs::rename(&self.path, &bak);
        let note =
            "Registro rotado (superó 5 MiB): el historial anterior se guardó en informe.log.1";
        let (y, mo, d, h, mi, s) = epoch_to_datetime(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|t| t.as_secs())
                .unwrap_or(0),
        );
        let line = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  {}  [{}] {}\n",
            y,
            mo,
            d,
            h,
            mi,
            s,
            Level::Info.tag(),
            "informe",
            note
        );
        // Bloqueo normal: reabrir el fichero tras la rotación y escribir la
        // nota en el mismo bloque, sin try_lock (evita perder la línea).
        let mut guard = self.file.lock().unwrap_or_else(|p| p.into_inner());
        *guard = open_append(&self.path);
        if let Some(f) = guard.as_mut() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
        self.bytes.store(0, Ordering::Relaxed);
    }

    /// Copia de los eventos de la sesión actual (para la pestaña Informe).
    pub fn snapshot(&self) -> Vec<Entry> {
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn count(&self) -> usize {
        self.entries.lock().unwrap_or_else(|p| p.into_inner()).len()
    }

    /// Texto completo del historial en disco (todas las sesiones).
    pub fn file_text(&self) -> Result<String, String> {
        let f = BufReader::new(
            File::open(&self.path).map_err(|e| format!("{}: {}", self.path.display(), e))?,
        );
        let mut out = String::new();
        for line in f.lines() {
            match line {
                Ok(l) => {
                    out.push_str(&l);
                    out.push('\n');
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(out)
    }

    pub fn file_bytes(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    /// Ruta del fichero de rotación (informe.log.1).
    pub fn rotate_path(&self) -> PathBuf {
        self.path.with_extension("log.1")
    }

    /// Borra el registro automático que la app guarda en disco:
    /// `informe.log` y su rotación `informe.log.1`. También vacía los eventos
    /// de la sesión mostrados en la pestaña «Informe». El registro sigue
    /// activo a partir de ese momento (los eventos nuevos se vuelven a
    /// escribir desde cero). Devuelve un mensaje de error si no se pudo.
    pub fn clear_auto_log(&self) -> Result<(), String> {
        // Cerrar el fichero antes de borrar para no dejar un descriptor
        // apuntando a un fichero eliminado.
        {
            let mut guard = self.file.lock().unwrap_or_else(|p| p.into_inner());
            *guard = None;
        }
        let mut err: Option<String> = None;
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => err = Some(format!("{}: {}", self.path.display(), e)),
        }
        let bak = self.rotate_path();
        if err.is_none() {
            match std::fs::remove_file(&bak) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => err = Some(format!("{}: {}", bak.display(), e)),
            }
        }
        // Reabrir el registro desde cero.
        {
            let mut guard = self.file.lock().unwrap_or_else(|p| p.into_inner());
            *guard = open_append(&self.path);
        }
        self.bytes.store(0, Ordering::Relaxed);
        self.entries
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

// ------------------------------------------------------------------ filtro

/// Etiqueta de nivel de una línea del registro en disco.
/// Las líneas de entrada tienen la forma
/// «YYYY-MM-DD HH:MM:SS  NIVEL  [cat] …»; las líneas de continuación de un
/// mensaje multilínea van sangradas (8 espacios) o vacías y devuelven None.
fn entry_tag(line: &str) -> Option<&str> {
    if line.starts_with(' ') {
        return None; // continúa un mensaje multilínea
    }
    let b = line.as_bytes();
    if b.len() < 26 || &b[19..21] != b"  " {
        return None;
    }
    let tag = std::str::from_utf8(&b[21..26]).ok()?;
    const TAGS: [&str; 6] = ["INFO ", "OK   ", "AVISO", "ERROR", "CMD  ", "DETAL"];
    if TAGS.contains(&tag) {
        Some(tag)
    } else {
        None
    }
}

/// Reduce el historial del fichero en disco a las líneas de ERROR
/// (incluidas las continuaciones multilínea de cada error).
pub fn filter_errors_only(text: &str) -> String {
    let mut keep = false;
    let mut out = String::with_capacity(text.len());
    for l in text.lines() {
        match entry_tag(l) {
            Some(tag) => {
                keep = tag == "ERROR";
                if keep {
                    out.push_str(l);
                    out.push('\n');
                }
            }
            None => {
                if keep {
                    out.push_str(l);
                    out.push('\n');
                }
            }
        }
    }
    out
}

/// Filtra el texto del historial según el nivel de detalle elegido.
pub fn filter_by_detail(text: &str, detalle: ReportDetail) -> String {
    match detalle {
        ReportDetail::Full => text.to_string(),
        ReportDetail::ErrorsOnly => filter_errors_only(text),
    }
}

// ------------------------------------------------------------------ export

/// Ruta por defecto para exportar: ~/Informe-Centinela-UFW-AAAAMMDD-HHMMSS.txt
pub fn default_export_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = epoch_to_datetime(ts);
    let name = format!(
        "Informe-Centinela-UFW-{:04}{:02}{:02}-{:02}{:02}{:02}.txt",
        y, mo, d, h, mi, s
    );
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(name)
}

/// Expande «~/…» al directorio personal del usuario.
pub fn expand_path(p: &str) -> PathBuf {
    let t = p.trim();
    if let Some(rest) = t.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(t)
}

/// Exporta el informe (historial de todas las sesiones) a un .txt, según el
/// nivel de detalle elegido: completo o solo errores. Devuelve el número de
/// líneas escritas.
pub fn export_to(dest: &std::path::Path, detalle: ReportDetail) -> Result<usize, String> {
    let text = global()
        .ok_or_else(|| "informe no inicializado".to_string())?
        .file_text()?;
    let contenido = filter_by_detail(&text, detalle);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {}", dir.display(), e))?;
    }
    let mut out = String::new();
    let lang = Lang::Es;
    let nivel = match detalle {
        ReportDetail::Full => tr(K::ReportDetailFull, lang),
        ReportDetail::ErrorsOnly => tr(K::ReportDetailErrorsOnly, lang),
    };
    out.push_str("══════════════════════════════════════════════════════════════\n");
    out.push_str(&format!(
        "  {} v{}\n",
        tr(K::AppName, lang),
        env!("CARGO_PKG_VERSION")
    ));
    out.push_str(&format!("  {}\n", tr(K::ReportExportHeader, lang)));
    out.push_str(&format!(
        "  {}\n",
        trf(K::ReportExportDetailFmt, lang, &[("%d", nivel.to_string())])
    ));
    out.push_str(&format!("  Exportado: {} (hora UTC)\n", format_now()));
    out.push_str(&format!(
        "  Creado por {} — Licencia GNU GPL v3\n",
        tr(K::AuthorName, lang)
    ));
    out.push_str("══════════════════════════════════════════════════════════════\n\n");
    if contenido.trim().is_empty() && detalle.errors_only() {
        out.push_str(&format!("{}\n", tr(K::ReportNoErrors, lang)));
    }
    out.push_str(&contenido);
    let lines = out.lines().count();
    std::fs::write(dest, out).map_err(|e| format!("{}: {}", dest.display(), e))?;
    Ok(lines)
}

fn format_now() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = epoch_to_datetime(ts);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, mi, s)
}

// ------------------------------------------------------------------ tiempo

/// Convierte segundos del epoch (UTC) en (año, mes, día, hh, mm, ss).
/// Algoritmo civil-from-days (H. Hinnant), sin dependencias externas.
pub fn epoch_to_datetime(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fecha_epoch() {
        // 2026-11-19 00:00:00 UTC = 1795046400 (verificado con python)
        let (y, m, d, h, mi, s) = epoch_to_datetime(1_795_046_400);
        assert_eq!((y, m, d, h, mi, s), (2026, 11, 19, 0, 0, 0));
        let (y, m, d, h, mi, s) = epoch_to_datetime(0);
        assert_eq!((y, m, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
        let (y, m, d, _, _, _) = epoch_to_datetime(86_399);
        assert_eq!((y, m, d), (1970, 1, 1));
        let (y, m, d, _, _, _) = epoch_to_datetime(86_400);
        assert_eq!((y, m, d), (1970, 1, 2));
    }

    /// Historial de ejemplo con el formato exacto del fichero en disco.
    /// (las continuaciones multilínea van sangradas con 8 espacios, igual
    /// que escribe Report::add al volcar mensajes con saltos de línea)
    fn historial_ejemplo() -> String {
        let mut s = String::new();
        s.push_str("2026-11-20 10:00:00  INFO   [inicio] Sesión iniciada — Centinela UFW v1.2.0\n");
        s.push_str("2026-11-20 10:00:02  CMD    [ufw] $ ufw status verbose\n");
        s.push_str("2026-11-20 10:00:02  DETAL  [ufw] código de salida: 0\n");
        s.push_str("        stdout: Status: active\n");
        s.push_str("        stderr: \n");
        s.push_str("2026-11-20 10:00:10  OK     [ufw] Estado refrescado\n");
        s.push_str("2026-11-20 10:01:00  ERROR  [ufw] ERROR: Could not find rule\n");
        s.push_str("        stdout: \n");
        s.push_str("        stderr: ERROR: Could not find rule with number 7\n");
        s.push_str("2026-11-20 10:01:05  INFO   [app] Operación terminada\n");
        s.push_str("2026-11-20 10:02:00  AVISO  [bandeja] Bandeja del sistema no disponible\n");
        s
    }

    #[test]
    fn deteccion_de_etiquetas() {
        let t = historial_ejemplo();
        let lineas: Vec<&str> = t.lines().collect();
        assert_eq!(entry_tag(lineas[0]), Some("INFO "));
        assert_eq!(entry_tag(lineas[1]), Some("CMD  "));
        assert_eq!(entry_tag(lineas[2]), Some("DETAL"));
        assert_eq!(entry_tag(lineas[3]), None); // continuación (8 espacios)
        assert_eq!(entry_tag(lineas[4]), None); // línea vacía → continúa/vacía
        assert_eq!(entry_tag(lineas[5]), Some("OK   "));
        assert_eq!(entry_tag(lineas[6]), Some("ERROR"));
        assert_eq!(entry_tag(lineas[9]), Some("INFO "));
        assert_eq!(entry_tag(lineas[10]), Some("AVISO"));
        // líneas ajenas al formato
        assert_eq!(entry_tag("Rule added"), None);
        assert_eq!(entry_tag("2026-11-20 10:00:00  FALSO  [x] y"), None);
    }

    #[test]
    fn filtro_solo_errores() {
        let t = historial_ejemplo();
        let f = filter_errors_only(&t);
        let lineas: Vec<&str> = f.lines().collect();
        // solo la entrada ERROR y sus dos líneas de continuación
        assert_eq!(lineas.len(), 3, "salida:\n{f}");
        assert!(lineas[0].contains("ERROR  [ufw] ERROR: Could not find rule"));
        assert!(lineas[1].contains("stdout:"));
        assert!(lineas[2].contains("stderr: ERROR: Could not find rule with number 7"));
        assert!(!f.contains("Sesión iniciada"));
        assert!(!f.contains("AVISO"));
    }

    #[test]
    fn filtro_por_detalle() {
        let t = historial_ejemplo();
        assert_eq!(filter_by_detail(&t, ReportDetail::Full), t);
        assert_eq!(
            filter_by_detail(&t, ReportDetail::ErrorsOnly),
            filter_errors_only(&t)
        );
        assert!(ReportDetail::default() == ReportDetail::Full);
        assert!(ReportDetail::ErrorsOnly.errors_only());
        assert!(!ReportDetail::Full.errors_only());
    }

    #[test]
    fn borrado_del_registro_automatico() {
        // Directorio temporal propio del test
        let dir = std::env::temp_dir().join(format!(
            "cufw-test-informe-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("informe.log");
        std::fs::write(&path, "2026-11-21 10:00:00  ERROR  [ufw] prueba\n").unwrap();
        let bak = path.with_extension("log.1");
        std::fs::write(&bak, "2026-11-20 10:00:00  INFO   [ufw] historial anterior\n").unwrap();

        let rep = Report {
            entries: Mutex::new(Vec::new()),
            file: Mutex::new(open_append(&path)),
            bytes: AtomicU64::new(std::fs::metadata(&path).unwrap().len()),
            path: path.clone(),
        };
        rep.add(Level::Info, "prueba", "evento pendiente".into());
        assert!(rep.count() > 0);

        // Borrar: no debe quedar ni informe.log con contenido ni informe.log.1
        rep.clear_auto_log().expect("borrado correcto");
        assert_eq!(rep.count(), 0);
        assert_eq!(rep.file_bytes(), 0, "informe.log debe quedar vacío");
        assert!(!bak.exists(), "informe.log.1 debe eliminarse");

        // El registro sigue activo: los eventos nuevos se escriben desde cero
        rep.add(Level::Ok, "prueba", "tras el borrado".into());
        let texto = rep.file_text().unwrap();
        assert!(texto.contains("tras el borrado"));
        assert!(
            !texto.contains("ERROR  [ufw] prueba"),
            "el historial antiguo no debe reaparecer"
        );

        // Idempotente: borrar de nuevo no falla aunque no existan ficheros
        rep.clear_auto_log().expect("segundo borrado correcto");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
