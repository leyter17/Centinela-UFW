//! Motor de ejecución: consulta y modifica el firewall UFW en segundo plano.
//!
//! La app nunca se ejecuta como root: los comandos privilegiados se lanzan
//! mediante `pkexec` (PolicyKit), con degradación a ejecución directa si el
//! usuario arrancó la app como root. Las consultas de estado (solo lectura)
//! se intentan primero sin privilegios para no pedir contraseña al arrancar.
//!
//! IMPORTANTE: UFW exige root incluso para consultar el estado, así que ese
//! primer intento sin privilegios normalmente responde «ERROR: You need to be
//! root to run this script». Es el comportamiento ESPERADO de ufw y NO se
//! registra como error (antes aparecía en el registro de actividad y en el
//! informe aunque la contraseña se escribiera bien): se detecta, se escala la
//! consulta mediante pkexec y se recuerda para las siguientes consultas.
//!
//! Todas las operaciones quedan registradas en el informe (report.rs).

use eframe::egui;
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use crate::i18n::{tr, trf, Lang, K};
use crate::report::{self, Level};

// ------------------------------------------------------------------ estado

#[derive(Clone, Default, Debug)]
pub struct Rule {
    pub num: u32,
    pub to: String,
    pub action: String,
    pub dir: String,
    pub from: String,
    pub comment: String,
    pub v6: bool,
}

#[derive(Clone, Debug)]
pub struct UfwState {
    pub active: Option<bool>,
    pub logging: String,
    pub def_in: String,
    pub def_out: String,
    pub def_routed: String,
    pub rules: Vec<Rule>,
    pub ufw_path: Option<String>,
    pub ufw_version: Option<String>,
    pub pkexec: Option<String>,
    pub ipv6: Option<bool>,
}

impl Default for UfwState {
    fn default() -> Self {
        UfwState {
            active: None,
            logging: "-".into(),
            def_in: "-".into(),
            def_out: "-".into(),
            def_routed: "-".into(),
            rules: Vec::new(),
            ufw_path: None,
            ufw_version: None,
            pkexec: None,
            ipv6: None,
        }
    }
}

/// Comprueba si la política de autenticación de Centinela UFW está instalada.
pub fn polkit_installed() -> bool {
    match std::fs::read_to_string(POLKIT_PATH) {
        Ok(s) => s.contains("com.ufwsentinel.ufw.manage"),
        Err(_) => false,
    }
}

pub const POLKIT_PATH: &str = "/usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy";

/// Bloquea un mutex tolerando envenenamiento (nunca entra en pánico).
pub fn lock_ok<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

// -------------------------------------------------------------------- logs

#[derive(Clone, Copy, PartialEq)]
pub enum LogKind {
    Info,
    Ok,
    Warn,
    Err,
    Cmd,
}

#[derive(Clone)]
pub struct LogEntry {
    pub secs: u64,
    pub kind: LogKind,
    pub text: String,
}

impl LogKind {
    fn level(self) -> Level {
        match self {
            LogKind::Info => Level::Info,
            LogKind::Ok => Level::Ok,
            LogKind::Warn => Level::Warn,
            LogKind::Err => Level::Err,
            LogKind::Cmd => Level::Cmd,
        }
    }
}

// ------------------------------------------------------------------ operaciones

pub enum Op {
    Refresh,
    Run {
        /// Descripción de la operación (para el informe).
        title: String,
        /// comandos como argumentos de ufw: [["allow","22/tcp"], …]
        cmds: Vec<Vec<String>>,
        /// script de shell completo (para sed, políticas polkit…)
        script: Option<String>,
        refresh_after: bool,
        capture: Option<Sender<String>>,
        /// mensaje de éxito opcional para el registro de actividad
        ok_log: Option<K>,
    },
}

// ------------------------------------------------------------------ engine

/// Modo de ejecución de un comando ufw.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    /// Consulta directa sin privilegios (no pide contraseña). Si ufw responde
    /// «You need to be root to run this script», se trata como señal de
    /// escalado y no como error.
    User,
    /// Comando que MODIFICA el firewall: vía pkexec (o directo si la app es
    /// root). Respeta el modo simulación (no ejecuta nada).
    PrivAction,
    /// Consulta de estado con privilegios: vía pkexec (o directo si es root).
    /// El modo simulación NO la bloquea: consultar el estado nunca modifica
    /// el firewall.
    PrivQuery,
}

