use super::Layout;
use crate::app::App;
use egui::Ui;
pub fn show(app: &mut App, ui: &mut Ui, l: &Layout) {
    crate::ui::library::show_touch(app, ui, true, l);
}
