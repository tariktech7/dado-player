use super::{components, login, tokens};
use crate::{app::App, types::ProfileKind};
use egui::{pos2, vec2, Align, Button, Context, Frame, Id, Layout, Rect, RichText, Ui};

pub(super) fn show(app: &mut App, ctx: &Context) {
    crate::platform::set_text_focus(false);
    let blocked = app.pending_delete.is_some();
    let safe = crate::ui::desktop::safe_rect(ctx, true);
    let body = safe.shrink2(vec2(12.0, 6.0));
    let width = body.width().min(1100.0);
    let left = body.center().x - width * 0.5;
    let split = left + width * 0.64;
    let form_rect = Rect::from_min_max(pos2(left, body.top()), pos2(split - 8.0, body.bottom()));
    let device_rect = Rect::from_min_max(
        pos2(split + 8.0, body.top()),
        pos2(left + width, body.bottom()),
    );
    egui::CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, |ui| {
            components::page_bg(ui);
            ui.painter().vline(
                split,
                body.y_range(),
                egui::Stroke::new(1.0, tokens::hairline(ui)),
            );
            pane(ui, form_rect, "provider-login", |ui| {
                ui.set_enabled(!blocked && !app.busy);
                provider(app, ui, form_rect);
            });
            pane(ui, device_rect, "login-device", |ui| {
                ui.set_enabled(!blocked);
                device(app, ui);
            });
        });

    if blocked {
        login::confirm_delete(app, ctx);
    } else {
        ctx.data_mut(|data| data.remove::<bool>(login::confirm_shown_key()));
    }
}

fn pane(ui: &mut Ui, rect: Rect, name: &str, render: impl FnOnce(&mut Ui)) {
    let mut pane = ui.child_ui_with_id_source(rect, Layout::top_down(Align::Min), name);
    pane.set_clip_rect(rect);
    pane.spacing_mut().item_spacing = vec2(8.0, 5.0);
    pane.style_mut()
        .text_styles
        .insert(egui::TextStyle::Small, tokens::reg(15.0));
    egui::ScrollArea::vertical()
        .id_source(name)
        .auto_shrink([false, false])
        .animated(false)
        .show(&mut pane, render);
}

