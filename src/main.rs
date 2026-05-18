mod player;

use std::fs;
use eframe::egui;
// Wir importieren gezielt die Strukturen, die wir hier zum Starten brauchen
use player::{Chapter, VnPlayer};

fn main() -> eframe::Result<()> {
    // 1. Datei laden
    let file_content = fs::read_to_string("test_chapter.json")
        .expect("Konnte test_chapter.json nicht finden!");

    // 2. JSON parsen (nutzt das Chapter aus dem player-Modul)
    let chapter: Chapter = serde_json::from_str(&file_content)
        .expect("Fehler beim Parsen der JSON-Datei");

    let window_title = chapter.title.clone();

    // 3. Eframe starten
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        &window_title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            // Wir übergeben das Chapter an unseren VnPlayer
            Ok(Box::new(VnPlayer::new(chapter)))
        }),
    )
}