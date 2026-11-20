use crate::{app::App, parental::scoped_key, types::PlayItem};
use egui::{Context, Ui};

fn button(ui: &mut Ui, label: &str) -> bool {
    let r = ui.add_sized(
        [ui.available_width().min(360.0), 38.0],
        egui::Button::new(label),
    );
    clicked(ui, &r)
}
fn clicked(ui: &Ui, r: &egui::Response) -> bool {
    if crate::ui::mobile::active(ui.ctx()) {
        crate::ui::mobile::focus::clicked(ui, r)
    } else {
        r.clicked()
    }
}
impl App {
    pub fn parental_gate(&mut self, item: &PlayItem, preview: bool) -> bool {
        let profile = self
            .active_profile
            .as_ref()
            .map(|p| p.id.as_str())
            .unwrap_or("");
        let key = scoped_key(profile, &item.progress_key);
        if self.parental.grant.take().as_ref() == Some(&key) {
            return true;
        }
        if !self.parental.policy.locked(profile, &item.progress_key) {
            return true;
        }
        if self.current_play.is_some() || self.view == crate::app::View::Player {
            self.stop_play();
        }
        self.close_preview();
        self.parental.close();
        self.parental.open = true;
        self.parental.pending = Some((item.clone(), preview));
        false
    }
}
pub fn entry(app: &mut App, ui: &mut Ui) {
    ui.heading("Elternsperre");
    if crate::ui::desktop::active(ui.ctx()) {
        ui.add_space(12.0 * crate::ui::desktop::scale(ui.ctx()));
    }
    ui.label(if app.parental.policy.enabled() {
        "Aktiv · gesperrte Inhalte benötigen die Eltern-PIN"
    } else {
        "Vierstellige PIN für Sender, Filme und Serien einrichten"
    });
    if button(
        ui,
        if app.parental.policy.enabled() {
            "Elternsperre verwalten"
        } else {
            "Eltern-PIN einrichten"
        },
    ) {
        app.close_preview();
        app.parental.close();
        app.parental.open = true;
    }
    ui.add_space(12.0);
}
pub fn show(app: &mut App, ctx: &Context) {
    let layout = crate::ui::mobile::Layout::new(ctx);
    egui::CentralPanel::default().frame(egui::Frame::none().inner_margin(egui::Margin {left:layout.side+12.0,right:layout.side+12.0,top:layout.top_inset+12.0,bottom:layout.bottom_inset+12.0})).show(ctx,|ui| {
        egui::ScrollArea::vertical().show(ui,|ui| {
            ui.set_max_width(720.0);
            ui.heading("Elternsperre");
            if button(ui,"Zurück / Abbrechen") {app.parental.close();return;}
            if !app.parental.error.is_empty() {ui.colored_label(egui::Color32::from_rgb(220,70,90),&app.parental.error);}
            let setup=!app.parental.policy.enabled() || app.parental.changing;
            if setup || !app.parental.authenticated || app.parental.pending.is_some() {
                ui.label(if setup {if app.parental.first_pin.is_empty(){"Neue PIN: genau vier Ziffern"}else{"Neue PIN bestätigen"}}else{"Geschützt · Eltern-PIN eingeben"});
                if let Some((item,_))=&app.parental.pending {ui.label(&item.title);}
                ui.label(format!("{}{}","●".repeat(app.parental.pin.len()),"○".repeat(4-app.parental.pin.len())));
                // Numeric keypad works with touch, mouse and the TV focus engine.
                egui::Grid::new("parent-pin-pad").show(ui,|ui| {
                    for (i,n) in ["1","2","3","4","5","6","7","8","9","Löschen","0"].iter().enumerate() {
                        let r=ui.add_sized([80.0,42.0],egui::Button::new(*n));
                        if clicked(ui,&r) {if *n=="Löschen"{app.parental.pin.pop();}else if app.parental.pin.len()<4{app.parental.pin.push_str(n);}}
                        if i%3==2 {ui.end_row();}
                    }
                });
                if button(ui,if setup{"PIN speichern / bestätigen"}else{"Entsperren"}) {
                    if setup {
                        if !crate::parental::valid_pin(&app.parental.pin){app.parental.error="Bitte genau vier Ziffern eingeben.".into();}
                        else if app.parental.first_pin.is_empty(){app.parental.first_pin=std::mem::take(&mut app.parental.pin);app.parental.error.clear();}
                        else if app.parental.first_pin!=app.parental.pin {app.parental.pin.clear();app.parental.first_pin.clear();app.parental.error="PINs stimmen nicht überein. Bitte erneut einrichten.".into();}
                        else {
                            let mut policy=app.parental.policy.clone();
                            let result=policy.set_pin(&app.parental.pin).and_then(|_|crate::store::save_parental(&policy));
                            match result { Ok(())=>{app.parental.policy=policy;app.parental.pin.clear();app.parental.first_pin.clear();app.parental.changing=false;app.parental.authenticated=true;app.parental.error.clear();},Err(_)=>app.parental.error="PIN konnte nicht gespeichert werden.".into() }
                        }
                    } else if app.parental.check() {
                        if let Some((item,preview))=app.parental.pending.take() {
                            let key=scoped_key(app.active_profile.as_ref().map(|p|p.id.as_str()).unwrap_or(""),&item.progress_key);
                            app.parental.close();app.parental.grant=Some(key);
                            if preview {app.start_preview(item);}else{app.play(item);}
                        } else {app.parental.authenticated=true;}
                    }
                }
                return;
            }
            ui.label("Gesperrte Serien schützen alle Staffeln und Folgen. Auswahl gilt für das jeweilige Anbieterprofil auf diesem Gerät.");
            if button(ui,"PIN ändern") {app.parental.changing=true;return;}
            if button(ui,"Elternsperre ausschalten") {
                let policy=Default::default();
                if crate::store::save_parental(&policy).is_ok(){app.parental.policy=policy;app.parental.close();}else{app.parental.error="Änderung konnte nicht gespeichert werden.".into();}return;
            }
            ui.horizontal_wrapped(|ui| {for (i,label) in ["Sender","Filme","Serien"].iter().enumerate(){let r=ui.selectable_label(app.parental.kind==i,*label);if clicked(ui,&r){app.parental.kind=i;}}});
            ui.add(egui::TextEdit::singleline(&mut app.parental.query).hint_text("Inhalte suchen …"));
            let q=app.parental.query.to_lowercase();
            let signature=format!("{}:{}:{}:{}:{}:{:p}",app.parental.kind,q,app.library.channels.len(),app.library.movies.len(),app.library.series.len(),app.library.channels.as_ptr());
            if app.parental.catalog_cache.as_ref().map(|c|c.0.as_str())!=Some(signature.as_str()) {
                let rows:Vec<(String,String)>=match app.parental.kind {
                0=>app.library.channels.iter().filter(|c|c.name.to_lowercase().contains(&q)).map(|c|(format!("live:{}",c.id),c.name.clone())).collect(),
                1=>app.library.movies.iter().filter(|c|c.name.to_lowercase().contains(&q)).map(|c|(format!("vod:{}",c.id),c.name.clone())).collect(),
                _=>app.library.series.iter().filter(|c|c.name.to_lowercase().contains(&q)).map(|c|(format!("series:{}",c.id),c.name.clone())).collect(),
            };
                app.parental.page=0;app.parental.catalog_cache=Some((signature,rows));
            }
            let cached=&app.parental.catalog_cache.as_ref().unwrap().1;
            let count=cached.len();
            let start=app.parental.page*40;
            let rows=cached.iter().skip(start).take(40).cloned().collect::<Vec<_>>();
            if rows.is_empty(){ui.label("Keine passenden Inhalte. Zuerst ein Anbieterprofil anmelden.");}
            let profile=app.active_profile.as_ref().map(|p|p.id.clone()).unwrap_or_default();
            egui::ScrollArea::vertical().id_source(("parental-list",app.parental.page,app.parental.kind)).max_height(420.0).show(ui,|ui| {
                for (key,name) in &rows {let scoped=scoped_key(&profile,key);let locked=app.parental.policy.blocked.contains(&scoped);
                    if button(ui,&format!("{} {}",if locked{"Gesperrt ·"}else{"Freigegeben ·"},name)) {
                        let mut next=app.parental.policy.clone();if locked{next.blocked.remove(&scoped);}else{next.blocked.insert(scoped);}
                        if crate::store::save_parental(&next).is_ok(){app.parental.policy=next;}else{app.parental.error="Sperre konnte nicht gespeichert werden.".into();}
                    }
                }
            });
            if app.parental.page>0 && button(ui,"Vorherige Inhalte") {app.parental.page-=1;}
            if start+rows.len()<count && button(ui,"Weitere Inhalte") {app.parental.page+=1;}
        });
    });
    if app.parental.open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.parental.close();
    }
}
