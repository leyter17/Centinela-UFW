//! Centinela UFW (UFW Sentinel) — administrador gráfico del firewall UFW.
//!
//! Creado por Leyter Vázquez . V — Licencia GNU GPL v3.

mod config;
mod i18n;
mod report;
mod theme;
mod tray;
mod ufw;
mod ui;

use eframe::egui;
use egui::ViewportCommand;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

use config::{Config, ReportDetail};
use i18n::{tr, trf, Lang, K};
use tray::TrayCmd;
use ufw::{lock_ok, Engine, LogKind, Op};

/// Recorridos automáticos para pruebas y capturas (uso interno).
#[derive(Clone, Copy, PartialEq)]
pub enum TourKind {
    /// Modo demo: recorre las pestañas sin tocar el firewall.
    AutoTour,
    /// Ejecuta un flujo completo contra un ufw real o simulado.
    SimFlow,
}

pub struct App {
    pub eng: Engine,
    pub cfg: Config,
    pub tab: ui::Tab,
    pub lang_shared: Arc<Mutex<Lang>>,

    // formulario de reglas
    pub r_action: usize,
    pub r_dir: usize,
    pub r_proto: usize,
    pub r_port: String,
    pub r_from: String,
    pub r_to: String,
    pub r_comment: String,

    // perfiles de aplicación de ufw
    pub apps_rx: Option<Receiver<String>>,
    pub apps: Vec<String>,
    pub apps_loading: bool,

    // informe
    pub report_export_path: String,
    pub report_status: Option<(bool, String)>,

    // confirmaciones
    pub confirm: Option<ui::ConfirmKind>,

    // bandeja
    pub tray: Option<ksni::blocking::Handle<tray::TrayData>>,
    pub tray_rx: Receiver<TrayCmd>,
    pub hidden: Arc<AtomicBool>,
    pub active_flag: Arc<AtomicBool>,
    pub last_active: Option<bool>,

    pub logo: Option<egui::TextureHandle>,
    pub tour: Option<(TourKind, std::time::Instant, usize)>,
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: Config,
        demo: bool,
        tour: Option<TourKind>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        theme::apply(&ctx);

        // informe persistente
        report::init();
        if let Some(r) = report::global() {
            report::event(
                report::Level::Info,
                "inicio",
                format!(
                    "Sesión iniciada — Centinela UFW v{} · usuario: {} (uid {}) · escritorio: {} · modo demo: {} · argumentos: {:?}",
                    env!("CARGO_PKG_VERSION"),
                    std::env::var("USER").unwrap_or_else(|_| std::env::var("LOGNAME").unwrap_or_else(|_| "-".into())),
                    unsafe { libc::geteuid() },
                    ufw::detect_desktop(),
                    if demo { "sí" } else { "no" },
                    std::env::args().skip(1).collect::<Vec<_>>()
                ),
            );
            report::event(
                report::Level::Info,
                "informe",
                format!(
                    "Registro automático activo (se escribe cada evento en: {})",
                    r.path.display()
                ),
            );
        }

        let lang_shared = Arc::new(Mutex::new(cfg.lang));
        let eng = Engine::spawn(demo, ctx.clone(), lang_shared.clone());
        eng.dry_run.store(cfg.dry_run, Ordering::Relaxed);
        eng.log_add(LogKind::Info, tr(K::WelcomeLog, cfg.lang).to_string());
        eng.submit(Op::Refresh);

        let hidden = Arc::new(AtomicBool::new(false));
        let active_flag = Arc::new(AtomicBool::new(false));
        let (tx, tray_rx) = std::sync::mpsc::channel();

        let icon_on = include_bytes!("../assets/tray_on_32.png").to_vec();
        let icon_off = include_bytes!("../assets/tray_off_32.png").to_vec();