pub struct Engine {
    pub state: Arc<Mutex<UfwState>>,
    pub log: Arc<Mutex<Vec<LogEntry>>>,
    pub busy: Arc<AtomicBool>,
    pub dry_run: Arc<AtomicBool>,
    pub lang: Arc<Mutex<Lang>>,
    pub demo: bool,
    /// true cuando ya se detectó que ufw exige root para las consultas de
    /// estado (lo habitual): las siguientes consultas van directo por pkexec.
    status_priv: Arc<AtomicBool>,
    tx: Sender<Op>,
    start: Instant,
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Engine {
            state: self.state.clone(),
            log: self.log.clone(),
            busy: self.busy.clone(),
            dry_run: self.dry_run.clone(),
            lang: self.lang.clone(),
            demo: self.demo,
            status_priv: self.status_priv.clone(),
            tx: self.tx.clone(),
            start: self.start,
        }
    }
}

impl Engine {
    pub fn spawn(demo: bool, ctx: egui::Context, lang: Arc<Mutex<Lang>>) -> Engine {
        let (tx, rx) = std::sync::mpsc::channel();
        let eng = Engine {
            state: Arc::new(Mutex::new(UfwState::default())),
            log: Arc::new(Mutex::new(Vec::new())),
            busy: Arc::new(AtomicBool::new(false)),
            dry_run: Arc::new(AtomicBool::new(false)),
            lang,
            demo,
            status_priv: Arc::new(AtomicBool::new(false)),
            tx,
            start: Instant::now(),
        };
        let w = eng.clone();
        let spawn_res = std::thread::Builder::new()
            .name("ufw-worker".into())
            .spawn(move || worker_loop(w, rx, ctx));
        if let Err(e) = spawn_res {
            report::event(
                Level::Err,
                "inicio",
                format!("No se pudo crear el hilo de trabajo: {e}"),
            );
        }
        eng
    }

    pub fn submit(&self, op: Op) {
        if let Err(e) = self.tx.send(op) {
            report::event(
                Level::Err,
                "ufw",
                format!("No se pudo encolar la operación: {e}"),
            );
        }
    }

    /// Añade una entrada al registro de actividad (panel inferior) y la
    /// replica en el informe persistente.
    pub fn log_add(&self, kind: LogKind, text: String) {
        let secs = self.start.elapsed().as_secs();
        {
            let mut logs = lock_ok(&self.log);
            logs.push(LogEntry {
                secs,
                kind,
                text: text.clone(),
            });
            let n = logs.len();
            if n > 400 {
                logs.drain(0..n - 400);
            }
        }
        report::event(kind.level(), "app", text);
    }

    fn tr(&self, k: K) -> String {
        let l = *lock_ok(&self.lang);
        tr(k, l).to_string()
    }

