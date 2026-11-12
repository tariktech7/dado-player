use crate::app::App;
use crate::theme;
use crate::types::ProfileKind;
use crate::ui;
use egui::{
    Align, Color32, Context, Layout, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};

pub fn show(app: &mut App, ctx: &Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // The card is taller than a phone in landscape (~411dp). Without a
        // scroll area the "Anmelden" button and the Teststream link fall off
        // screen and are unreachable by touch. See MOBILE-BAUPLAN.md §17.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail = ui.available_size();
                // Keep the card readable on very narrow screens.
                let card_w = (avail.x - 32.0).clamp(280.0, 440.0);
                ui.allocate_ui_with_layout(avail, Layout::top_down(Align::Center), |ui| {
                    ui.add_space((avail.y * 0.08).clamp(8.0, 64.0));

                    egui::Frame::none()
                        .fill(ui.visuals().window_fill)
                        .rounding(Rounding::same(18.0))
                        .stroke(Stroke::new(
                            1.0_f32,
                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                        ))
                        .inner_margin(egui::Margin::same(26.0))
                        .show(ui, |ui| {
                            ui.set_width(card_w);
                            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                                form(app, ui);
                            });
                        });
                    ui.add_space(24.0);
                });
            });
    });
}

fn form(app: &mut App, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui::brand_mark(ui, 40.0);
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.add(egui::Label::new(
                RichText::new("Dado-Player").size(24.0).strong(),
            ));
            ui.add(egui::Label::new(
                RichText::new("Dein Anbieter. Keine Werbung. Keine Telemetrie.")
                    .size(11.5)
                    .weak(),
            ));
        });
    });

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(10.0);

    if !app.profiles.is_empty() {
        ui.add(egui::Label::new(
            RichText::new("Gespeicherte Profile").size(12.0).weak(),
        ));
        ui.add_space(6.0);
        // Snapshot only the three fields the list actually renders. Cloning the
        // whole Vec<Profile> on every frame duplicated every saved password and
        // M3U URL (which usually embeds credentials) for no reason.
        let rows: Vec<(usize, String, ProfileKind)> = app
            .profiles
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.name.clone(), p.kind))
            .collect();
        for (idx, name, kind) in rows {
            let (kind_name, col) = match kind {
                ProfileKind::Xtream => ("Xtream", theme::PROVIDER_XTREAM),
                ProfileKind::M3u => ("M3U", theme::PROVIDER_M3U),
            };
            let row_h = 44.0;
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), row_h),
                egui::Sense::click(),
            );
            let dark = ui.visuals().dark_mode;
            let bg = if resp.hovered() {
                theme::card_hover(ui)
            } else if dark {
                Color32::from_white_alpha(10)
            } else {
                Color32::from_black_alpha(8)
            };
            ui.painter().rect_filled(rect, Rounding::same(8.0), bg);
            if resp.hovered() {
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(8.0),
                    Stroke::new(1.0_f32, theme::accent(ui).gamma_multiply(0.5)),
                );
            }

            // Left: dot + name + kind
            let mut left_ui = ui.child_ui(
                egui::Rect::from_min_max(
                    rect.min + egui::vec2(10.0, 4.0),
                    egui::pos2(rect.right() - 130.0, rect.bottom() - 4.0),
                ),
                Layout::left_to_right(Align::Center),
            );
            left_ui.painter().circle_filled(
                egui::pos2(rect.left() + 14.0, rect.center().y),
                3.5,
                col,
            );
            left_ui.add_space(14.0);
            left_ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(&name).size(13.5).strong()).truncate(true));
                ui.add(egui::Label::new(
                    RichText::new(kind_name).size(10.5).color(col),
                ));
            });

            // Right: Anmelden + Löschen
            let mut right_ui = ui.child_ui(
                egui::Rect::from_min_max(
                    egui::pos2(rect.right() - 130.0, rect.top() + 6.0),
                    rect.max - egui::vec2(8.0, 6.0),
                ),
                Layout::right_to_left(Align::Center),
            );
            if right_ui
                .button("🗑")
                .on_hover_text("Profil löschen")
                .clicked()
            {
                if let Some(pid) = app.profiles.get(idx).map(|p| p.id.clone()) {
                    app.delete_profile(&pid);
                }
            }
            right_ui.add_space(4.0);
            let login_btn =
                egui::Button::new(RichText::new("Anmelden").size(12.0).color(Color32::WHITE))
                    .fill(theme::accent_fill(ui))
                    .rounding(Rounding::same(6.0));
            if right_ui.add(login_btn).clicked() || resp.clicked() {
                if let Some(p) = app.profiles.get(idx).cloned() {
                    app.start_login(p);
                }
            }
            ui.add_space(4.0);
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(10.0);
    }

    ui.add(egui::Label::new(
        RichText::new("Neues Profil").size(12.0).weak(),
    ));
    ui.add_space(6.0);

    field(
        ui,
        "Name (optional)",
        &mut app.f_name,
        "z.B. Wohnzimmer",
        false,
        None,
    );

    ui.add_space(4.0);
    let tab_w = (ui.available_width() - 8.0) / 2.0;
    ui.horizontal(|ui| {
        for (kind, label) in [
            (ProfileKind::Xtream, "Xtream Codes"),
            (ProfileKind::M3u, "M3U / Playlist"),
        ] {
            let sel = app.login_kind == kind;
            let mut b = egui::Button::new(RichText::new(label).size(13.0).color(if sel {
                Color32::WHITE
            } else {
                ui.visuals().text_color()
            }))
            .rounding(Rounding::same(8.0));
            if sel {
                b = b.fill(theme::accent_fill(ui));
            }
            if ui.add_sized([tab_w, 32.0], b).clicked() {
                app.login_kind = kind;
            }
        }
    });
    ui.add_space(8.0);

    match app.login_kind {
        ProfileKind::Xtream => {
            field(
                ui,
                "Host",
                &mut app.f_host,
                "http://server:8080",
                false,
                None,
            );
            field(ui, "Benutzer", &mut app.f_user, "Benutzername", false, None);
            field(
                ui,
                "Passwort",
                &mut app.f_pass,
                "Passwort",
                true,
                Some(&mut app.show_password),
            );
        }
        ProfileKind::M3u => {
            field(
                ui,
                "M3U-URL",
                &mut app.f_m3u_url,
                "http://…/liste.m3u",
                false,
                None,
            );
            field(
                ui,
                "oder lokale Datei",
                &mut app.f_m3u_file,
                "C:\\…\\liste.m3u",
                false,
                None,
            );
        }
    }

    ui.add_space(10.0);
    let btn = egui::Button::new(
        RichText::new(if app.busy { "Verbinde…" } else { "Anmelden" })
            .size(14.5)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(theme::accent_fill(ui))
    .rounding(Rounding::same(10.0));
    if ui.add_sized([ui.available_width(), 38.0], btn).clicked() && !app.busy {
        app.submit_form();
    }

    if app.busy {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add(egui::Label::new(
                RichText::new("Bibliothek wird geladen…").size(12.0).weak(),
            ));
        });
    }
    if let Some(e) = &app.error {
        ui.add_space(8.0);
        egui::Frame::none()
            .fill(theme::DANGER.gamma_multiply(0.12))
            .stroke(Stroke::new(1.0_f32, theme::DANGER))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(14.0).color(theme::DANGER));
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("Fehler: {}", e))
                                .size(12.0)
                                .color(theme::DANGER),
                        )
                        .wrap(true),
                    );
                });
            });
    }

    ui.add_space(14.0);
    ui.separator();
    ui.add_space(8.0);
    let test_resp = ui.add_sized(
        [ui.available_width(), 32.0],
        egui::Button::new(RichText::new("🎬  Ohne Anbieter testen: Teststream starten").size(12.5))
            .rounding(Rounding::same(8.0)),
    );
    if test_resp.clicked() {
        app.play_test_stream();
    }
}