        let tray = tray::start_tray(tray::TrayData {
            active: active_flag.clone(),
            hidden: hidden.clone(),
            lang: lang_shared.clone(),
            tx: tx.clone(),
            ctx: ctx.clone(),
            icon_on,
            icon_off,
        });
        if let Err(e) = &tray {
            let msg = format!(
                "{} ({e})",
                match cfg.lang {
                    Lang::Es => "Bandeja del sistema no disponible",
                    Lang::En => "System tray not available",
                }
            );
            eng.log_add(LogKind::Warn, msg.clone());
            report::event(report::Level::Warn, "bandeja", msg);
        } else {
            report::event(
                report::Level::Ok,
                "bandeja",
                "Icono de bandeja del sistema registrado",
            );
        }

        // logo para la pestaña Acerca de y la barra superior
        let logo = image::load_from_memory(include_bytes!("../assets/icon_256.png")).ok();
        let logo = logo.map(|img| {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            ctx.load_texture(
                "logo",
                egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                egui::TextureOptions::LINEAR,
            )
        });

        let export_default = report::default_export_path().to_string_lossy().to_string();

        App {
            eng,
            cfg,
            tab: ui::Tab::Dashboard,
            lang_shared,
            r_action: 0,
            r_dir: 0,
            r_proto: 0,
            r_port: String::new(),
            r_from: String::new(),
            r_to: String::new(),
            r_comment: String::new(),
            apps_rx: None,
            apps: Vec::new(),
            apps_loading: false,
            report_export_path: export_default,
            report_status: None,
            confirm: None,
            tray: tray.ok(),
            tray_rx,
            hidden,
            active_flag,
            last_active: None,
            logo,
            tour: tour.map(|k| (k, std::time::Instant::now(), 0)),
        }
    }

    // ------------------------------------------------------------ acciones

    pub fn run_cmds(&self, label: String, cmds: Vec<Vec<String>>, refresh_after: bool) {
        self.eng.submit(Op::Run {
            title: label,
            cmds,
            script: None,
            refresh_after,
            capture: None,
            ok_log: None,
        });
    }

    pub fn run_script(
        &self,
        label: String,
        script: String,
        refresh_after: bool,
        ok_log: Option<K>,
    ) {
        self.eng.submit(Op::Run {
            title: label,
            cmds: Vec::new(),
            script: Some(script),
            refresh_after,
            capture: None,
            ok_log,
        });
    }

    pub fn request_apps(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.eng.submit(Op::Run {
            title: "ufw app list".into(),
            cmds: vec![vec!["app".into(), "list".into()]],
            script: None,
            refresh_after: false,
            capture: Some(tx),
            ok_log: None,
        });
        self.apps_loading = true;
        self.apps_rx = Some(rx);
        report::event(
            report::Level::Info,
            "ufw",
            "Solicitando la lista de perfiles de aplicación de UFW",
        );
    }

    pub fn clear_form(&mut self) {
        self.r_action = 0;
        self.r_dir = 0;
        self.r_proto = 0;
        self.r_port.clear();
        self.r_from.clear();
        self.r_to.clear();
        self.r_comment.clear();
    }

    /// Construye los argumentos de ufw para la regla del formulario.
    pub fn rule_args(&self) -> Result<Vec<String>, String> {
        let lang = self.cfg.lang;
        let action = ["allow", "deny", "reject", "limit"][self.r_action].to_string();
        let dir = ["in", "out", "route"][self.r_dir].to_string();
        let proto = ["", "tcp", "udp"][self.r_proto].to_string();
        let port = self.r_port.trim().to_string();
        let from = self.r_from.trim().to_string();
        let to = self.r_to.trim().to_string();
        let comment = self.r_comment.trim().to_string();

        let from = if from.is_empty() {
            "any".to_string()
        } else {
            from
        };
        let to = if to.is_empty() { "any".to_string() } else { to };

        // direcciones: sin espacios ni comillas (se pasan como argumento único)
        for v in [&from, &to] {
            if v.contains(' ') || v.contains('\'') || v.contains('"') {
                return Err(trf(K::ErrBadAddr, lang, &[("%v", v.to_string())]));
            }
        }

        // validación de puertos: 22 · 80,443 · 5000:5010 (1-65535)
        let port_ok = |p: &str| {
            p.split(',').all(|part| {
                if part.contains(':') {
                    let mut it = part.split(':');
                    let (Some(a), Some(b)) = (it.next(), it.next()) else {
                        return false;
                    };
                    if it.next().is_some() {
                        return false;
                    }
                    match (a.parse::<u32>(), b.parse::<u32>()) {
                        (Ok(x), Ok(y)) => {
                            (1..=65535).contains(&x) && (1..=65535).contains(&y) && x <= y
                        }
                        _ => false,
                    }
                } else {
                    match part.parse::<u32>() {
                        Ok(x) => (1..=65535).contains(&x),
                        _ => false,
                    }
                }
            })
        };
        if !port.is_empty() {
            if !port_ok(&port) {
                return Err(tr(K::ErrBadPort, lang).to_string());
            }
            if (port.contains(',') || port.contains(':')) && proto.is_empty() {
                return Err(tr(K::ErrPortNeedsProto, lang).to_string());
            }
        }
        if port.is_empty() && from == "any" && to == "any" {
            return Err(tr(K::ErrRuleNeedsTarget, lang).to_string());
        }

        let mut a: Vec<String> = Vec::new();
        if dir == "route" {
            a.push("route".into());
        }
        a.push(action);
        if dir != "route" {
            a.push(dir);
        }
        if !proto.is_empty() {
            a.push("proto".into());
            a.push(proto);
        }
        a.push("from".into());
        a.push(from);
        a.push("to".into());
        a.push(to);
        if !port.is_empty() {
            a.push("port".into());
            a.push(port);
        }
        if !comment.is_empty() {
            a.push("comment".into());
            a.push(comment);
        }
        Ok(a)
    }

    // ------------------------------------------------------------ ventana

    fn hide_window(&mut self, ctx: &egui::Context) {
        self.hidden.store(true, Ordering::Relaxed);
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        if let Some(t) = &self.tray {
            t.update(|_| {});
        }
        report::event(
            report::Level::Info,
            "interfaz",
            "Ventana oculta (controlada desde la bandeja)",
        );
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        self.hidden.store(false, Ordering::Relaxed);
        if let Some(t) = &self.tray {
            t.update(|_| {});
        }
        report::event(
            report::Level::Info,
            "interfaz",
            "Ventana mostrada desde la bandeja",
        );
    }

    fn quit(&mut self) {
        report::event(
            report::Level::Info,
            "inicio",
            "La aplicación se cierra a petición del usuario",
        );
        self.cfg.save();
        std::process::exit(0);
    }

    fn poll_apps(&mut self) {
        if let Some(rx) = &self.apps_rx {
            match rx.try_recv() {
                Ok(out) => {
                    self.apps = parse_app_list(&out);
                    self.apps_loading = false;
                    self.apps_rx = None;
                    let n = self.apps.len();
                    self.eng.log_add(
                        LogKind::Info,
                        trf(K::AppsLoadedFmt, self.cfg.lang, &[("%n", n.to_string())]),
                    );
                    report::event(
                        report::Level::Ok,
                        "ufw",
                        format!("Lista de perfiles de aplicación recibida: {n} perfiles"),
                    );
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.apps_loading = false;
                    self.apps_rx = None;
                }
            }
        }
    }

    // ------------------------------------------------------------ recorridos de prueba

    fn autotour_step(&mut self, n: usize) {
        self.tab = match (n - 1) % 6 {
            0 => ui::Tab::Rules,
            1 => ui::Tab::Profiles,
            2 => ui::Tab::Advanced,
            3 => ui::Tab::Report,
            4 => ui::Tab::About,
            _ => ui::Tab::Dashboard,
        };
        if n >= 12 {
            self.cfg.save();
            std::process::exit(0);
        }
    }

    /// Flujo completo contra ufw real/simulado, para validar el funcionamiento
    /// de principio a fin. Devuelve true cuando hay que terminar.
    fn simflow_step(&mut self, n: usize) -> bool {
        let s = |v: &str| v.to_string();
        match n {
            1 => {
                self.run_cmds(
                    "ufw --force enable".into(),
                    vec![vec!["--force".into(), "enable".into()]],
                    true,
                );
            }
            2 => self.tab = ui::Tab::Rules,
            3 => {
                let args: Vec<String> = vec![
                    s("allow"),
                    s("in"),
                    s("proto"),
                    s("tcp"),
                    s("from"),
                    s("any"),
                    s("to"),
                    s("any"),
                    s("port"),
                    s("22"),
                    s("comment"),
                    s("SSH administrativo"),
                ];
                self.run_cmds(ufw::pretty_cmd("ufw", &args), vec![args], true);
            }
            4 => {
                let args: Vec<String> = vec![
                    s("allow"),
                    s("in"),
                    s("proto"),
                    s("tcp"),
                    s("from"),
                    s("any"),
                    s("to"),
                    s("any"),
                    s("port"),
                    s("80,443"),
                    s("comment"),
                    s("Web"),
                ];
                self.run_cmds(ufw::pretty_cmd("ufw", &args), vec![args], true);
            }
            5 => {
                self.tab = ui::Tab::Profiles;
                let cmds = ui::profile_cmds(1);
                let label = cmds
                    .iter()
                    .map(|c| ufw::pretty_cmd("ufw", c))
                    .collect::<Vec<_>>()
                    .join(" && ");
                self.run_cmds(label, cmds, true);
            }
            6 => {
                self.tab = ui::Tab::Advanced;
                self.run_cmds(
                    "ufw logging low".into(),
                    vec![vec!["logging".into(), "low".into()]],
                    true,
                );
            }
            7 => {
                // borrado intencional de una regla inexistente: genera un
                // ERROR real en el informe para validar el modo «solo errores»
                self.tab = ui::Tab::Rules;
                self.run_cmds(
                    "ufw --force delete 9".into(),
                    vec![vec!["--force".into(), "delete".into(), "9".into()]],
                    true,
                );
            }
            8 => {
                // exportación COMPLETA del informe
                self.tab = ui::Tab::Report;
                let dest = report::expand_path("~/Informe-Centinela-UFW-simulacion-completo.txt");
                self.export_report_to(&dest);
            }
            9 => {
                // NUEVO 1.2.0: cambiar el nivel de detalle a «solo errores»:
                // la vista, la copia y la exportación muestran únicamente errores
                self.tab = ui::Tab::Report;
                self.cfg.report_detail = ReportDetail::ErrorsOnly;
                self.cfg.save();
                report::event(
                    report::Level::Info,
                    "informe",
                    "Nivel de detalle del informe cambiado a «Solo errores» (la vista, la copia y la exportación muestran solo los errores)",
                );
                self.eng.log_add(
                    LogKind::Info,
                    "Nivel de detalle del informe: Solo errores".into(),
                );
            }
            10 => {
                // exportación con el nivel «solo errores» activo
                self.tab = ui::Tab::Report;
                let dest =
                    report::expand_path("~/Informe-Centinela-UFW-simulacion-solo-errores.txt");
                self.export_report_to(&dest);
            }
            11 => {
                // volver a «completo» y regresar a Reglas para borrar una regla
                self.tab = ui::Tab::Rules;
                self.cfg.report_detail = ReportDetail::Full;
                self.cfg.save();
            }
            12 => {
                // NUEVO 1.3.0: borrar el registro automático en disco
                // (informe.log e informe.log.1). Los pasos siguientes deben
                // quedar grabados en el fichero recreado, demostrando que el
                // registro sigue activo desde cero.
                self.tab = ui::Tab::Report;
                if let Some(rep) = report::global() {
                    match rep.clear_auto_log() {
                        Ok(()) => {
                            self.eng.log_add(
                                LogKind::Ok,
                                "Registro automático borrado correctamente (el registro sigue activo desde ahora).".into(),
                            );
                            report::event(
                                report::Level::Ok,
                                "informe",
                                "Registro automático borrado en el flujo de simulación; los eventos siguientes se escriben desde cero",
                            );
                        }
                        Err(e) => {
                            self.eng.log_add(
                                LogKind::Err,
                                format!("No se pudo borrar el registro automático: {e}"),
                            );
                            report::event(
                                report::Level::Err,
                                "informe",
                                format!("Fallo borrando el registro automático: {e}"),
                            );
                        }
                    }
                }
            }
            13 => {
                // borrado real de la regla 2 (después del borrado del registro)
                self.tab = ui::Tab::Rules;
                self.run_cmds(
                    "ufw --force delete 2".into(),
                    vec![vec!["--force".into(), "delete".into(), "2".into()]],
                    true,
                );
            }
            14 => {
                // modo simulación: se muestran los comandos sin ejecutarlos
                self.cfg.dry_run = true;
                self.eng.dry_run.store(true, Ordering::Relaxed);
                let args: Vec<String> = vec![
                    s("allow"),
                    s("in"),
                    s("proto"),
                    s("tcp"),
                    s("from"),
                    s("any"),
                    s("to"),
                    s("any"),
                    s("port"),
                    s("8080"),
                    s("comment"),
                    s("prueba simulada"),
                ];
                self.run_cmds(ufw::pretty_cmd("ufw", &args), vec![args], true);
            }
            15 => {
                self.cfg.dry_run = false;
                self.eng.dry_run.store(false, Ordering::Relaxed);
                self.tab = ui::Tab::About;
                self.cfg.save();
            }
            16 => return true,
            _ => {}
        }
        false
    }

    /// Exporta el informe a `dest` con el nivel de detalle configurado y
    /// registra el resultado (usado por el recorrido de prueba).
    fn export_report_to(&mut self, dest: &std::path::Path) {
        match report::export_to(dest, self.cfg.report_detail) {
            Ok(lines) => {
                let msg = trf(
                    K::ReportExportedFmt,
                    self.cfg.lang,
                    &[
                        ("%f", dest.display().to_string()),
                        ("%n", lines.to_string()),
                    ],
                );
                self.eng.log_add(LogKind::Ok, msg);
                report::event(
                    report::Level::Ok,
                    "informe",
                    format!(
                        "Informe exportado a {}: {lines} líneas (nivel: {})",
                        dest.display(),
                        if self.cfg.report_detail.errors_only() {
                            "solo errores"
                        } else {
                            "completo"
                        }
                    ),
                );
            }
            Err(e) => {
                let msg = trf(K::ReportExportErr, self.cfg.lang, &[("%e", e.clone())]);
                self.eng.log_add(LogKind::Err, msg);
                report::event(
                    report::Level::Err,
                    "informe",
                    format!("Fallo exportando a {}: {e}", dest.display()),
                );
            }
        }
    }
}

