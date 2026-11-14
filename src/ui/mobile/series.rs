use super::Layout;
use crate::app::App;
use egui::Ui;
pub fn show(app: &mut App, ui: &mut Ui, _l: &Layout) {
    crate::ui::library::show(app, ui, true);
}
