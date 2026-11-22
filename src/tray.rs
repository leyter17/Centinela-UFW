//! Bandeja del sistema vía StatusNotifierItem (ksni, Rust puro con zbus).
//!
//! Menú: mostrar/ocultar ventana, activar/desactivar firewall y salir.
//! El icono cambia según el estado del firewall.

use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, ToolTip, Tray};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::i18n::{tr, Lang, K};

pub enum TrayCmd {
    ShowHide,
    Enable,
    Disable,
    Quit,
}

pub struct TrayData {
    pub active: Arc<AtomicBool>,
    pub hidden: Arc<AtomicBool>,
    pub lang: Arc<Mutex<Lang>>,
    pub tx: Sender<TrayCmd>,
    pub ctx: eframe::egui::Context,
    pub icon_on: Vec<u8>,
    pub icon_off: Vec<u8>,
}

fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    let mut data = rgba.to_vec();
    for px in data.chunks_exact_mut(4) {
        px.rotate_right(1); // RGBA -> ARGB
    }
    data
}

impl TrayData {
    fn lang(&self) -> Lang {
        *self.lang.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl Tray for TrayData {
    fn id(&self) -> String {
        "ufw-sentinel".into()
    }

    fn category(&self) -> ksni::Category {
        ksni::Category::SystemServices
    }

    fn title(&self) -> String {
        let l = self.lang();
        tr(K::AppName, l).to_string()
    }

    fn status(&self) -> ksni::Status {
        ksni::Status::Active
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let png = if self.active.load(Ordering::Relaxed) {
            &self.icon_on
        } else {
            &self.icon_off
        };
        if let Ok(img) = image::load_from_memory(png) {
            let rgba = img.to_rgba8();
            return vec![Icon {
                width: rgba.width() as i32,
                height: rgba.height() as i32,
                data: rgba_to_argb(rgba.as_raw()),
            }];
        }
        Vec::new()
    }

    fn tool_tip(&self) -> ToolTip {
        let l = self.lang();
        let active = self.active.load(Ordering::Relaxed);
        ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: tr(K::AppName, l).to_string(),
            description: tr(
                if active {
                    K::TrayTipActive
                } else {
                    K::TrayTipInactive
                },
                l,
            )
            .to_string(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCmd::ShowHide);
        self.ctx.request_repaint();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let l = self.lang();
        let hidden = self.hidden.load(Ordering::Relaxed);
        let active = self.active.load(Ordering::Relaxed);
        let ctx = self.ctx.clone();
        vec![
            StandardItem {
                label: if hidden {
                    tr(K::TrayShow, l).into()
                } else {
                    tr(K::TrayHide, l).into()
                },
                activate: Box::new(move |t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::ShowHide);
                    ctx.request_repaint();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: tr(K::TrayEnable, l).into(),
                enabled: !active,
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::Enable);
                    t.ctx.request_repaint();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: tr(K::TrayDisable, l).into(),
                enabled: active,
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::Disable);
                    t.ctx.request_repaint();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: tr(K::TrayExit, l).into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send(TrayCmd::Quit);
                    t.ctx.request_repaint();
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Lanza la bandeja en su propio hilo. Devuelve el handle para actualizarla.
pub fn start_tray(data: TrayData) -> Result<ksni::blocking::Handle<TrayData>, String> {
    use ksni::blocking::TrayMethods;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("ufw-tray".into())
        .spawn(move || {
            let res = data.spawn();
            let _ = tx.send(res);
        })
        .map_err(|e| e.to_string())?;
    match rx.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(res) => res.map_err(|e| e.to_string()),
        Err(_) => Err("la bandeja no respondió".to_string()),
    }
}
