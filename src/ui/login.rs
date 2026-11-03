use crate::app::App;
use crate::theme;
use crate::types::ProfileKind;
use crate::ui;
use egui::{Align, Color32, Context, Layout, RichText, Rounding, Stroke, Ui};

pub fn show(app: &mut App, ctx: &Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let avail = ui.available_size();
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
                    ui.set_width(440.0);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        form(app, ui);
                    });
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
        ui.add_space(4.0);
        for p in app.profiles.clone() {
            let kind = match p.kind {
                ProfileKind::Xtream => "Xtream",
                ProfileKind::M3u => "M3U",
            };
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(&p.name).size(14.0).strong()).truncate(true));
                ui.add(egui::Label::new(
                    RichText::new(format!("({})", kind)).size(11.0).weak(),
                ));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let pid = p.id.clone();
                    if ui.button("Löschen").on_hover_text("Profil löschen").clicked() {
                        app.delete_profile(&pid);
                    }
                    if ui.button("Anmelden").clicked() {
                        app.start_login(p.clone());
                    }
                });
            });
        }
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(10.0);
    }

    ui.add(egui::Label::new(
        RichText::new("Neues Profil").size(12.0).weak(),
    ));
    ui.add_space(4.0);

    field(ui, "Name (optional)", &mut app.f_name, "z.B. Wohnzimmer", false);

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        for (kind, label) in [
            (ProfileKind::Xtream, "Xtream Codes"),
            (ProfileKind::M3u, "M3U / Playlist"),
        ] {
            let sel = app.login_kind == kind;
            let mut b = egui::Button::new(
                RichText::new(label)
                    .size(13.0)
                    .color(if sel { Color32::WHITE } else { ui.visuals().text_color() }),
            )
            .rounding(Rounding::same(9.0));
            if sel {
                b = b.fill(theme::ACCENT);
            }
            if ui.add(b).clicked() {
                app.login_kind = kind;
            }
        }
    });
    ui.add_space(6.0);

    match app.login_kind {
        ProfileKind::Xtream => {
            field(ui, "Host", &mut app.f_host, "http://server:8080", false);
            field(ui, "Benutzer", &mut app.f_user, "Benutzername", false);
            field(ui, "Passwort", &mut app.f_pass, "Passwort", true);
        }
        ProfileKind::M3u => {
            field(ui, "M3U-URL", &mut app.f_m3u_url, "http://…/liste.m3u", false);
            field(
                ui,
                "oder lokale Datei",
                &mut app.f_m3u_file,
                "C:\\…\\liste.m3u",
                false,
            );
        }
    }

    ui.add_space(8.0);
    let btn = egui::Button::new(
        RichText::new(if app.busy { "Verbinde…" } else { "Anmelden" })
            .size(14.5)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(theme::ACCENT)
    .rounding(Rounding::same(10.0));
    if ui
        .add_sized([ui.available_width(), 38.0], btn)
        .clicked()
        && !app.busy
    {
        app.submit_form();
    }

    if app.busy {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.add(egui::Label::new(
                RichText::new("Bibliothek wird geladen…").size(12.0).weak(),
            ));
        });
    }
    if let Some(e) = &app.error {
        ui.add_space(6.0);
        ui.add(egui::Label::new(
            RichText::new(format!("Fehler: {}", e))
                .size(12.0)
                .color(theme::DANGER),
        ));
    }

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add(egui::Label::new(
            RichText::new("Ohne Anbieter testen:").size(11.5).weak(),
        ));
        if ui.button("Teststream").clicked() {
            app.play_test_stream();
        }
    });
}

fn field(ui: &mut Ui, label: &str, text: &mut String, hint: &str, password: bool) {
    ui.add(egui::Label::new(RichText::new(label).size(11.0).weak()));
    let mut te = egui::TextEdit::singleline(text)
        .hint_text(hint)
        .desired_width(f32::INFINITY);
    if password {
        te = te.password(true);
    }
    ui.add(te);
    ui.add_space(3.0);
}
