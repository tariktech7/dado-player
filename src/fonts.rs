//! Bundled, identical UI typography on iOS, Android and Windows.
//! Source Sans 3 static weights, SIL OFL; see assets/fonts/LICENSE.md.
use egui::{Context, FontData, FontDefinitions, FontFamily};

pub fn license(ui: &mut egui::Ui) {
    ui.collapsing("Schriftlizenz · Source Sans 3", |ui| {
        ui.label(include_str!("../assets/fonts/LICENSE.md"));
    });
}

pub fn ready(ctx: &Context) -> bool {
    !ctx.data_mut(|d| {
        d.remove_temp::<bool>(egui::Id::new("dado-fonts-pending"))
            .unwrap_or(false)
    })
}

pub fn install(ctx: &Context) {
    let installed = egui::Id::new("dado-fonts-installed");
    if ctx.data(|d| d.get_temp::<bool>(installed)).unwrap_or(false) {
        return;
    }
    let mut defs = FontDefinitions::default();
    for (name, bytes) in [
        (
            "ui-regular",
            include_bytes!("../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
        ),
        (
            "ui-medium",
            include_bytes!("../assets/fonts/SourceSans3-Semibold.ttf").as_slice(),
        ),
        (
            "ui-bold",
            include_bytes!("../assets/fonts/SourceSans3-Bold.ttf").as_slice(),
        ),
    ] {
        defs.font_data
            .insert(name.into(), FontData::from_static(bytes));
        let mut family = vec![name.to_owned()];
        family.extend(defs.families[&FontFamily::Proportional].iter().cloned());
        defs.families.insert(FontFamily::Name(name.into()), family);
    }
    defs.families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "ui-regular".into());
    ctx.set_fonts(defs);
    ctx.data_mut(|d| {
        d.insert_temp(installed, true);
        d.insert_temp(egui::Id::new("dado-fonts-pending"), true);
    });
}