    /// Consulta el estado (detecta ufw, ejecuta status verbose/numbered).
    pub fn do_refresh(&self) {
        if self.demo {
            *lock_ok(&self.state) = demo_state();
            self.log_add(LogKind::Info, self.tr(K::DemoRefresh));
            report::event(
                Level::Info,
                "demo",
                "Estado de demostración cargado (no se ejecuta ningún comando)",
            );
            return;
        }
        let mut st = lock_ok(&self.state);
        if st.ufw_path.is_none() {
            st.ufw_path = find_binary("ufw");
            st.pkexec = find_binary("pkexec");
            match (&st.ufw_path, &st.pkexec) {
                (Some(u), Some(p)) => {
                    report::event(
                        Level::Info,
                        "ufw",
                        format!("ufw detectado: {u} · pkexec: {p}"),
                    );
                }
                (Some(u), None) => {
                    report::event(
                        Level::Warn,
                        "ufw",
                        format!("ufw detectado: {u} · pkexec NO encontrado"),
                    );
                }
                (None, _) => {
                    report::event(
                        Level::Err,
                        "ufw",
                        "ufw no encontrado en el sistema (PATH, /usr/sbin, /sbin, /usr/local/sbin)",
                    );
                }
            }
            if st.ufw_path.is_none() {
                drop(st);
                self.log_add(LogKind::Err, self.tr(K::ErrNoUfw));
                return;
            }
        }
        let ufw = st.ufw_path.clone().unwrap();
        if st.pkexec.is_none() {
            st.pkexec = find_binary("pkexec");
        }
        if st.ufw_version.is_none() {
            match run_process(&ufw, &["--version".to_string()]) {
                Ok((0, out, _)) => {
                    let first = out.trim().lines().next().unwrap_or("").to_string();
                    if !first.is_empty() {
                        st.ufw_version = Some(first.clone());
                        report::event(Level::Info, "ufw", format!("Versión de UFW: {first}"));
                    }
                }
                Ok((c, _, e)) => {
                    report::event(
                        Level::Warn,
                        "ufw",
                        format!("«ufw --version» devolvió el código {c} · {e}"),
                    );
                }
                Err(e) => {
                    report::event(
                        Level::Err,
                        "ufw",
                        format!("No se pudo ejecutar «ufw --version»: {e}"),
                    );
                }
            }
        }
        st.ipv6 = read_ipv6_flag();
        if let Some(v6) = st.ipv6 {
            report::event(
                Level::Info,
                "ufw",
                format!("IPV6={} en /etc/default/ufw", if v6 { "yes" } else { "no" }),
            );
        } else {
            report::event(
                Level::Warn,
                "ufw",
                "No se pudo leer IPV6 de /etc/default/ufw",
            );
        }
        drop(st);

        // Consultas de solo lectura: primero sin privilegios (sin contraseña).
        // UFW normalmente exige root incluso para consultar el estado; ese
        // primer intento fallido es el comportamiento ESPERADO y no se registra
        // como error (antes aparecía «ERROR: You need to be root to run this
        // script» en el registro y en el informe aunque la contraseña se
        // escribiera bien). Se detecta, se escala por pkexec y se recuerda.
        let args1: Vec<String> = vec!["status".into(), "verbose".into()];
        let args2: Vec<String> = vec!["status".into(), "numbered".into()];
        let (ok1, out1) = self.status_query(&args1);
        let (ok2, out2) = self.status_query(&args2);
        if ok1 && ok2 {
            let mut st = lock_ok(&self.state);
            parse_verbose(&out1, &mut st);
            st.rules = parse_numbered(&out2);
            let n = st.rules.len();
            let activo = st.active;
            let logging = st.logging.clone();
            drop(st);
            report::event(
                Level::Ok,
                "ufw",
                format!(
                    "Estado refrescado: {} · reglas: {} · logging: {}",
                    match activo {
                        Some(true) => "activo",
                        Some(false) => "inactivo",
                        None => "desconocido",
                    },
                    n,
                    logging
                ),
            );
            self.log_add(
                LogKind::Ok,
                format!(
                    "{} · {}",
                    self.tr(K::RefreshDone),
                    trf(
                        K::RuleCountFmt,
                        *lock_ok(&self.lang),
                        &[("%n", n.to_string())]
                    )
                ),
            );
        } else {
            report::event(
                Level::Err,
                "ufw",
                "No se pudo completar la consulta de estado (auth cancelada o ufw no disponible); el estado mostrado no cambia",
            );
        }
    }

    /// Consulta de estado de solo lectura. Se intenta primero sin privilegios
    /// (sin contraseña); si ufw exige root —lo habitual— se escala mediante
    /// pkexec SIN registrar error alguno, porque es el comportamiento esperado
    /// de ufw y no un fallo de la app. Se recuerda para las siguientes
    /// consultas y así no se repite el intento innecesario.
    fn status_query(&self, args: &[String]) -> (bool, String) {
        if self.status_priv.load(Ordering::Relaxed) {
            return self.exec_ufw(args, false, ExecMode::PrivQuery);
        }
        let (ok, out) = self.exec_ufw(args, false, ExecMode::User);
        if ok {
            return (ok, out);
        }
        if !self.status_priv.swap(true, Ordering::Relaxed) {
            report::event(
                Level::Info,
                "ufw",
                "UFW exige privilegios de root incluso para consultar el estado (comportamiento normal de ufw, no es un error): las consultas se realizarán mediante pkexec",
            );
        }
        self.exec_ufw(args, false, ExecMode::PrivQuery)
    }

