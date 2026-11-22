//! Interfaz de usuario de Centinela UFW (egui immediate mode).

use eframe::egui;
use egui::{Align, Color32, ComboBox, RichText, Rounding, Stroke, Vec2};
use std::sync::atomic::Ordering;

use crate::config::ReportDetail;
use crate::i18n::{tr, trf, Lang, K};
use crate::report;
use crate::theme::*;
use crate::ufw::{lock_ok, LogKind, Op};
use crate::App;

pub const LOG_LEVELS: [&str; 5] = ["off", "low", "medium", "high", "full"];

// ---------------------------------------------------------------- helpers

fn card<R>(ui: &mut egui::Ui, title: Option<&str>, add: impl FnOnce(&mut egui::Ui) -> R) {
    egui::Frame::default()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(14.0))
        .inner_margin(egui::Margin::same(14.0))
        .outer_margin(egui::Margin {
            bottom: 4.0,
            ..Default::default()
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(t) = title {
                ui.label(RichText::new(t).size(16.0).strong().color(TEXT));
                ui.add_space(6.0);
            }
            add(ui);
        });
}

fn dot(ui: &mut egui::Ui, color: Color32, radius: f32) {
    let t = ui.input(|i| i.time) as f32;
    let pulse = radius * 0.22 * t.sin();
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(radius * 2.0 + 4.0), egui::Sense::hover());
    let c = rect.center();
    ui.painter()
        .circle_filled(c, radius + 4.0 + pulse, color.gamma_multiply(0.18));
    ui.painter().circle_filled(c, radius + pulse * 0.5, color);
}

fn chip(ui: &mut egui::Ui, text: &str, color: Color32, filled: bool) -> egui::Response {
    let txt = RichText::new(text).size(12.5).strong();
    let btn = egui::Button::new(txt)
        .rounding(Rounding::same(20.0))
        .fill(if filled {
            color.gamma_multiply(0.25)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.7)))
        .small();
    ui.add(btn)
}

fn primary_btn(
    ui: &mut egui::Ui,
    text: &str,
    w: f32,
    h: f32,
    fill: Color32,
    enabled: bool,
) -> bool {
    let btn = egui::Button::new(
        RichText::new(text)
            .size(14.5)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(fill)
    .rounding(Rounding::same(10.0))
    .min_size(Vec2::new(w, h));
    ui.add_enabled(enabled, btn).clicked()
}

fn dim_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(TEXT_DIM));
}

fn key_val(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(k).size(13.0).color(TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(v).size(13.0).strong().color(TEXT));
        });
    });
}

fn action_color(a: &str) -> Color32 {
    match a {
        "ALLOW" => GREEN,
        "DENY" => RED,
        "REJECT" => Color32::from_rgb(0xFB, 0x92, 0x6A),
        "LIMIT" => AMBER,
        _ => TEXT_DIM,
    }
}

// ---------------------------------------------------------------- modal

pub fn modal_dialog(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    body: &str,
    ok_label: &str,
    ok_color: Color32,
    lang: Lang,
) -> Option<bool> {
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return Some(false);
    }
    let screen = ctx.input(|i| i.screen_rect());
    let dim = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new((id, "dim")),
    ));
    dim.rect_filled(screen, 0.0, Color32::from_black_alpha(160));

    let mut result = None;
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(SURFACE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .rounding(Rounding::same(16.0))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_max_width(440.0);
                    ui.label(RichText::new(title).size(17.0).strong().color(TEXT));
                    ui.add_space(10.0);
                    ui.label(RichText::new(body).size(13.5).color(TEXT_DIM));
                    ui.add_space(16.0);
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if primary_btn(ui, ok_label, 110.0, 34.0, ok_color, true) {
                            result = Some(true);
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new(tr(K::Cancel, lang)).size(14.0))
                                    .rounding(Rounding::same(10.0))
                                    .min_size(Vec2::new(110.0, 34.0)),
                            )
                            .clicked()
                        {
                            result = Some(false);
                        }
                    });
                });
        });
    result
}

// ---------------------------------------------------------------- marco

pub fn draw_top(app: &mut App, ctx: &egui::Context) {
    let lang = app.cfg.lang;
    egui::TopBottomPanel::top("top_bar")
        .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin {
            left: 14.0,
            right: 14.0,
            top: 10.0,
            bottom: 4.0,
        }))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(logo) = app.logo.clone() {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        &logo,
                        Vec2::splat(26.0),
                    )));
                }
                ui.label(
                    RichText::new(tr(K::AppName, lang))
                        .size(19.0)
                        .strong()
                        .color(TEXT),
                );
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(11.5)
                        .color(GREEN_PALE)
                        .background_color(GREEN.gamma_multiply(0.15)),
                );
                if app.eng.demo {
                    chip(ui, tr(K::DemoBadge, lang), AMBER, true);
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    // botón minimizar (comportamiento normal del escritorio)
                    let min_resp = ui.add(
                        egui::Button::new(RichText::new("–").size(16.0).strong())
                            .fill(SURFACE2)
                            .rounding(Rounding::same(8.0))
                            .min_size(Vec2::new(34.0, 30.0)),
                    );
                    if min_resp.clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    min_resp.on_hover_text(tr(K::MinimizeBtn, lang));

                    // idioma
                    ComboBox::from_id_salt("lang_sel")
                        .selected_text(lang.name())
                        .width(108.0)
                        .show_ui(ui, |ui| {
                            for l in Lang::all() {
                                ui.selectable_value(&mut app.cfg.lang, l, l.name());
                            }
                        });
                    // chip simulación
                    let dry = app.cfg.dry_run;
                    let r = chip(
                        ui,
                        tr(K::DryRun, lang),
                        if dry { AMBER } else { TEXT_DIM },
                        dry,
                    );
                    if r.clicked() {
                        app.cfg.dry_run = !dry;
                        app.eng.dry_run.store(app.cfg.dry_run, Ordering::Relaxed);
                        app.cfg.save();
                    }
                    r.on_hover_text(tr(K::DryRunHint, lang));
                });
            });
        });
}

