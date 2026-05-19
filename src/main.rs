mod player;
mod editor;

use eframe::egui;
use player::{VnPlayer, Chapter};
use editor::VnEditor;

// --- ZUSTANDS-MASCHINE ---
enum AppState {
    MainMenu,
    Playing(VnPlayer),
    Editing(VnEditor),
}

struct VisualNovelApp {
    state: AppState,
    show_exit_warning: bool, // Steuert, ob das Warn-Popup offen ist
    skip_exit_warning: bool, // Merkt sich, ob der Haken gesetzt wurde
}

impl VisualNovelApp {
    fn new() -> Self {
        Self {
            state: AppState::MainMenu,
            show_exit_warning: false,
            skip_exit_warning: false,
        }
    }
}

// HIER ist unsere echte Haupt-App!
impl eframe::App for VisualNovelApp {

    // FIX 1: eframe 0.34 verlangt 'fn ui' anstelle von 'fn update'
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        let mut go_back = false;

        // --- GLOBALE NAVIGATION ---
        // Ein "Zurück"-Knopf, der IMMER ganz oben angezeigt wird, außer wir sind im Hauptmenü
        if !matches!(self.state, AppState::MainMenu) {

            // FIX 2: Panels verlangen jetzt .show_inside(ui, ...) statt .show(ctx, ...)
            egui::Panel::top("global_nav").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("⬅ Zurück zum Hauptmenü").clicked() {

                        // Wenn wir im Editor sind UND die Warnung nicht blockiert ist: Popup zeigen!
                        if matches!(self.state, AppState::Editing(_)) && !self.skip_exit_warning {
                            self.show_exit_warning = true;
                        } else {
                            // Wenn wir im Spiel sind oder die Warnung übersprungen wird: Direkt zurück!
                            go_back = true;
                        }
                    }
                });
            });
        }

        // --- DAS WARN-POPUP ---
        if self.show_exit_warning {
            // Ein freischwebendes Fenster, das in der Mitte verankert wird
            // Fenster schweben "über" allem, daher heften wir sie an den Context: ui.ctx()
            egui::Window::new("⚠ Ungespeicherte Änderungen")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Bist du sicher, dass du zum Hauptmenü zurückkehren möchtest?");
                    ui.label(egui::RichText::new("Nicht gespeicherte Änderungen gehen dabei verloren!").color(egui::Color32::RED));

                    ui.add_space(15.0);

                    // Die Checkbox, um das Fenster für diese Sitzung abzustellen
                    ui.checkbox(&mut self.skip_exit_warning, "In dieser Sitzung nicht mehr fragen");

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        if ui.button("❌ Abbrechen").clicked() {
                            self.show_exit_warning = false; // Schließt das Popup
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✅ Ja, verlassen").clicked() {
                                self.show_exit_warning = false;
                                go_back = true; // Löst den Wechsel zum Hauptmenü aus
                            }
                        });
                    });
                });
        }

        // --- DER ZUSTANDS-WECHSEL ---
        if go_back {
            self.state = AppState::MainMenu;
            self.show_exit_warning = false; // Zur Sicherheit das Popup schließen
        }

        // --- HAUPTBEREICH ANZEIGEN ---
        // FIX 3: Auch hier 'show_inside(ui, ...)'!
        egui::CentralPanel::default().show_inside(ui, |ui| {

            match &mut self.state {

                // 1. DAS HAUPTMENÜ
                AppState::MainMenu => {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {

                            ui.heading(egui::RichText::new("🚀 Visual Novel Studio").size(40.0).strong());
                            ui.add_space(40.0);

                            if ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("▶ Spiel laden & starten").size(20.0))).clicked() {
                                if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                                    if let Ok(json_string) = std::fs::read_to_string(&path) {
                                        if let Ok(chapter) = serde_json::from_str::<Chapter>(&json_string) {
                                            self.state = AppState::Playing(VnPlayer::new(chapter));
                                        }
                                    }
                                }
                            }

                            ui.add_space(20.0);

                            if ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("✏ Neuen Editor öffnen").size(20.0))).clicked() {
                                self.state = AppState::Editing(VnEditor::new());
                            }

                        });
                    });
                }

                // 2. DAS SPIEL (VnPlayer)
                AppState::Playing(player) => {
                    player.ui(ui);
                }

                // 3. DER EDITOR (VnEditor)
                AppState::Editing(editor) => {
                    editor.ui(ui);
                }
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Visual Novel Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "Visual Novel Studio",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(VisualNovelApp::new()))
        }),
    )
}