    /// Ejecuta ufw con argumentos. Devuelve (ok, salida combinada).
    ///
    /// - `log_cmd`: registra «$ ufw …» en el registro de actividad.
    /// - `mode`: ver `ExecMode`. Las consultas `User` nunca registran como
    ///   error la respuesta «You need to be root to run this script» de ufw.
    fn exec_ufw(&self, args: &[String], log_cmd: bool, mode: ExecMode) -> (bool, String) {
        let (ufw, pk) = {
            let st = lock_ok(&self.state);
            (st.ufw_path.clone(), st.pkexec.clone())
        };
        let Some(ufw) = ufw else {
            self.log_add(LogKind::Err, self.tr(K::ErrNoUfw));
            return (false, String::new());
        };
        if log_cmd {
            self.log_add(LogKind::Cmd, format!("$ {}", pretty_cmd("ufw", args)));
        }
        if mode == ExecMode::PrivAction && self.dry_run.load(Ordering::Relaxed) {
            self.log_add(LogKind::Warn, self.tr(K::DryRunLog));
            report::event(
                Level::Warn,
                "ufw",
                format!("[SIMULACIÓN] no se ejecutó: {}", pretty_cmd("ufw", args)),
            );
            return (true, String::new());
        }
        let use_pk = mode != ExecMode::User && !is_root();
        if use_pk && pk.is_none() {
            self.log_add(LogKind::Err, self.tr(K::ErrNoPkexec));
            report::event(
                Level::Err,
                "ufw",
                "pkexec no disponible y el proceso no es root: comando cancelado",
            );
            return (false, String::new());
        }
        let (prog, full_args): (String, Vec<String>) = if use_pk {
            let mut a = vec![ufw.clone()];
            a.extend_from_slice(args);
            (pk.unwrap(), a)
        } else {
            (ufw.clone(), args.to_vec())
        };
        report::event(
            Level::Cmd,
            "ufw",
            format!(
                "$ {}{}",
                pretty_cmd(&prog, &full_args),
                if is_root() { "  (como root)" } else { "" }
            ),
        );
        match run_process(&prog, &full_args) {
            Ok((code, out, err)) => {
                report::event(
                    Level::Detail,
                    "ufw",
                    format!("código de salida: {code}\nstdout: {out}\nstderr: {err}"),
                );
                let mut combined = out;
                if !err.is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(&err);
                }
                if code == 0 {
                    (true, combined)
                } else if is_auth_cancelled(&combined) {
                    self.log_add(LogKind::Warn, self.tr(K::AuthCancelled));
                    (false, combined)
                } else if mode == ExecMode::User && is_needs_root(&combined) {
                    // Respuesta esperada de ufw ante una consulta sin
                    // privilegios: NO es un error de la app. Solo se anota el
                    // detalle en el informe; el llamador decidirá escalar por
                    // pkexec. Antes esto aparecía como ERROR aunque la
                    // contraseña introducida después fuera correcta.
                    report::event(
                        Level::Detail,
                        "ufw",
                        "La consulta sin privilegios requiere root (respuesta esperada de ufw, no es un error); se escala mediante pkexec",
                    );
                    (false, combined)
                } else {
                    self.log_add(LogKind::Err, combined.clone());
                    (false, combined)
                }
            }
            Err(e) => {
                self.log_add(LogKind::Err, e.clone());
                report::event(
                    Level::Err,
                    "ufw",
                    format!("Fallo al lanzar el proceso: {e}"),
                );
                (false, String::new())
            }
        }
    }

    /// Ejecuta un script de shell como root (vía pkexec sh -c) o directamente
    /// si la app corre como root.
    fn exec_script(&self, script: &str, display: &str) -> bool {
        self.log_add(LogKind::Cmd, format!("$ {display}"));
        if self.dry_run.load(Ordering::Relaxed) {
            self.log_add(LogKind::Warn, self.tr(K::DryRunLog));
            report::event(
                Level::Warn,
                "ufw",
                format!("[SIMULACIÓN] no se ejecutó el script: {display}"),
            );
            return true;
        }
        let pk = lock_ok(&self.state).pkexec.clone();
        let (prog, args): (String, Vec<String>) = if is_root() {
            ("sh".into(), vec!["-c".into(), script.to_string()])
        } else {
            let Some(pk) = pk else {
                self.log_add(LogKind::Err, self.tr(K::ErrNoPkexec));
                report::event(Level::Err, "ufw", "pkexec no disponible: script cancelado");
                return false;
            };
            (pk, vec!["sh".into(), "-c".into(), script.to_string()])
        };
        report::event(Level::Cmd, "ufw", format!("$ {display}\nscript:\n{script}"));
        match run_process(&prog, &args) {
            Ok((code, out, err)) => {
                report::event(
                    Level::Detail,
                    "ufw",
                    format!("código de salida: {code}\nstdout: {out}\nstderr: {err}"),
                );
                // stdout y stderr con separador, igual que en exec_ufw, para
                // que la detección de «autenticación cancelada» no se fíe de
                // palabras fusionadas entre ambas salidas.
                let mut combined = out;
                if !err.is_empty() {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(&err);
                }
                if code == 0 {
                    true
                } else if is_auth_cancelled(&combined) {
                    self.log_add(LogKind::Warn, self.tr(K::AuthCancelled));
                    false
                } else {
                    let detail = if combined.trim().is_empty() {
                        format!("(sin salida del sistema) código {code}")
                    } else {
                        combined.trim().to_string()
                    };
                    self.log_add(LogKind::Err, detail.clone());
                    false
                }
            }
            Err(e) => {
                self.log_add(LogKind::Err, e.clone());
                report::event(Level::Err, "ufw", format!("Fallo al lanzar el script: {e}"));
                false
            }
        }
    }
}