pub fn draw_sidebar(app: &mut App, ctx: &egui::Context) {
    let lang = app.cfg.lang;
    egui::SidePanel::left("nav")
        .exact_width(188.0)
        .frame(egui::Frame::none().fill(BG).inner_margin(egui::Margin {
            left: 10.0,
            right: 10.0,
            top: 6.0,
            bottom: 10.0,
        }))
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.add_space(4.0);
                let items = [
                    (Tab::Dashboard, tr(K::NavDashboard, lang)),
                    (Tab::Rules, tr(K::NavRules, lang)),
                    (Tab::Profiles, tr(K::NavProfiles, lang)),
                    (Tab::Advanced, tr(K::NavAdvanced, lang)),
                    (Tab::Report, tr(K::NavReport, lang)),
                    (Tab::About, tr(K::NavAbout, lang)),
                ];
                for (tab, label) in items {
                    let selected = app.tab == tab;
                    let txt = RichText::new(label).size(15.0).strong();
                    let txt = if selected {
                        txt.color(GREEN_PALE)
                    } else {
                        txt.color(TEXT)
                    };
                    let btn = egui::SelectableLabel::new(selected, txt);
                    if ui.add_sized([ui.available_width(), 34.0], btn).clicked() {
                        app.tab = tab;
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(Align::Min), |ui| {
                    ui.add_space(8.0);
                    // tarjeta de estado mini
                    let st = lock_ok(&app.eng.state).clone();
                    egui::Frame::default()
                        .fill(SURFACE)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (color, txt) = match st.active {
                                    Some(true) => (GREEN, "UFW · ON"),
                                    Some(false) => (RED, "UFW · OFF"),
                                    None => (TEXT_DIM, "UFW · ?"),
                                };
                                dot(ui, color, 5.0);
                                ui.label(RichText::new(txt).size(12.5).strong().color(TEXT));
                            });
                            ui.add_space(4.0);
                            let busy = app.eng.busy.load(Ordering::Relaxed);
                            let label = if busy {
                                RichText::new(tr(K::Refreshing, lang))
                                    .size(12.0)
                                    .color(TEXT_DIM)
                            } else {
                                RichText::new(tr(K::Refresh, lang)).size(12.5).color(TEXT)
                            };
                            if ui
                                .add_sized(
                                    [ui.available_width(), 28.0],
                                    egui::Button::new(label)
                                        .fill(SURFACE2)
                                        .rounding(Rounding::same(8.0)),
                                )
                                .clicked()
                            {
                                app.eng.submit(Op::Refresh);
                                report::event(
                                    report::Level::Info,
                                    "ufw",
                                    "Actualización de estado solicitada manualmente",
                                );
                            }
                        });
                });
            });
        });
}

pub fn draw_log(app: &mut App, ctx: &egui::Context) {
    let lang = app.cfg.lang;
    egui::TopBottomPanel::bottom("log_panel")
        .exact_height(148.0)
        .frame(
            egui::Frame::none()
                .fill(SURFACE)
                .inner_margin(egui::Margin {
                    left: 14.0,
                    right: 14.0,
                    top: 8.0,
                    bottom: 8.0,
                }),
        )
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr(K::LogTitle, lang))
                            .size(12.5)
                            .strong()
                            .color(TEXT_DIM),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new(tr(K::ClearLog, lang)).size(11.5))
                                    .fill(Color32::TRANSPARENT)
                                    .rounding(Rounding::same(6.0))
                                    .small(),
                            )
                            .clicked()
                        {
                            lock_ok(&app.eng.log).clear();
                        }
                    });
                });
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        let logs = lock_ok(&app.eng.log).clone();
                        if logs.is_empty() {
                            dim_label(ui, tr(K::LogEmpty, lang));
                        }
                        for e in logs {
                            let color = match e.kind {
                                LogKind::Info => TEXT_DIM,
                                LogKind::Ok => GREEN_PALE,
                                LogKind::Warn => AMBER,
                                LogKind::Err => RED,
                                LogKind::Cmd => BLUE,
                            };
                            let mins = e.secs / 60;
                            let secs = e.secs % 60;
                            ui.label(
                                RichText::new(format!("[{mins:02}:{secs:02}] {}", e.text))
                                    .size(11.5)
                                    .color(color)
                                    .monospace(),
                            );
                        }
                    });
            });
        });
}

// ---------------------------------------------------------------- pestañas

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Dashboard,
    Rules,
    Profiles,
    Advanced,
    Report,
    About,
}