fn parse_app_list(out: &str) -> Vec<String> {
    out.lines()
        .skip(1) // "Available applications:"
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

// ---------------------------------------------------------------- eframe App

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- comandos de la bandeja
        while let Ok(c) = self.tray_rx.try_recv() {
            match c {
                TrayCmd::ShowHide => {
                    if self.hidden.load(Ordering::Relaxed) {
                        self.show_window(ctx);
                    } else {
                        self.hide_window(ctx);
                    }
                }
                TrayCmd::Enable => {
                    self.run_cmds(
                        "ufw --force enable".into(),
                        vec![vec!["--force".into(), "enable".into()]],
                        true,
                    );
                }
                TrayCmd::Disable => {
                    self.run_cmds("ufw disable".into(), vec![vec!["disable".into()]], true);
                }
                TrayCmd::Quit => self.quit(),
            }
        }

        // --- sincronizar idioma
        if self.cfg.lang != *lock_ok(&self.lang_shared) {
            *lock_ok(&self.lang_shared) = self.cfg.lang;
            ctx.send_viewport_cmd(ViewportCommand::Title(
                tr(K::AppName, self.cfg.lang).to_string(),
            ));
            if let Some(t) = &self.tray {
                t.update(|_| {});
            }
            self.cfg.save();
        }

        // --- cierre de la ventana: guardar ajustes y salir con normalidad
        if ctx.input(|i| i.viewport().close_requested()) {
            self.cfg.save();
        }

        // --- estado activo compartido con la bandeja
        let st = lock_ok(&self.eng.state).clone();
        let active = st.active;
        if active != self.last_active {
            self.active_flag
                .store(active == Some(true), Ordering::Relaxed);
            self.last_active = active;
            if let Some(t) = &self.tray {
                t.update(|_| {});
            }
        }

        // --- resultados de «ufw app list»
        self.poll_apps();

        // --- ventana oculta: esperar sin pintar
        if self.hidden.load(Ordering::Relaxed) {
            ctx.request_repaint_after(std::time::Duration::from_millis(800));
            return;
        }

        // --- recorridos automáticos (pruebas/capturas)
        if let Some((kind, t, n)) = &mut self.tour {
            if t.elapsed().as_millis() > 2200 {
                *t = std::time::Instant::now();
                *n += 1;
                let step = *n;
                let exit = match kind {
                    TourKind::AutoTour => {
                        self.autotour_step(step);
                        false
                    }
                    TourKind::SimFlow => self.simflow_step(step),
                };
                if exit {
                    std::process::exit(0);
                }
            }
        }

        // --- interfaz
        ui::draw_top(self, ctx);
        ui::draw_sidebar(self, ctx);
        ui::draw_log(self, ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin {
                        left: 6.0,
                        right: 14.0,
                        top: 6.0,
                        bottom: 4.0,
                    }),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        ui::Tab::Dashboard => ui::draw_dashboard(self, ui),
                        ui::Tab::Rules => ui::draw_rules(self, ui),
                        ui::Tab::Profiles => ui::draw_profiles(self, ui),
                        ui::Tab::Advanced => ui::draw_advanced(self, ui),
                        ui::Tab::Report => ui::draw_report(self, ui),
                        ui::Tab::About => ui::draw_about(self, ui),
                    });
            });

        self.draw_confirm(ctx);

        // --- ritmo de repintado
        let busy = self.eng.busy.load(Ordering::Relaxed);
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        } else if self.tab == ui::Tab::Dashboard {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(900));
        }
    }
}

