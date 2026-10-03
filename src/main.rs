use eframe::{Frame, egui};

struct SnippetApp;


impl eframe::App for SnippetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        ui.label("...");
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Snippet Manager",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp))),
    )
}