fn field(
    ui: &mut Ui,
    label: &str,
    text: &mut String,
    hint: &str,
    password: bool,
    show: Option<&mut bool>,
) {
    ui.add(egui::Label::new(RichText::new(label).size(11.0).weak()));
    let mut show_val = show.as_deref().copied().unwrap_or(false);
    let mut te = egui::TextEdit::singleline(text)
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    if password {
        te = te.password(!show_val);
    }
    if password {
        // Reserve space for the trailing eye toggle.
        let btn_w = 34.0;
        ui.horizontal(|ui| {
            let te_w = (ui.available_width() - btn_w).max(40.0);
            ui.add_sized([te_w, 20.0], te);
            ui.add_space(4.0);
            let (_, r) = ui.allocate_space(Vec2::new(btn_w, 20.0));
            let resp = ui.interact(r, ui.id().with(("eye_toggle", label)), Sense::click());
            let col = if show_val {
                theme::accent_fill(ui)
            } else {
                ui.visuals().text_color()
            };
            draw_eye(ui, r, show_val, col);
            if resp.clicked() {
                show_val = !show_val;
            }
        });
    } else {
        ui.add(te);
    }
    if let Some(b) = show {
        *b = show_val;
    }
    ui.add_space(3.0);
}

/// Vector eye glyph. Open = eye outline + pupil; closed = same with a slash
/// across, signalling the password is masked.
fn draw_eye(ui: &Ui, rect: Rect, open: bool, color: Color32) {
    let p = ui.painter();
    let c = rect.center();
    let radius = Vec2::new(rect.width() * 0.36, rect.height() * 0.44);
    let s = Stroke::new(rect.width().min(rect.height()) * 0.11, color);
    p.add(Shape::ellipse_stroke(c, radius, s));
    if open {
        p.circle_filled(c, radius.x * 0.34, color);
    } else {
        p.line_segment(
            [
                c - Vec2::new(radius.x, radius.y),
                c + Vec2::new(radius.x, radius.y),
            ],
            s,
        );
    }
}