fn provider(app: &mut App, ui: &mut Ui, viewport: Rect) {
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    let brand = |ui: &mut Ui| {
        crate::ui::brand_mark(ui, 28.0);
        ui.label(RichText::new("Dado Player").font(tokens::bold(21.0)));
    };
    let kind =
        |ui: &mut Ui, selected| components::segmented(ui, &["Xtream Codes", "M3U"], selected);
    let selected = usize::from(app.login_kind == ProfileKind::M3u);
    let mode = if ui.available_width() >= 352.0 {
        ui.horizontal(|ui| {
            brand(ui);
            ui.add_space(6.0);
            kind(ui, selected)
        })
        .inner
    } else {
        ui.horizontal(brand);
        kind(ui, selected)
    };
    if let Some(mode) = mode {
        app.login_kind = if mode == 0 {
            ProfileKind::Xtream
        } else {
            ProfileKind::M3u
        };
    }

    use components::Kb;
    let mut focused = None;
    let mut unused_password = false;
    match app.login_kind {
        ProfileKind::Xtream => {
            ui.columns(2, |columns| {
                login::field(
                    &mut columns[0],
                    t("Serveradresse", "Server address"),
                    &mut app.f_host,
                    "https://server:8080",
                    Kb::Url,
                    &mut focused,
                    &mut unused_password,
                );
                login::field(
                    &mut columns[1],
                    t("Profilname (optional)", "Profile name (optional)"),
                    &mut app.f_name,
                    t("Wohnzimmer", "Living room"),
                    Kb::Name,
                    &mut focused,
                    &mut unused_password,
                );
            });
            ui.columns(2, |columns| {
                login::field(
                    &mut columns[0],
                    t("Benutzername", "Username"),
                    &mut app.f_user,
                    "",
                    Kb::Text,
                    &mut focused,
                    &mut unused_password,
                );
                login::field(
                    &mut columns[1],
                    t("Passwort", "Password"),
                    &mut app.f_pass,
                    "",
                    Kb::Password,
                    &mut focused,
                    &mut app.show_password,
                );
            });
        }
        ProfileKind::M3u => {
            login::field(
                ui,
                t("M3U-Link", "M3U URL"),
                &mut app.f_m3u_url,
                "https://…/liste.m3u",
                Kb::Url,
                &mut focused,
                &mut unused_password,
            );
            ui.columns(2, |columns| {
                login::field(
                    &mut columns[0],
                    t("Oder lokale Datei", "Or local file"),
                    &mut app.f_m3u_file,
                    "playlist.m3u",
                    Kb::Url,
                    &mut focused,
                    &mut unused_password,
                );
                login::field(
                    &mut columns[1],
                    t("Profilname (optional)", "Profile name (optional)"),
                    &mut app.f_name,
                    t("Wohnzimmer", "Living room"),
                    Kb::Name,
                    &mut focused,
                    &mut unused_password,
                );
            });
        }
    }
    if let Some(rect) = focused {
        let target = rect.expand2(vec2(0.0, 6.0));
        if viewport.height() >= target.height()
            && !viewport.contains_rect(target)
            && !ui.input(|i| i.pointer.any_down())
        {
            ui.scroll_to_rect(target, None);
        }
    }

    let label = if app.busy {
        t("Verbinde …", "Connecting …")
    } else {
        t("Anmelden", "Sign in")
    };
    let (button, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), 46.0), egui::Sense::hover());
    if components::button_at(ui, button, label, true, !app.busy).clicked()
        && ui.is_enabled()
        && !app.busy
    {
        if let Some(id) = ui.memory(|memory| memory.focused()) {
            ui.memory_mut(|memory| memory.surrender_focus(id));
        }
        app.submit_form();
    }
    if let Some(error) = &app.error {
        ui.colored_label(crate::theme::DANGER, error);
    }
    components::storage_banner(ui, crate::store::storage_failed());
}

fn device(app: &mut App, ui: &mut Ui) {
    let language = app.settings.language;
    let t = |de, en| language.text(de, en);
    ui.label(RichText::new(t("Gerät und Profile", "Device and profiles")).font(tokens::bold(19.0)));
    app.cloud_profiles_entry(ui);

    if !app.profiles.is_empty() {
        ui.separator();
        ui.label(t("Gespeicherte Profile", "Saved profiles"));
        let id = Id::new("handheld-saved-profile");
        let mut selected = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_default();
        if !app.profiles.iter().any(|p| p.id == selected) {
            selected.clone_from(&app.profiles[0].id);
        }
        if let Some(profile) = app.profiles.iter().find(|p| p.id == selected) {
            egui::ComboBox::from_id_source(id)
                .width(ui.available_width())
                .wrap(true)
                .selected_text(&profile.name)
                .show_ui(ui, |ui| {
                    for profile in &app.profiles {
                        ui.selectable_value(&mut selected, profile.id.clone(), &profile.name);
                    }
                });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !app.busy,
                    Button::new(t("Öffnen", "Open")).min_size(vec2(0.0, 44.0)),
                )
                .clicked()
            {
                if let Some(profile) = app.profiles.iter().find(|p| p.id == selected) {
                    app.start_login(profile.clone());
                }
            }
            if ui
                .add_enabled(
                    !app.busy,
                    Button::new(t("Löschen", "Delete")).min_size(vec2(0.0, 44.0)),
                )
                .clicked()
            {
                app.pending_delete = Some(selected.clone());
            }
        });
        ui.data_mut(|d| d.insert_temp(id, selected));
    }

    ui.add_enabled_ui(!app.busy, |ui| {
        ui.menu_button(t("Ausprobieren", "Try the app"), |ui| {
            if ui
                .button(t("Testvideo abspielen", "Play test video"))
                .clicked()
            {
                app.play_test_stream();
                ui.close_menu();
            }
            if ui.button(t("Demo ansehen", "Browse demo")).clicked() {
                app.load_demo();
                ui.close_menu();
            }
        });
    });
}