// ---------------------------------------------------------------- main

fn load_window_icon() -> Option<Arc<egui::IconData>> {
    let img = image::load_from_memory(include_bytes!("../assets/icon_256.png")).ok()?;
    let rgba = img.to_rgba8();
    Some(Arc::new(egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    }))
}

fn print_help() {
    println!(
        "Centinela UFW (UFW Sentinel) v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!("Creado por Leyter Vázquez . V — GNU GPL v3");
    println!();
    println!("Uso: ufw-sentinel [opción]");
    println!("  (sin opciones)  Abre la interfaz gráfica");
    println!("  --version, -v   Muestra la versión y la licencia");
    println!("  --help, -h      Muestra esta ayuda");
    println!("  --demo          Modo demostración (sin tocar el firewall)");
    println!("  --autotour      Demo + recorrido automático (uso interno/pruebas)");
    println!("  --simflow       Flujo completo de prueba contra ufw real/simulado (uso interno)");
}

fn main() -> Result<(), eframe::Error> {
    let mut demo = false;
    let mut tour: Option<TourKind> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    for a in &args {
        match a.as_str() {
            "--version" | "-v" => {
                println!(
                    "{} v{}",
                    tr(K::AppName, Lang::Es),
                    env!("CARGO_PKG_VERSION")
                );
                println!(
                    "{}: {}",
                    tr(K::CreatedBy, Lang::Es),
                    tr(K::AuthorName, Lang::Es)
                );
                println!("Licencia / License: GNU GPL v3");
                println!("https://www.gnu.org/licenses/gpl-3.0.html");
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--demo" => demo = true,
            "--autotour" => {
                demo = true;
                tour = Some(TourKind::AutoTour);
            }
            "--simflow" => tour = Some(TourKind::SimFlow),
            other => {
                eprintln!("ufw-sentinel: opción no reconocida: «{other}»");
                eprintln!("Prueba «ufw-sentinel --help» para ver las opciones disponibles.");
                std::process::exit(2);
            }
        }
    }

    let cfg = Config::load();
    let mut vp = egui::ViewportBuilder::default()
        .with_inner_size([1080.0, 720.0])
        .with_min_inner_size([940.0, 640.0])
        .with_app_id("ufw-sentinel");
    if let Some(icon) = load_window_icon() {
        vp = vp.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport: vp,
        ..Default::default()
    };

    let title = tr(K::AppName, cfg.lang).to_string();
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, cfg, demo, tour)))),
    )
}