pub fn draw_dashboard(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    let busy = app.eng.busy.load(Ordering::Relaxed);
    let st = lock_ok(&app.eng.state).clone();

    // ---- tarjeta principal de estado
    card(ui, None, |ui| {
        ui.horizontal(|ui| {
            let (color, txt) = match st.active {
                Some(true) => (GREEN, tr(K::FirewallActive, lang)),
                Some(false) => (RED, tr(K::FirewallInactive, lang)),
                None => (
                    TEXT_DIM,
                    if busy {
                        tr(K::StatusChecking, lang)
                    } else {
                        tr(K::FirewallUnknown, lang)
                    },
                ),
            };
            dot(ui, color, 9.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(txt).size(20.0).strong().color(TEXT));
                dim_label(ui, tr(K::Tagline, lang));
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let enabled = !busy && st.active.is_some() && !app.eng.demo;
                if st.active.is_some() {
                    let activar = st.active == Some(false);
                    let label = if activar {
                        tr(K::BtnEnable, lang)
                    } else {
                        tr(K::BtnDisable, lang)
                    };
                    let color = if activar { GREEN_DEEP } else { RED_DEEP };
                    if primary_btn(ui, label, 165.0, 42.0, color, enabled) {
                        if activar {
                            app.run_cmds(
                                "ufw --force enable".into(),
                                vec![vec!["--force".into(), "enable".into()]],
                                true,
                            );
                        } else {
                            app.confirm = Some(ConfirmKind::Disable);
                        }
                    }
                }
            });
        });
        if busy {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(14.0).color(GREEN));
                dim_label(ui, tr(K::Working, lang));
            });
        }
    });

    ui.add_space(4.0);

    // ---- políticas + ajustes
    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            card(ui, Some(tr(K::DefaultPolicies, lang)), |ui| {
                let opts = [
                    tr(K::PolicyDeny, lang),
                    tr(K::PolicyAllow, lang),
                    tr(K::PolicyReject, lang),
                ];
                let vals = ["deny", "allow", "reject"];
                let mut changed: Option<(&str, String)> = None;
                for (label_k, key) in [
                    (tr(K::Incoming, lang), "in"),
                    (tr(K::Outgoing, lang), "out"),
                    (tr(K::Routed, lang), "routed"),
                ] {
                    let cur = match key {
                        "in" => &st.def_in,
                        "out" => &st.def_out,
                        _ => &st.def_routed,
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(label_k).size(13.5).color(TEXT_DIM));
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            ComboBox::from_id_salt(format!("pol_{key}"))
                                .selected_text(pretty_policy(cur, lang))
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for (i, o) in opts.iter().enumerate() {
                                        let sel = ui.selectable_label(cur == vals[i], *o);
                                        if sel.clicked() && cur != vals[i] {
                                            changed = Some((key, vals[i].to_string()));
                                        }
                                    }
                                });
                        });
                    });
                    ui.add_space(2.0);
                }
                if let Some((target, policy)) = changed {
                    app.run_cmds(
                        format!("ufw default {policy} {target}"),
                        vec![vec!["default".into(), policy, target.to_string()]],
                        true,
                    );
                }
            });
        });
        cols[1].vertical(|ui| {
            card(ui, Some(tr(K::QuickSettings, lang)), |ui| {
                ui.add_space(2.0);
                let mut d = app.cfg.dry_run;
                if ui.checkbox(&mut d, tr(K::DryRun, lang)).changed() {
                    app.cfg.dry_run = d;
                    app.eng.dry_run.store(d, Ordering::Relaxed);
                    app.cfg.save();
                    report::event(
                        report::Level::Info,
                        "config",
                        format!(
                            "Modo simulación: {}",
                            if d { "activado" } else { "desactivado" }
                        ),
                    );
                }
                dim_label(ui, tr(K::DryRunHint, lang));
            });
        });
    });

    ui.add_space(2.0);

    // ---- fichas de resumen
    card(ui, None, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 18.0;
            summary_chip(
                ui,
                tr(K::ActiveRules, lang),
                &trf(K::RuleCountFmt, lang, &[("%n", st.rules.len().to_string())]),
                GREEN_PALE,
            );
            summary_chip(ui, tr(K::LoggingLabel, lang), &st.logging, BLUE);
            let ipv6_txt = match st.ipv6 {
                Some(true) => tr(K::Ipv6On, lang),
                Some(false) => tr(K::Ipv6Off, lang),
                None => tr(K::Ipv6Unknown, lang),
            };
            summary_chip(
                ui,
                "IPv6",
                ipv6_txt,
                if st.ipv6 == Some(true) {
                    GREEN_PALE
                } else {
                    AMBER
                },
            );
            summary_chip(
                ui,
                tr(K::DesktopDetected, lang),
                &crate::ufw::detect_desktop(),
                TEXT_DIM,
            );
        });
    });
}

fn summary_chip(ui: &mut egui::Ui, k: &str, v: &str, color: Color32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(k).size(11.0).color(TEXT_DIM));
        ui.label(RichText::new(v).size(14.0).strong().color(color));
    });
}

