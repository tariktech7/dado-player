use crate::app::App;
use egui::Ui;

pub(crate) fn rating_badge_for_home(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let num: String = t
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    let value: f32 = num.replace(',', ".").parse().ok()?;
    if !(0.1..=10.0).contains(&value) {
        return None;
    }
    Some(format!("★ {}", t.split_whitespace().next().unwrap_or(t)))
}

pub fn show(app: &mut App, ui: &mut Ui) {
    crate::ui::library::show(app, ui, false);
}