/// Ejecuta un proceso y devuelve (código, stdout, stderr) recortados.
/// Se fuerza LC_ALL=C para que la salida de ufw sea siempre en inglés y el
/// parseo sea independiente del idioma del sistema.
fn run_process(prog: &str, args: &[String]) -> Result<(i32, String, String), String> {
    let o = std::process::Command::new(prog)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| e.to_string())?;
    Ok((
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).trim().to_string(),
        String::from_utf8_lossy(&o.stderr).trim().to_string(),
    ))
}

fn worker_loop(eng: Engine, rx: Receiver<Op>, ctx: egui::Context) {
    while let Ok(op) = rx.recv() {
        eng.busy.store(true, Ordering::Relaxed);
        ctx.request_repaint();
        // Un pánico inesperado no debe dejar nunca la app atascada en
        // «Trabajando…»: se captura, se registra y el motor sigue vivo.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match &op {
            Op::Refresh => eng.do_refresh(),
            Op::Run {
                title,
                cmds,
                script,
                refresh_after,
                capture,
                ok_log,
            } => {
                report::event(Level::Info, "op", format!("Operación solicitada: {title}"));
                let mut all_ok = true;
                if eng.demo {
                    if let Some(s) = script {
                        eng.log_add(LogKind::Cmd, format!("$ {s}"));
                    }
                    for c in cmds {
                        eng.log_add(LogKind::Cmd, format!("$ {}", pretty_cmd("ufw", c)));
                    }
                    eng.log_add(LogKind::Warn, eng.tr(K::DryRunLog));
                    report::event(
                        Level::Warn,
                        "demo",
                        format!("[DEMO] no se ejecutó: {title}"),
                    );
                    if let Some(tx) = capture {
                        let _ = tx.send(
                            "CUPS\nOpenSSH\nSamba\nNginx Full\nApache Full\nPostfix\n".to_string(),
                        );
                    }
                } else if let Some(s) = script {
                    all_ok = eng.exec_script(s, title);
                } else {
                    for c in cmds {
                        let (ok, out) = eng.exec_ufw(c, true, ExecMode::PrivAction);
                        // Si falló, exec_ufw ya registró el error completo en
                        // el registro y el informe; registrar la salida aquí
                        // de nuevo duplicaría la línea en ambos. Solo se añade
                        // la salida de las operaciones correctas.
                        if ok && !out.is_empty() {
                            eng.log_add(LogKind::Info, out.clone());
                        }
                        if let Some(tx) = capture {
                            let _ = tx.send(out);
                        }
                        if !ok {
                            all_ok = false;
                            break;
                        }
                    }
                }
                if *refresh_after {
                    if eng.demo {
                        *lock_ok(&eng.state) = demo_state();
                    } else if all_ok {
                        eng.do_refresh();
                    }
                }
                if all_ok && !eng.demo {
                    if let Some(k) = ok_log {
                        eng.log_add(LogKind::Ok, eng.tr(*k));
                    }
                }
            }
        }));
        if let Err(p) = res {
            let msg = if let Some(s) = p.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = p.downcast_ref::<String>() {
                s.clone()
            } else {
                "pánico desconocido".to_string()
            };
            report::event(
                Level::Err,
                "motor",
                format!("Error interno recuperado en el hilo de trabajo: {msg}"),
            );
            eng.log_add(LogKind::Err, eng.tr(K::InternalErr));
        }
        // CRÍTICO: el indicador de ocupación se libera SIEMPRE, también tras
        // Op::Refresh (antes se quedaba a true y la app quedaba «Trabajando…»).
        eng.busy.store(false, Ordering::Relaxed);
        ctx.request_repaint();
    }
}