fn pretty_policy(p: &str, lang: Lang) -> String {
    match p {
        "deny" => tr(K::PolicyDeny, lang).to_string(),
        "allow" => tr(K::PolicyAllow, lang).to_string(),
        "reject" => tr(K::PolicyReject, lang).to_string(),
        "disabled" => "-".into(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------- reglas

pub fn draw_rules(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    let busy = app.eng.busy.load(Ordering::Relaxed);
    let st = lock_ok(&app.eng.state).clone();

    // ---- formulario
    card(ui, Some(tr(K::AddRuleTitle, lang)), |ui| {
        egui::Grid::new("rule_form")
            .num_columns(4)
            .spacing([14.0, 8.0])
            .show(ui, |ui| {
                let actions = [
                    ("ALLOW", tr(K::PolicyAllow, lang)),
                    ("DENY", tr(K::PolicyDeny, lang)),
                    ("REJECT", tr(K::PolicyReject, lang)),
                    ("LIMIT", "limit"),
                ];
                ui.label(
                    RichText::new(tr(K::Action, lang))
                        .size(13.0)
                        .color(TEXT_DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ComboBox::from_id_salt("f_action")
                        .selected_text(actions[app.r_action].1)
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for (i, (_, label)) in actions.iter().enumerate() {
                                ui.selectable_value(&mut app.r_action, i, *label);
                            }
                        });
                });
                ui.label(
                    RichText::new(tr(K::Direction, lang))
                        .size(13.0)
                        .color(TEXT_DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let dirs = [
                        ("in", format!("{} (IN)", tr(K::Incoming, lang))),
                        ("out", format!("{} (OUT)", tr(K::Outgoing, lang))),
                        ("route", tr(K::Routed, lang).to_string()),
                    ];
                    ComboBox::from_id_salt("f_dir")
                        .selected_text(dirs[app.r_dir].1.clone())
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for (i, (_, label)) in dirs.iter().enumerate() {
                                ui.selectable_value(&mut app.r_dir, i, label.clone());
                            }
                        });
                });
                ui.end_row();

                ui.label(
                    RichText::new(tr(K::Protocol, lang))
                        .size(13.0)
                        .color(TEXT_DIM),
                );
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let protos = [
                        (format!("{} ({})", tr(K::ProtoAny, lang), "tcp+udp")),
                        ("TCP".to_string()),
                        ("UDP".to_string()),
                    ];
                    ComboBox::from_id_salt("f_proto")
                        .selected_text(protos[app.r_proto].clone())
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for (i, label) in protos.iter().enumerate() {
                                ui.selectable_value(&mut app.r_proto, i, label.clone());
                            }
                        });
                });
                ui.label(RichText::new(tr(K::Port, lang)).size(13.0).color(TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut app.r_port)
                        .hint_text(tr(K::PortHint, lang))
                        .desired_width(170.0)
                        .font(egui::TextStyle::Monospace),
                );
                ui.end_row();

                ui.label(RichText::new(tr(K::From, lang)).size(13.0).color(TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut app.r_from)
                        .hint_text(tr(K::FromHint, lang))
                        .desired_width(170.0)
                        .font(egui::TextStyle::Monospace),
                );
                ui.label(RichText::new(tr(K::To, lang)).size(13.0).color(TEXT_DIM));
                ui.add(
                    egui::TextEdit::singleline(&mut app.r_to)
                        .hint_text(tr(K::ToHint, lang))
                        .desired_width(170.0)
                        .font(egui::TextStyle::Monospace),
                );
                ui.end_row();

                ui.label(
                    RichText::new(tr(K::Comment, lang))
                        .size(13.0)
                        .color(TEXT_DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.r_comment)
                        .hint_text(tr(K::CommentHint, lang))
                        .desired_width(354.0),
                );
                ui.end_row();
            });

        ui.add_space(8.0);
        // previsualización
        match app.rule_args() {
            Ok(args) => {
                dim_label(ui, tr(K::LivePreview, lang));
                ui.label(
                    RichText::new(crate::ufw::pretty_cmd("ufw", &args))
                        .size(12.5)
                        .color(GREEN_PALE)
                        .monospace(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if primary_btn(ui, tr(K::BtnAddRule, lang), 130.0, 34.0, GREEN_DEEP, !busy) {
                        app.run_cmds(crate::ufw::pretty_cmd("ufw", &args), vec![args], true);
                        app.clear_form();
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new(tr(K::BtnClearForm, lang)).size(13.5))
                                .fill(SURFACE2)
                                .rounding(Rounding::same(10.0))
                                .min_size(Vec2::new(130.0, 34.0)),
                        )
                        .clicked()
                    {
                        app.clear_form();
                    }
                });
            }
            Err(e) => {
                ui.label(RichText::new(e).size(13.0).color(RED));
            }
        }
    });

    ui.add_space(4.0);

    // ---- tabla de reglas
    card(ui, Some(tr(K::RulesTitle, lang)), |ui| {
        if st.rules.is_empty() {
            dim_label(ui, tr(K::RulesEmpty, lang));
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(280.0)
                .show(ui, |ui| {
                    egui::Grid::new("rules_grid")
                        .striped(true)
                        .num_columns(6)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(tr(K::ColNum, lang))
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_DIM),
                            );
                            ui.label(
                                RichText::new(tr(K::ColTo, lang))
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_DIM),
                            );
                            ui.label(
                                RichText::new(tr(K::ColAction, lang))
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_DIM),
                            );
                            ui.label(
                                RichText::new(tr(K::ColFrom, lang))
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_DIM),
                            );
                            ui.label(
                                RichText::new(tr(K::ColComment, lang))
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_DIM),
                            );
                            ui.end_row();
                            for r in &st.rules {
                                let to_txt = if r.v6 {
                                    format!("{} (v6)", r.to)
                                } else {
                                    r.to.clone()
                                };
                                let from_txt = if r.v6 {
                                    format!("{} (v6)", r.from)
                                } else {
                                    r.from.clone()
                                };
                                ui.label(
                                    RichText::new(r.num.to_string())
                                        .size(12.5)
                                        .monospace()
                                        .color(TEXT_DIM),
                                );
                                ui.label(RichText::new(to_txt).size(12.5).monospace().color(TEXT));
                                let act = RichText::new(r.action.clone())
                                    .size(12.5)
                                    .strong()
                                    .color(action_color(&r.action));
                                ui.label(act);
                                ui.label(
                                    RichText::new(from_txt).size(12.5).monospace().color(TEXT),
                                );
                                ui.label(
                                    RichText::new(if r.comment.is_empty() {
                                        "-"
                                    } else {
                                        &r.comment
                                    })
                                    .size(12.0)
                                    .color(TEXT_DIM),
                                );
                                let del = ui.add(
                                    egui::Button::new(
                                        RichText::new(tr(K::BtnDelete, lang)).size(11.5).color(RED),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::new(1.0_f32, RED.gamma_multiply(0.55)))
                                    .rounding(Rounding::same(6.0))
                                    .small(),
                                );
                                if del.clicked() {
                                    app.confirm = Some(ConfirmKind::DeleteRule(r.num));
                                }
                                ui.end_row();
                            }
                        });
                });
            ui.add_space(4.0);
            dim_label(ui, tr(K::RulesHint, lang));
        }
    });
}

// ---------------------------------------------------------------- perfiles

pub fn profile_cmds(idx: usize) -> Vec<Vec<String>> {
    let s = |v: &str| v.to_string();
    match idx {
        0 => vec![
            vec![s("default"), s("deny"), s("incoming")],
            vec![s("default"), s("allow"), s("outgoing")],
            vec![s("default"), s("deny"), s("routed")],
            vec![
                s("allow"),
                s("from"),
                s("192.168.0.0/16"),
                s("comment"),
                s("Red local confiable"),
            ],
        ],
        1 => vec![
            vec![s("default"), s("deny"), s("incoming")],
            vec![s("default"), s("allow"), s("outgoing")],
            vec![s("default"), s("deny"), s("routed")],
            vec![
                s("allow"),
                s("from"),
                s("192.168.0.0/16"),
                s("comment"),
                s("Red local confiable"),
            ],
            vec![s("limit"), s("22/tcp"), s("comment"), s("SSH limitado")],
        ],
        2 => vec![
            vec![s("default"), s("deny"), s("incoming")],
            vec![s("default"), s("allow"), s("outgoing")],
            vec![s("limit"), s("22/tcp"), s("comment"), s("SSH limitado")],
            vec![s("allow"), s("80/tcp")],
            vec![s("allow"), s("443/tcp")],
            vec![s("allow"), s("443/udp")],
        ],
        _ => vec![
            vec![s("default"), s("deny"), s("incoming")],
            vec![s("default"), s("allow"), s("outgoing")],
            vec![s("default"), s("deny"), s("routed")],
        ],
    }
}

