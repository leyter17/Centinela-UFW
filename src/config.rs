//! Configuración persistente de la aplicación.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::i18n::{tr, Lang, K};

/// Nivel de detalle del informe de actividad (pestaña «Informe»). Afecta a
/// lo que se muestra en la vista de sesión, a «Copiar al portapapeles» y a
/// la exportación a .txt. El fichero automático en disco conserva siempre
/// el historial completo.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReportDetail {
    /// Registra y muestra todas las operaciones, comandos y detalles.
    #[default]
    Full,
    /// Muestra y exporta únicamente los eventos de error.
    ErrorsOnly,
}

impl ReportDetail {
    /// true si este nivel solo deja pasar errores.
    pub fn errors_only(self) -> bool {
        self == ReportDetail::ErrorsOnly
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config {
    /// Idioma de la interfaz (español por defecto).
    pub lang: Lang,
    /// Modo simulación: mostrar comandos sin ejecutarlos.
    pub dry_run: bool,
    /// Nivel de detalle del informe (nuevo en 1.2.0; por defecto completo).
    #[serde(default)]
    pub report_detail: ReportDetail,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lang: Lang::Es,
            dry_run: false,
            report_detail: ReportDetail::default(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ufw-sentinel");
    base.join("config.json")
}

impl Config {
    pub fn load() -> Config {
        let p = config_path();
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(dir) = config_path().parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                crate::report::event(
                    crate::report::Level::Err,
                    "config",
                    format!("No se pudo crear {}: {}", dir.display(), e),
                );
                return;
            }
        }
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(config_path(), &json) {
                    crate::report::event(
                        crate::report::Level::Err,
                        "config",
                        format!("No se pudo guardar {}: {}", config_path().display(), e),
                    );
                } else {
                    crate::report::event(
                        crate::report::Level::Info,
                        "config",
                        format!("{} ({})", tr(K::Saved, self.lang), config_path().display()),
                    );
                }
            }
            Err(e) => {
                crate::report::event(
                    crate::report::Level::Err,
                    "config",
                    format!("Error serializando la configuración: {e}"),
                );
            }
        }
    }
}