// ------------------------------------------------------------------ helpers

pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

pub fn find_binary(name: &str) -> Option<String> {
    let mut dirs: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .map(|s| s.to_string())
        .collect();
    for extra in ["/usr/sbin", "/sbin", "/usr/local/sbin", "/usr/bin", "/bin"] {
        if !dirs.iter().any(|d| d == extra) {
            dirs.push(extra.to_string());
        }
    }
    for d in dirs {
        if d.is_empty() {
            continue;
        }
        let p = std::path::Path::new(&d).join(name);
        if let Ok(md) = std::fs::metadata(&p) {
            if md.is_file() {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn read_ipv6_flag() -> Option<bool> {
    let txt = std::fs::read_to_string("/etc/default/ufw").ok()?;
    for line in txt.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("IPV6=") {
            return Some(v.trim().eq_ignore_ascii_case("yes"));
        }
    }
    None
}

fn is_auth_cancelled(out: &str) -> bool {
    let o = out.to_lowercase();
    o.contains("request dismissed")
        || o.contains("dismissed")
        || o.contains("authentization failed")
        || o.contains("authentication failed")
        || o.contains("not authorized")
        || o.contains("no session for cookie")
        || o.contains("operation cancelled")
        || o.contains("aborted")
}

/// Detecta la respuesta de ufw cuando se ejecuta sin privilegios: UFW exige
/// root para casi todo (incluso para consultar el estado), y responde
/// «ERROR: You need to be root to run this script». Es un comportamiento
/// esperado, no un error de la aplicación.
fn is_needs_root(out: &str) -> bool {
    out.to_lowercase().contains("need to be root")
}

fn quote_arg(a: &str) -> String {
    if a.is_empty() || a.contains(' ') || a.contains('\'') {
        format!("'{}'", a.replace('\'', "'\\''"))
    } else {
        a.to_string()
    }
}

pub fn pretty_cmd(prog: &str, args: &[String]) -> String {
    let mut s = String::from(prog);
    for a in args {
        s.push(' ');
        s.push_str(&quote_arg(a));
    }
    s
}

// ------------------------------------------------------------------ parseo

static RE_RULE: OnceLock<Regex> = OnceLock::new();
static RE_COMMENT: OnceLock<Regex> = OnceLock::new();
static RE_DEFAULT: OnceLock<Regex> = OnceLock::new();

fn re_rule() -> &'static Regex {
    RE_RULE.get_or_init(|| {
        Regex::new(r"^\s*\[\s*(\d+)\]\s+(.+?)\s{2,}([A-Z]+)(?:\s+(IN|OUT|FWD))?\s{2,}(.+)$")
            .unwrap()
    })
}
fn re_comment() -> &'static Regex {
    RE_COMMENT.get_or_init(|| Regex::new(r"^\s*comment\s+'(.*)'\s*$").unwrap())
}
fn re_default() -> &'static Regex {
    RE_DEFAULT.get_or_init(|| Regex::new(r"^(\w+)\s+\((\w+)\)$").unwrap())
}