pub fn profile_names(lang: Lang) -> [(&'static str, &'static str); 4] {
    [
        (tr(K::ProfileHome, lang), tr(K::ProfileHomeDesc, lang)),
        (tr(K::ProfileOffice, lang), tr(K::ProfileOfficeDesc, lang)),
        (tr(K::ProfileServer, lang), tr(K::ProfileServerDesc, lang)),
        (tr(K::ProfilePublic, lang), tr(K::ProfilePublicDesc, lang)),
    ]
}

pub fn draw_profiles(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    let busy = app.eng.busy.load(Ordering::Relaxed);

    card(ui, Some(tr(K::ProfilesTitle, lang)), |ui| {
        dim_label(ui, tr(K::ProfilesHint, lang));
        ui.add_space(6.0);
        let names = profile_names(lang);
        for (idx, (name, desc)) in names.iter().enumerate() {
            egui::Frame::default()
                .fill(SURFACE2)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width(ui.available_width() - 150.0);
                            ui.label(RichText::new(*name).size(15.0).strong().color(GREEN_PALE));
                            dim_label(ui, desc);
                        });
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if primary_btn(ui, tr(K::Apply, lang), 130.0, 34.0, GREEN_DEEP, !busy) {
                                app.confirm = Some(ConfirmKind::ApplyProfile(idx));
                            }
                        });
                    });
                    ui.add_space(2.0);
                    let cmds = profile_cmds(idx);
                    let preview = cmds
                        .iter()
                        .map(|c| crate::ufw::pretty_cmd("ufw", c))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.label(
                        RichText::new(format!("{} \n{}", tr(K::CommandsPreview, lang), preview))
                            .size(11.0)
                            .color(TEXT_DIM)
                            .monospace(),
                    );
                });
            ui.add_space(4.0);
        }
    });

    ui.add_space(4.0);

    // ---- perfiles de aplicación
    card(ui, Some(tr(K::AppsTitle, lang)), |ui| {
        dim_label(ui, tr(K::AppsHint, lang));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new(tr(K::AppsRefresh, lang)).size(13.0))
                        .fill(SURFACE2)
                        .rounding(Rounding::same(8.0)),
                )
                .clicked()
                && !busy
            {
                app.request_apps();
            }
            if app.apps_loading {
                ui.add(egui::Spinner::new().size(13.0).color(GREEN));
            }
        });
        ui.add_space(4.0);
        if app.apps.is_empty() {
            dim_label(ui, tr(K::AppsEmpty, lang));
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(200.0)
                .show(ui, |ui| {
                    for name in app.apps.clone() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&name).size(13.0).monospace().color(TEXT));
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                if primary_btn(
                                    ui,
                                    tr(K::AppAllow, lang),
                                    100.0,
                                    27.0,
                                    SURFACE3,
                                    !busy,
                                ) {
                                    app.run_cmds(
                                        format!("ufw allow {name}"),
                                        vec![vec!["allow".into(), name.clone()]],
                                        true,
                                    );
                                }
                            });
                        });
                    }
                });
        }
    });
}

// ---------------------------------------------------------------- avanzado

pub fn draw_advanced(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    let busy = app.eng.busy.load(Ordering::Relaxed);
    let st = lock_ok(&app.eng.state).clone();

    card(ui, Some(tr(K::SystemCard, lang)), |ui| {
        egui::Grid::new("sys_grid")
            .num_columns(2)
            .spacing([16.0, 5.0])
            .show(ui, |ui| {
                key_val_g(
                    ui,
                    tr(K::UfwPath, lang),
                    st.ufw_path.as_deref().unwrap_or("-"),
                );
                key_val_g(
                    ui,
                    tr(K::UfwVersion, lang),
                    st.ufw_version.as_deref().unwrap_or("-"),
                );
                key_val_g(
                    ui,
                    tr(K::PkexecDetected, lang),
                    if st.pkexec.is_some() || crate::ufw::is_root() {
                        tr(K::Yes, lang)
                    } else {
                        tr(K::No, lang)
                    },
                );
                key_val_g(
                    ui,
                    tr(K::DesktopDetected, lang),
                    &crate::ufw::detect_desktop(),
                );
                let pol = crate::ufw::polkit_installed();
                key_val_g(
                    ui,
                    tr(K::PolkitPolicy, lang),
                    if pol {
                        tr(K::PolkitInstalled, lang)
                    } else {
                        tr(K::PolkitNotInstalled, lang)
                    },
                );
            });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if primary_btn(
                ui,
                tr(K::BtnInstallPolicy, lang),
                290.0,
                32.0,
                SURFACE3,
                !busy && !app.eng.demo,
            ) {
                app.confirm = Some(ConfirmKind::InstallPolicy);
            }
        });
        dim_label(ui, tr(K::PolicyInstallNote, lang));
    });

    ui.add_space(2.0);

    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            card(ui, Some(tr(K::LoggingCard, lang)), |ui| {
                dim_label(ui, tr(K::LoggingHint, lang));
                ui.add_space(6.0);
                let levels = [
                    tr(K::LogOff, lang),
                    tr(K::LogLow, lang),
                    tr(K::LogMedium, lang),
                    tr(K::LogHigh, lang),
                    tr(K::LogFull, lang),
                ];
                for (i, label) in levels.iter().enumerate() {
                    let cur = st.logging.contains(LOG_LEVELS[i]);
                    if ui
                        .add(egui::SelectableLabel::new(
                            cur,
                            RichText::new(*label).size(13.0).color(if cur {
                                GREEN_PALE
                            } else {
                                TEXT
                            }),
                        ))
                        .clicked()
                        && !cur
                        && !busy
                    {
                        app.run_cmds(
                            format!("ufw logging {}", LOG_LEVELS[i]),
                            vec![vec!["logging".into(), LOG_LEVELS[i].to_string()]],
                            true,
                        );
                    }
                }
            });
        });
        cols[1].vertical(|ui| {
            card(ui, Some(tr(K::Ipv6Card, lang)), |ui| {
                dim_label(ui, tr(K::Ipv6Note, lang));
                ui.add_space(6.0);
                let state = match st.ipv6 {
                    Some(true) => tr(K::Ipv6On, lang),
                    Some(false) => tr(K::Ipv6Off, lang),
                    None => tr(K::Ipv6Unknown, lang),
                };
                ui.label(RichText::new(state).size(14.0).strong().color(
                    if st.ipv6 == Some(true) {
                        GREEN_PALE
                    } else {
                        AMBER
                    },
                ));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let en = !busy && !app.eng.demo && st.ipv6 != Some(true);
                    if primary_btn(ui, tr(K::BtnIpv6Enable, lang), 130.0, 30.0, GREEN_DEEP, en) {
                        app.confirm = Some(ConfirmKind::Ipv6(true));
                    }
                    let en2 = !busy && !app.eng.demo && st.ipv6 != Some(false);
                    if primary_btn(ui, tr(K::BtnIpv6Disable, lang), 130.0, 30.0, SURFACE3, en2) {
                        app.confirm = Some(ConfirmKind::Ipv6(false));
                    }
                });
            });
        });
    });

    ui.add_space(2.0);

    // ---- zona de riesgo
    egui::Frame::default()
        .fill(RED_DARK_BG)
        .stroke(Stroke::new(1.2_f32, RED.gamma_multiply(0.5)))
        .rounding(Rounding::same(14.0))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(tr(K::DangerCard, lang))
                    .size(16.0)
                    .strong()
                    .color(RED),
            );
            ui.add_space(2.0);
            dim_label(ui, tr(K::DangerNote, lang));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if primary_btn(
                    ui,
                    tr(K::BtnReload, lang),
                    140.0,
                    32.0,
                    SURFACE3,
                    !busy && !app.eng.demo,
                ) {
                    app.run_cmds("ufw reload".into(), vec![vec!["reload".into()]], true);
                }
                if primary_btn(
                    ui,
                    tr(K::BtnReset, lang),
                    220.0,
                    32.0,
                    RED_DEEP,
                    !busy && !app.eng.demo,
                ) {
                    app.confirm = Some(ConfirmKind::Reset);
                }
            });
            ui.add_space(2.0);
            dim_label(ui, tr(K::ResetNote2, lang));
        });
}

