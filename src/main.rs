use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

// --- DATENMODEL ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
    pub title: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    SetBackground { image_path: String },
    ShowCharacter { character_id: String, image_path: String, position: CharacterPosition },
    Dialogue { speaker_name: Option<String>, text: String, audio_path: Option<String> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CharacterPosition { Left, Center, Right }

struct ActiveCharacter {
    id: String,
    image_path: String,
    position: CharacterPosition, // HIER NEU
}

// --- DER PLAYER (ENGINE) ---
struct VnPlayer {
    chapter: Chapter,
    current_index: usize,

    current_background: Option<String>,
    active_characters: Vec<ActiveCharacter>,

    current_speaker: Option<String>,
    current_text: String,
    is_finished: bool,

    // AUDIO: Wir nutzen jetzt die neuen Typen von Rodio!
    _audio_stream: MixerDeviceSink,
    audio_player: Player,
}

impl VnPlayer {
    fn new(chapter: Chapter) -> Self {
        // HIER IST DAS NEUE AUDIO-SETUP FÜR RODIO 0.22
        let stream_handle = DeviceSinkBuilder::open_default_sink()
            .expect("Konnte kein Audio-Gerät finden!");
        let player = Player::connect_new(stream_handle.mixer());

        let mut vn_player = Self {
            chapter,
            current_index: 0,
            current_background: None,
            active_characters: vec![],
            current_speaker: None,
            current_text: String::new(),
            is_finished: false,
            _audio_stream: stream_handle,
            audio_player: player,
        };
        vn_player.process_actions_until_dialogue();
        vn_player
    }

    fn process_actions_until_dialogue(&mut self) {
        while self.current_index < self.chapter.actions.len() {
            let action = &self.chapter.actions[self.current_index];

            match action {
                Action::SetBackground { image_path } => {
                    self.current_background = Some(image_path.clone());
                    self.current_index += 1;
                }
                Action::ShowCharacter { character_id, image_path, position } => {
                    // Alten Charakter entfernen, falls er schon da ist
                    self.active_characters.retain(|c| c.id != *character_id);

                    // Neuen Charakter mit Position speichern
                    self.active_characters.push(ActiveCharacter {
                        id: character_id.clone(),
                        image_path: image_path.clone(),
                        position: position.clone(), // HIER NEU
                    });
                    self.current_index += 1;
                }
                Action::Dialogue { speaker_name, text, audio_path } => {
                    self.current_speaker = speaker_name.clone();
                    self.current_text = text.clone();

                    // AUDIO ABSPIELEN (Neuer Ansatz für Rodio 0.22)
                    if let Some(path) = audio_path {
                        if let Ok(file) = fs::File::open(path) {
                            // Erstelle einen neuen Player, um garantieren zu können, dass alter Ton stoppt
                            self.audio_player = Player::connect_new(self._audio_stream.mixer());

                            // Decoder::try_from ist die neue, simplere Methode, um Audio zu laden
                            if let Ok(source) = Decoder::try_from(file) {
                                self.audio_player.append(source);
                            } else {
                                eprintln!("Audio-Datei konnte nicht gelesen werden: {}", path);
                            }
                        } else {
                            eprintln!("Konnte Audio-Datei nicht finden: {}", path);
                        }
                    }

                    break;
                }
            }
        }

        if self.current_index >= self.chapter.actions.len() {
            self.is_finished = true;
        }
    }

    fn next_action(&mut self) {
        self.current_index += 1;
        self.process_actions_until_dialogue();
    }
}

// --- EGUI BENUTZEROBERFLÄCHE (Die ultimative 0.34+ Methode) ---
impl eframe::App for VnPlayer {

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        if self.is_finished {
            ui.centered_and_justified(|ui| {
                ui.heading("Kapitel beendet.");
            });
            return;
        }

        let full_rect = ui.max_rect();

        // --- 1. HINTERGRUND ---
        if let Some(bg) = &self.current_background {
            let uri = format!("file://{}", bg);
            let image = egui::Image::new(&uri).fit_to_exact_size(full_rect.size());
            ui.put(full_rect, image);
        }

        // --- BEREICHE BERECHNEN ---
        let textbox_height = 160.0;
        let char_height = 450.0; // Die Ziel-Höhe der Figuren

        // Textbox Bereich
        let mut textbox_rect = full_rect;
        textbox_rect.set_top(full_rect.bottom() - textbox_height);

        // Charakter Bereich (Ein exakt 450px hoher Streifen ÜBER der Textbox)
        let mut char_rect = full_rect;
        char_rect.set_bottom(textbox_rect.top());
        char_rect.set_top(char_rect.bottom() - char_height);

        // --- 2. CHARAKTERE ZEICHNEN ---
        if !self.active_characters.is_empty() {

            for char in &self.active_characters {
                // Jeder Charakter bekommt sein eigenes, unsichtbares "Blatt Papier",
                // das den exakt gleichen Bereich (char_rect) abdeckt.
                let mut char_ui = ui.new_child(egui::UiBuilder::new().max_rect(char_rect));

                let uri = format!("file://{}", char.image_path);
                let image = egui::Image::new(&uri).max_height(char_height);

                // Wir platzieren das Bild basierend auf der gespeicherten Position!
                match char.position {
                    CharacterPosition::Left => {
                        char_ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                            ui.add(image);
                        });
                    }
                    CharacterPosition::Right => {
                        char_ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                            ui.add(image);
                        });
                    }
                    CharacterPosition::Center => {
                        char_ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add(image);
                        });
                    }
                }
            }

            ui.ctx().request_repaint();
        }

        // --- 3. TEXTBOX ZEICHNEN ---
        let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(textbox_rect));

        egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(220))
            .inner_margin(20.0)
            .show(&mut text_ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.vertical(|ui| {
                    if let Some(speaker) = &self.current_speaker {
                        ui.heading(egui::RichText::new(speaker).color(egui::Color32::from_rgb(100, 200, 255)));
                    } else {
                        ui.heading(egui::RichText::new("Erzähler").italics().color(egui::Color32::GRAY));
                    }

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&self.current_text).size(20.0));
                    ui.add_space(15.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                        if ui.button("Weiter ➡").clicked() || ui.ctx().input(|i| i.key_pressed(egui::Key::Space)) {
                            self.next_action();
                        }
                        ui.label(egui::RichText::new("(Leertaste)").small());
                    });
                });
            });
    }
}

fn main() -> eframe::Result<()> {
    let file_content = fs::read_to_string("test_chapter.json")
        .expect("Konnte test_chapter.json nicht finden!");

    let chapter: Chapter = serde_json::from_str(&file_content)
        .expect("Fehler beim Parsen der JSON-Datei");

    let window_title = chapter.title.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        &window_title,
        options,
        Box::new(|cc| {
            // Bilder-Loader von egui installieren
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(VnPlayer::new(chapter)))
        }),
    )
}