pub fn parse_verbose(text: &str, st: &mut UfwState) {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Status:") {
            st.active = Some(rest.trim() == "active");
        } else if let Some(rest) = line.strip_prefix("Logging:") {
            st.logging = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Default:") {
            for part in rest.split(',') {
                if let Some(c) = re_default().captures(part.trim()) {
                    let policy = c[1].to_string();
                    match &c[2] {
                        "incoming" => st.def_in = policy,
                        "outgoing" => st.def_out = policy,
                        "routed" => st.def_routed = policy,
                        _ => {}
                    }
                }
            }
        }
    }
}

pub fn parse_numbered(text: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    for line in text.lines() {
        if let Some(c) = re_rule().captures(line) {
            let mut to = c[2].trim().to_string();
            let mut v6 = false;
            if let Some(pos) = to.find(" (v6)") {
                v6 = true;
                to = to[..pos].to_string();
            } else if to.ends_with("(v6)") {
                v6 = true;
                to = to.replace("(v6)", "").trim().to_string();
            }
            let action = c[3].to_string();
            let dir = c.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();
            let mut from = c[5].trim().to_string();
            if from.ends_with("(v6)") {
                v6 = true;
                from = from.replace("(v6)", "").trim().to_string();
            }
            rules.push(Rule {
                num: c[1].parse().unwrap_or(0),
                to,
                action,
                dir,
                from,
                comment: String::new(),
                v6,
            });
        } else if let Some(cm) = re_comment().captures(line) {
            if let Some(last) = rules.last_mut() {
                last.comment = cm[1].to_string();
            }
        }
    }
    rules
}

fn demo_state() -> UfwState {
    UfwState {
        active: Some(true),
        logging: "on (low)".into(),
        def_in: "deny".into(),
        def_out: "allow".into(),
        def_routed: "disabled".into(),
        rules: vec![
            Rule {
                num: 1,
                to: "22/tcp".into(),
                action: "LIMIT".into(),
                dir: "IN".into(),
                from: "Anywhere".into(),
                comment: "SSH administrativo".into(),
                v6: false,
            },
            Rule {
                num: 2,
                to: "80,443/tcp".into(),
                action: "ALLOW".into(),
                dir: "IN".into(),
                from: "Anywhere".into(),
                comment: String::new(),
                v6: false,
            },
            Rule {
                num: 3,
                to: "137,138/udp".into(),
                action: "ALLOW".into(),
                dir: "IN".into(),
                from: "192.168.1.0/24".into(),
                comment: "Samba LAN".into(),
                v6: false,
            },
            Rule {
                num: 4,
                to: "22/tcp".into(),
                action: "LIMIT".into(),
                dir: "IN".into(),
                from: "Anywhere".into(),
                comment: String::new(),
                v6: true,
            },
            Rule {
                num: 5,
                to: "8080/tcp".into(),
                action: "DENY".into(),
                dir: "IN".into(),
                from: "Anywhere".into(),
                comment: String::new(),
                v6: false,
            },
        ],
        ufw_path: Some("/usr/sbin/ufw".into()),
        ufw_version: Some("ufw 0.36.2 (demo)".into()),
        pkexec: Some("/usr/bin/pkexec".into()),
        ipv6: Some(true),
    }
}

// ------------------------------------------------------------------ polkit

pub fn polkit_policy_xml(ufw_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
<policyconfig>
  <vendor>Centinela UFW (UFW Sentinel)</vendor>
  <vendor_url>https://www.gnu.org/licenses/gpl-3.0.html</vendor_url>
  <action id="com.ufwsentinel.ufw.manage">
    <description>Manage the UFW firewall with UFW Sentinel</description>
    <description xml:lang="es">Gestionar el firewall UFW con Centinela UFW</description>
    <message>Authentication is required to manage the UFW firewall</message>
    <message xml:lang="es">Se requiere autenticación para gestionar el firewall UFW</message>
    <defaults>
      <allow_any>auth_admin_keep</allow_any>
      <allow_inactive>auth_admin_keep</allow_inactive>
      <allow_active>auth_admin_keep</allow_active>
    </defaults>
    <annotate key="org.freedesktop.policykit.exec.path">{ufw_path}</annotate>
    <annotate key="org.freedesktop.policykit.exec.allow_gui">true</annotate>
  </action>
</policyconfig>
"#
    )
}