fn key_val_g(ui: &mut egui::Ui, k: &str, v: &str) {
    ui.label(RichText::new(k).size(13.0).color(TEXT_DIM));
    ui.label(RichText::new(v).size(13.0).strong().color(TEXT));
    ui.end_row();
}

// ---------------------------------------------------------------- acerca de

pub fn draw_about(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        if let Some(logo) = app.logo.clone() {
            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                &logo,
                Vec2::splat(88.0),
            )));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr(K::AppName, lang))
                .size(28.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            RichText::new(tr(K::Tagline, lang))
                .size(13.5)
                .color(TEXT_DIM),
        );
        ui.label(
            RichText::new(format!(
                "{} {}",
                tr(K::VersionLabel, lang),
                env!("CARGO_PKG_VERSION")
            ))
            .size(12.5)
            .color(GREEN_PALE),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr(K::CreatedBy, lang))
                .size(13.0)
                .color(TEXT_DIM),
        );
        ui.label(
            RichText::new(tr(K::AuthorName, lang))
                .size(21.0)
                .strong()
                .color(GREEN_PALE),
        );
        ui.add_space(14.0);
    });

    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            card(ui, Some(tr(K::LicenseTitle, lang)), |ui| {
                ui.label(
                    RichText::new("GNU GPL v3")
                        .size(17.0)
                        .strong()
                        .color(GREEN_PALE),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr(K::LicenseBody, lang))
                        .size(12.5)
                        .color(TEXT_DIM),
                );
                ui.add_space(6.0);
                ui.hyperlink_to(
                    RichText::new(tr(K::LicenseLink, lang)).size(12.5),
                    "https://www.gnu.org/licenses/gpl-3.0.html",
                );
            });
        });
        cols[1].vertical(|ui| {
            card(ui, Some(tr(K::CompatibleTitle, lang)), |ui| {
                for k in [K::CompatGnome, K::CompatKde, K::CompatCosmic, K::CompatXfce] {
                    ui.horizontal(|ui| {
                        dot_plain(ui, GREEN);
                        ui.label(RichText::new(tr(k, lang)).size(12.5).color(TEXT));
                    });
                }
                ui.add_space(8.0);
                key_val(
                    ui,
                    tr(K::DesktopDetected, lang),
                    &crate::ufw::detect_desktop(),
                );
                key_val(ui, tr(K::Language, lang), lang.name());
            });
            ui.add_space(2.0);
            card(ui, Some(tr(K::ChangelogTitle, lang)), |ui| {
                ui.label(
                    RichText::new(tr(K::ChangelogBody, lang))
                        .size(11.5)
                        .color(TEXT_DIM),
                );
            });
        });
    });
}

fn dot_plain(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}

// ---------------------------------------------------------------- confirmación

#[derive(Clone, PartialEq)]
pub enum ConfirmKind {
    Disable,
    Reset,
    ApplyProfile(usize),
    DeleteRule(u32),
    InstallPolicy,
    Ipv6(bool),
    /// Borrar el registro automático del informe en disco (1.3.0).
    ClearReportLog,
}

impl App {
    pub fn draw_confirm(&mut self, ctx: &egui::Context) {
        let lang = self.cfg.lang;
        let Some(kind) = self.confirm.clone() else {
            return;
        };
        let (title, body, ok, color) = match &kind {
            ConfirmKind::Disable => (
                tr(K::BtnDisable, lang).to_string(),
                format!(
                    "{}\n{}",
                    tr(K::FirewallActive, lang),
                    tr(K::ConfirmApplyProfile, lang)
                ),
                tr(K::BtnDisable, lang).to_string(),
                RED_DEEP,
            ),
            ConfirmKind::Reset => (
                tr(K::BtnReset, lang).to_string(),
                tr(K::ConfirmReset, lang).to_string(),
                tr(K::Confirm, lang).to_string(),
                RED_DEEP,
            ),
            ConfirmKind::ApplyProfile(i) => {
                let names = profile_names(lang);
                (
                    names[*i].0.to_string(),
                    format!(
                        "{}\n{}",
                        tr(K::ConfirmApplyProfile, lang),
                        tr(K::CommandsPreview, lang)
                    ),
                    tr(K::Apply, lang).to_string(),
                    GREEN_DEEP,
                )
            }
            ConfirmKind::DeleteRule(n) => (
                tr(K::BtnDelete, lang).to_string(),
                trf(K::ConfirmDeleteRule, lang, &[("%n", n.to_string())]),
                tr(K::Confirm, lang).to_string(),
                RED_DEEP,
            ),
            ConfirmKind::InstallPolicy => (
                tr(K::BtnInstallPolicy, lang).to_string(),
                tr(K::PolicyInstallNote, lang).to_string(),
                tr(K::Confirm, lang).to_string(),
                GREEN_DEEP,
            ),
            ConfirmKind::Ipv6(enable) => (
                if *enable {
                    tr(K::BtnIpv6Enable, lang)
                } else {
                    tr(K::BtnIpv6Disable, lang)
                }
                .to_string(),
                tr(K::Ipv6Note, lang).to_string(),
                tr(K::Confirm, lang).to_string(),
                GREEN_DEEP,
            ),
            ConfirmKind::ClearReportLog => {
                let ruta = report::global()
                    .map(|r| r.path.display().to_string())
                    .unwrap_or_default();
                (
                    tr(K::ReportClearBtn, lang).to_string(),
                    format!("{}\n\n{}", tr(K::ReportClearNote, lang), ruta),
                    tr(K::Confirm, lang).to_string(),
                    RED_DEEP,
                )
            }
        };
        if let Some(ok_click) = modal_dialog(ctx, "confirm", &title, &body, &ok, color, lang) {
            self.confirm = None;
            if ok_click {
                self.execute_confirm(kind);
            }
        }
    }