/// Script que instala la política polkit (el contenido viaja en base64).
pub fn polkit_install_script(ufw_path: &str) -> (String, String) {
    let xml = polkit_policy_xml(ufw_path);
    let b64 = base64_encode(xml.as_bytes());
    let script = format!(
        "mkdir -p /usr/share/polkit-1/actions && echo {b64} | base64 -d > {POLKIT_PATH} && sync"
    );
    let display =
        format!("install /usr/share/polkit-1/actions/com.ufwsentinel.ufw.policy (ufw: {ufw_path})");
    (script, display)
}

pub fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// ------------------------------------------------------------------ varios

pub fn detect_desktop() -> String {
    let env_xdg = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    let session = std::env::var("DESKTOP_SESSION")
        .unwrap_or_default()
        .to_lowercase();
    if env_xdg.contains("cosmic") || session.contains("cosmic") {
        "COSMIC".into()
    } else if env_xdg.contains("kde") || session.contains("plasma") || session.contains("kde") {
        "KDE Plasma".into()
    } else if env_xdg.contains("xfce") || session.contains("xfce") {
        "XFCE".into()
    } else if env_xdg.contains("gnome") || session.contains("gnome") || session.contains("ubuntu") {
        "GNOME".into()
    } else if !env_xdg.is_empty() {
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()
    } else if !session.is_empty() {
        session
    } else {
        "-".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseo_status_verbose() {
        let txt = "Status: active\nLogging: on (low)\nDefault: deny (incoming), allow (outgoing), disabled (routed)\n";
        let mut st = UfwState::default();
        parse_verbose(txt, &mut st);
        assert_eq!(st.active, Some(true));
        assert_eq!(st.logging, "on (low)");
        assert_eq!(st.def_in, "deny");
        assert_eq!(st.def_out, "allow");
        assert_eq!(st.def_routed, "disabled");
    }

    #[test]
    fn parseo_status_inactive() {
        let mut st = UfwState::default();
        parse_verbose("Status: inactive\n", &mut st);
        assert_eq!(st.active, Some(false));
    }

    #[test]
    fn parseo_numbered_con_comentarios() {
        let txt = "Status: active\n\
                   To                         Action      From\n\
                   --                         ------      ----\n\
                   [ 1] 22/tcp                     LIMIT IN    Anywhere\n\
                        comment 'SSH administrativo'\n\
                   [ 2] 80,443/tcp                 ALLOW IN    Anywhere\n\
                   [ 3] 22/tcp (v6)                LIMIT IN    Anywhere (v6)\n\
                   [ 4] 137,138/udp                ALLOW IN    192.168.1.0/24\n";
        let r = parse_numbered(txt);
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].num, 1);
        assert_eq!(r[0].to, "22/tcp");
        assert_eq!(r[0].action, "LIMIT");
        assert_eq!(r[0].dir, "IN");
        assert_eq!(r[0].from, "Anywhere");
        assert_eq!(r[0].comment, "SSH administrativo");
        assert_eq!(r[1].to, "80,443/tcp");
        assert_eq!(r[1].comment, "");
        assert!(r[2].v6);
        assert_eq!(r[2].to, "22/tcp");
        assert_eq!(r[2].from, "Anywhere");
        assert_eq!(r[3].from, "192.168.1.0/24");
    }

    #[test]
    fn base64_conocido() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn deteccion_autenticacion_cancelada() {
        assert!(is_auth_cancelled("Request dismissed"));
        assert!(is_auth_cancelled("Not authorized to perform operation"));
        assert!(is_auth_cancelled("Authentication failed"));
        assert!(!is_auth_cancelled("Rule added"));
    }

    #[test]
    fn deteccion_consulta_sin_root() {
        // Respuesta exacta de ufw al ejecutarlo sin privilegios: es el
        // comportamiento esperado y NO debe registrarse como error.
        assert!(is_needs_root("ERROR: You need to be root to run this script"));
        assert!(is_needs_root("you need to be root to run this script"));
        assert!(!is_needs_root("ERROR: Could not find rule with number 7"));
        assert!(!is_needs_root("Rule added"));
        assert!(!is_needs_root("Status: active"));
    }
}