    fn execute_confirm(&mut self, kind: ConfirmKind) {
        let lang = self.cfg.lang;
        match kind {
            ConfirmKind::Disable => {
                self.run_cmds("ufw disable".into(), vec![vec!["disable".into()]], true);
            }
            ConfirmKind::Reset => {
                self.run_cmds(
                    "ufw --force reset".into(),
                    vec![vec!["--force".into(), "reset".into()]],
                    true,
                );
            }
            ConfirmKind::ApplyProfile(i) => {
                let cmds = profile_cmds(i);
                let label = cmds
                    .iter()
                    .map(|c| crate::ufw::pretty_cmd("ufw", c))
                    .collect::<Vec<_>>()
                    .join(" && ");
                self.run_cmds(label, cmds, true);
            }
            ConfirmKind::DeleteRule(n) => {
                self.run_cmds(
                    format!("ufw --force delete {n}"),
                    vec![vec!["--force".into(), "delete".into(), n.to_string()]],
                    true,
                );
            }
            ConfirmKind::InstallPolicy => {
                let st = lock_ok(&self.eng.state).clone();
                if let Some(ufw) = st.ufw_path {
                    let (script, display) = crate::ufw::polkit_install_script(&ufw);
                    self.run_script(display, script, false, Some(K::PolkitInstalledLog));
                } else {
                    self.eng
                        .log_add(LogKind::Err, tr(K::ErrNoUfw, lang).to_string());
                }
            }
            ConfirmKind::Ipv6(enable) => {
                let st = lock_ok(&self.eng.state).clone();
                if let Some(ufw) = st.ufw_path {
                    let val = if enable { "yes" } else { "no" };
                    let script = format!(
                        "sed -i 's/^IPV6=.*/IPV6={val}/' /etc/default/ufw && '{ufw}' reload"
                    );
                    let display = format!("set IPV6={val} in /etc/default/ufw && ufw reload");
                    self.run_script(display, script, true, None);
                } else {
                    self.eng
                        .log_add(LogKind::Err, tr(K::ErrNoUfw, lang).to_string());
                }
            }
            ConfirmKind::ClearReportLog => {
                if let Some(rep) = report::global() {
                    match rep.clear_auto_log() {
                        Ok(()) => {
                            self.eng.log_add(
                                LogKind::Ok,
                                tr(K::ReportCleared, lang).to_string(),
                            );
                            report::event(
                                report::Level::Ok,
                                "informe",
                                format!(
                                    "Registro automático borrado a petición del usuario; el registro sigue activo en: {}",
                                    rep.path.display()
                                ),
                            );
                            self.report_status = None;
                        }
                        Err(e) => {
                            let msg = trf(K::ReportClearErr, lang, &[("%e", e.clone())]);
                            self.eng.log_add(LogKind::Err, msg.clone());
                            report::event(
                                report::Level::Err,
                                "informe",
                                format!("Fallo borrando el registro automático: {e}"),
                            );
                            self.report_status = Some((false, msg));
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- informe

fn report_level_color(l: report::Level) -> Color32 {
    match l {
        report::Level::Info => TEXT_DIM,
        report::Level::Ok => GREEN_PALE,
        report::Level::Warn => AMBER,
        report::Level::Err => RED,
        report::Level::Cmd => BLUE,
        report::Level::Detail => TEXT_DIM,
    }
}

fn report_session_text(entries: &[report::Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        let (y, mo, d, h, mi, s) = report::epoch_to_datetime(e.ts);
        out.push_str(&format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  {}  [{}] {}\n",
            y,
            mo,
            d,
            h,
            mi,
            s,
            e.level.tag(),
            e.cat,
            e.msg
        ));
    }
    out
}

pub fn draw_report(app: &mut App, ui: &mut egui::Ui) {
    let lang = app.cfg.lang;
    let Some(rep) = report::global() else {
        dim_label(ui, "-");
        return;
    };

    // eventos de la sesión ya filtrados según el nivel de detalle elegido
    let solo_errores = app.cfg.report_detail.errors_only();
    let mut entries = rep.snapshot();
    if solo_errores {
        entries.retain(|e| e.level == report::Level::Err);
    }
    let n_mostrados = entries.len();

    // ---- tarjeta principal
    card(ui, Some(tr(K::ReportTitle, lang)), |ui| {
        dim_label(ui, tr(K::ReportHint, lang));
        ui.add_space(8.0);
        key_val(
            ui,
            tr(K::ReportFileLabel, lang),
            &rep.path.display().to_string(),
        );
        let kb = rep.file_bytes() / 1024;
        let contador = if solo_errores {
            if n_mostrados == 1 {
                trf(
                    K::ReportErrorSgFmt,
                    lang,
                    &[("%n", n_mostrados.to_string())],
                )
            } else {
                trf(K::ReportErrorsFmt, lang, &[("%n", n_mostrados.to_string())])
            }
        } else {
            let n = rep.count();
            if n == 1 {
                trf(K::ReportEntrySgFmt, lang, &[("%n", n.to_string())])
            } else {
                trf(K::ReportEntriesFmt, lang, &[("%n", n.to_string())])
            }
        };
        key_val(
            ui,
            &contador,
            &trf(K::ReportDiskSizeFmt, lang, &[("%n", kb.to_string())]),
        );
        ui.add_space(4.0);
        dim_label(ui, tr(K::ReportFullNote, lang));
        ui.add_space(8.0);

        // ---- nivel de detalle del informe
        ui.label(
            RichText::new(tr(K::ReportDetailLabel, lang))
                .size(13.5)
                .strong()
                .color(TEXT),
        );
        ui.horizontal(|ui| {
            if ui
                .radio_value(&mut app.cfg.report_detail, ReportDetail::ErrorsOnly, tr(K::ReportDetailErrorsOnly, lang))
                .changed()
            {
                app.cfg.save();
                report::event(
                    report::Level::Info,
                    "informe",
                    format!(
                        "Nivel de detalle del informe cambiado a «{}» (la vista, la copia y la exportación muestran solo los errores)",
                        tr(K::ReportDetailErrorsOnly, app.cfg.lang)
                    ),
                );
            }
            if ui
                .radio_value(&mut app.cfg.report_detail, ReportDetail::Full, tr(K::ReportDetailFull, lang))
                .changed()
            {
                app.cfg.save();
                report::event(
                    report::Level::Info,
                    "informe",
                    format!(
                        "Nivel de detalle del informe cambiado a «{}» (se registran y muestran todas las operaciones)",
                        tr(K::ReportDetailFull, app.cfg.lang)
                    ),
                );
            }
        });
        dim_label(ui, tr(K::ReportDetailNote, lang));
        ui.add_space(8.0);

        // exportación
        ui.label(
            RichText::new(tr(K::ReportPathLabel, lang))
                .size(13.0)
                .color(TEXT_DIM),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.report_export_path)
                .desired_width(ui.available_width())
                .font(egui::TextStyle::Monospace),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if primary_btn(
                ui,
                tr(K::ReportExportBtn, lang),
                170.0,
                34.0,
                GREEN_DEEP,
                true,
            ) {
                if app.report_export_path.trim().is_empty() {
                    let msg = trf(
                        K::ReportExportErr,
                        lang,
                        &[("%e", "la ruta está vacía".to_string())],
                    );
                    app.report_status = Some((false, msg));
                } else {
                    let path = report::expand_path(&app.report_export_path);
                    match report::export_to(&path, app.cfg.report_detail) {
                        Ok(lines) => {
                            let msg = trf(
                                K::ReportExportedFmt,
                                lang,
                                &[
                                    ("%f", path.display().to_string()),
                                    ("%n", lines.to_string()),
                                ],
                            );
                            report::event(
                                report::Level::Ok,
                                "informe",
                                format!(
                                    "Informe exportado a {}: {lines} líneas (nivel: {})",
                                    path.display(),
                                    if solo_errores {
                                        tr(K::ReportDetailErrorsOnly, lang)
                                    } else {
                                        tr(K::ReportDetailFull, lang)
                                    }
                                ),
                            );
                            app.eng.log_add(LogKind::Ok, msg.clone());
                            app.report_status = Some((true, msg));
                        }
                        Err(e) => {
                            let msg = trf(K::ReportExportErr, lang, &[("%e", e.clone())]);
                            report::event(
                                report::Level::Err,
                                "informe",
                                format!("Fallo exportando a {}: {e}", path.display()),
                            );
                            app.eng.log_add(LogKind::Err, msg.clone());
                            app.report_status = Some((false, msg));
                        }
                    }
                }
            }
            if ui
                .add(
                    egui::Button::new(RichText::new(tr(K::ReportCopyBtn, lang)).size(13.5))
                        .fill(SURFACE2)
                        .rounding(Rounding::same(10.0))
                        .min_size(Vec2::new(170.0, 34.0)),
                )
                .clicked()
            {
                let text = report_session_text(&entries);
                ui.output_mut(|o| o.copied_text = text);
                app.eng
                    .log_add(LogKind::Info, tr(K::ReportCopied, lang).to_string());
            }
            // NUEVO 1.3.0: borrar el registro automático que la app guarda en
            // disco (informe.log y su rotación informe.log.1), con confirmación.
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(tr(K::ReportClearBtn, lang)).size(13.5).color(RED),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(1.0_f32, RED.gamma_multiply(0.55)))
                    .rounding(Rounding::same(10.0))
                    .min_size(Vec2::new(170.0, 34.0)),
                )
                .clicked()
            {
                app.confirm = Some(ConfirmKind::ClearReportLog);
            }
        });
        if let Some((ok, msg)) = &app.report_status {
            ui.label(RichText::new(msg.clone()).size(12.5).color(if *ok {
                GREEN_PALE
            } else {
                RED
            }));
        }
    });

    ui.add_space(4.0);

    // ---- eventos de la sesión
    card(ui, Some(tr(K::ReportSessionTitle, lang)), |ui| {
        if entries.is_empty() {
            dim_label(ui, tr(K::ReportEmpty, lang));
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(360.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for e in &entries {
                        let (y, mo, d, h, mi, s) = report::epoch_to_datetime(e.ts);
                        let color = report_level_color(e.level);
                        ui.label(
                            RichText::new(format!(
                                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}  [{}] [{}] {}",
                                y,
                                mo,
                                d,
                                h,
                                mi,
                                s,
                                e.level.tag(),
                                e.cat,
                                e.msg
                            ))
                            .size(11.5)
                            .color(color)
                            .monospace(),
                        );
                    }
                });
        }
    });
}
