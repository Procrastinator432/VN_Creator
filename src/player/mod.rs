use eframe::egui;
use serde::{Deserialize, Serialize};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

// --- DATENMODEL ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
    pub title: String,
    pub theme: Option<Theme>, // NEU: Das optionale Design-Thema!
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Theme {
    pub textbox_color: Option<[u8; 4]>,
    pub frame_color: Option<[u8; 4]>,
    pub show_character_frames: Option<bool>, // NEU: Nur für die Charaktere
    pub show_textbox_frame: Option<bool>,    // NEU: Nur für die Haupt-Textbox
}

// ... (Der Rest von Action, CharacterPosition etc. bleibt gleich)

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    SetBackground { image_path: String },
    ShowCharacter { character_id: String, image_path: String }, // "position" entfernt!
    Dialogue { speaker_name: Option<String>, text: String, audio_path: Option<String> },
}

struct ActiveCharacter {
    id: String,
    image_path: String,
}

// --- DER PLAYER (ENGINE) ---
pub struct VnPlayer {
    chapter: Chapter,
    current_index: usize,

    current_background: Option<String>,
    current_bg_aspect: f32, // NEU: Merkt sich das dynamische Bildformat

    active_characters: Vec<ActiveCharacter>,

    current_speaker: Option<String>,
    current_text: String,

    // NEU: Lade-System
    is_loading: bool,
    preload_uris: Vec<String>,

    is_finished: bool,
    _audio_stream: MixerDeviceSink,
    audio_player: Player,
}

impl VnPlayer {
    pub fn new(chapter: Chapter) -> Self {
        let stream_handle = DeviceSinkBuilder::open_default_sink()
            .expect("Konnte kein Audio-Gerät finden!");
        let player = Player::connect_new(stream_handle.mixer());

        // NEU: Wir suchen VOR dem Start alle Bilder zusammen, die in diesem Kapitel vorkommen
        let mut preload_uris = Vec::new();
        for action in &chapter.actions {
            match action {
                Action::SetBackground { image_path } => {
                    let uri = format!("file://{}", image_path);
                    if !preload_uris.contains(&uri) { preload_uris.push(uri); }
                }
                Action::ShowCharacter { image_path, .. } => {
                    let uri = format!("file://{}", image_path);
                    if !preload_uris.contains(&uri) { preload_uris.push(uri); }
                }
                _ => {}
            }
        }

        let mut vn_player = Self {
            chapter,
            current_index: 0,
            current_background: None,
            current_bg_aspect: 16.0 / 9.0, // Standardwert
            active_characters: vec![],
            current_speaker: None,
            current_text: String::new(),
            is_loading: true, // Das Spiel startet im Ladebildschirm!
            preload_uris,
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

                    // NEU: Wir lesen in Millisekunden das Format des Bildes aus (z.B. für Hochkant-Bilder)
                    if let Ok(dimensions) = image::image_dimensions(image_path) {
                        self.current_bg_aspect = dimensions.0 as f32 / dimensions.1 as f32;
                    } else {
                        self.current_bg_aspect = 16.0 / 9.0;
                    }

                    self.current_index += 1;
                }
                Action::ShowCharacter { character_id, image_path } => {
                    self.active_characters.retain(|c| c.id != *character_id);

                    self.active_characters.push(ActiveCharacter {
                        id: character_id.clone(),
                        image_path: image_path.clone(),
                    });
                    self.current_index += 1;
                }
                Action::Dialogue { speaker_name, text, audio_path } => {
                    self.current_speaker = speaker_name.clone();
                    self.current_text = text.clone();

                    if let Some(path) = audio_path {
                        if let Ok(file) = std::fs::File::open(path) {
                            self.audio_player = Player::connect_new(self._audio_stream.mixer());
                            if let Ok(source) = Decoder::try_from(file) {
                                self.audio_player.append(source);
                            }
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

    // --- HILFSFUNKTION (REPARIERT): Zeichnet eine einzelne Porträt-Spalte ---
    fn draw_portrait_column(
        &self,
        ui: &mut egui::Ui,
        character: Option<&ActiveCharacter>,
        frame: egui::Frame,
        width: f32,
        height: f32
    ) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let mut col_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));

        col_ui.set_clip_rect(rect);

        if let Some(char) = character {
            let uri = format!("file://{}", char.image_path);

            frame.show(&mut col_ui, |ui| {
                let available_size = ui.available_size();

                // 1. Wir reservieren den exakten inneren Platz.
                // Das zwingt den Rahmen auf seine finale Größe, er wird nicht mehr wachsen.
                let (_, inner_rect) = ui.allocate_space(available_size);

                // 2. DER FIX: Wir erstellen eine UNABHÄNGIGE Child-UI für das Bild.
                // Wenn wir das Bild jetzt zeichnen, sprengt es nicht mehr die Grenzen
                // der Eltern-UI und der Rahmen bleibt komplett sichtbar!
                let mut image_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                image_ui.set_clip_rect(inner_rect); // Hier wird nur das Bild beschnitten

                // --- BILD LADEN UND SKALIEREN ---
                if let Ok(egui::load::TexturePoll::Ready { texture, .. }) =
                    ui.ctx().try_load_texture(&uri, egui::TextureOptions::default(), egui::SizeHint::default())
                {
                    // Wichtig: 'as f32' behalten, damit das Seitenverhältnis korrekt berechnet wird
                    let image_aspect = texture.size[0] / texture.size[1];
                    let frame_aspect = inner_rect.width() / inner_rect.height();

                    let mut target_size = inner_rect.size();

                    // Deine Original-Logik: Das Bild so skalieren, dass es die Box ausfüllt ("Cover")
                    if frame_aspect > image_aspect {
                        target_size.y = inner_rect.width() / image_aspect;
                    } else {
                        target_size.x = inner_rect.height() * image_aspect;
                    }

                    // Ausrichtung: Oben anlegen (Top-Left)
                    let image_rect = egui::Rect::from_min_size(inner_rect.min, target_size);

                    let image = egui::Image::new(&uri)
                        .fit_to_exact_size(target_size)
                        .maintain_aspect_ratio(false);

                    // 3. Wir zeichnen das Bild in unsere isolierte image_ui!
                    image_ui.put(image_rect, image);

                } else {
                    let mut spinner_ui = image_ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                    spinner_ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                }
            });
        }
    }
}

// --- EGUI BENUTZEROBERFLÄCHE ---
impl eframe::App for VnPlayer {

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        // --- DER LADEBILDSCHIRM ---
        if self.is_loading {
            let mut loaded_count = 0;

            for uri in &self.preload_uris {
                // FIX: Wir nutzen 'try_load_texture' statt 'try_load_image'
                // und übergeben zusätzlich die Standard-Texturoptionen!
                if let Ok(egui::load::TexturePoll::Ready { .. }) = ui.ctx().try_load_texture(
                    uri,
                    egui::TextureOptions::default(),
                    egui::SizeHint::default()
                ) {
                    loaded_count += 1;
                }
            }

            if loaded_count < self.preload_uris.len() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Lade Kapitel...");
                        ui.add_space(20.0);
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label(format!("Assets geladen: {} / {}", loaded_count, self.preload_uris.len()));
                    });
                });
                ui.ctx().request_repaint();
                return;
            } else {
                self.is_loading = false;
            }
        }

        // --- DAS EIGENTLICHE SPIEL ---
        if self.is_finished {
            ui.centered_and_justified(|ui| {
                ui.heading("Kapitel beendet.");
            });
            return;
        }

        let mut advance_chapter = false;
        let full_rect = ui.max_rect();

        // --- 1. HINTERGRUND (Dynamischer "Cover" Zoom) ---
        if let Some(bg) = &self.current_background {
            let uri = format!("file://{}", bg);

            let mut bg_ui = ui.new_child(egui::UiBuilder::new().max_rect(full_rect));
            bg_ui.set_clip_rect(full_rect); // Alles abschneiden, was übersteht!

            let bg_aspect = self.current_bg_aspect;
            let win_aspect = full_rect.width() / full_rect.height();

            let mut target_size = full_rect.size();

            if win_aspect > bg_aspect {
                target_size.y = full_rect.width() / bg_aspect;
            } else {
                target_size.x = full_rect.height() * bg_aspect;
            }

            bg_ui.centered_and_justified(|ui| {
                let image = egui::Image::new(&uri)
                    .fit_to_exact_size(target_size)
                    .maintain_aspect_ratio(false); // Die Mathematik oben verhindert die Verzerrung

                ui.add(image);
            });
        }

        // --- 2. TEXTBOX BEREICH ---
        let textbox_height = (full_rect.height() * 0.30).max(180.0);
        let mut textbox_rect = full_rect;
        textbox_rect.set_top(full_rect.bottom() - textbox_height);

        // --- 3. SPRECHER ZUORDNEN ---
        let mut active_char = None;
        let mut inactive_char = None;

        if let Some(speaker) = &self.current_speaker {
            let speaker_lower = speaker.to_lowercase();
            for char in &self.active_characters {
                if char.id.to_lowercase() == speaker_lower {
                    active_char = Some(char);
                } else if inactive_char.is_none() {
                    inactive_char = Some(char);
                }
            }
        } else {
            let mut iter = self.active_characters.iter();
            active_char = iter.next();
            inactive_char = iter.next();
        }

        // --- 4. THEME AUSLESEN ---
        let mut box_bg = egui::Color32::from_black_alpha(230);
        let mut frame_color = egui::Color32::from_white_alpha(150);
        let mut show_char_frames = false;
        let mut show_text_frame = false;

        if let Some(theme) = &self.chapter.theme {
            if let Some(c) = theme.textbox_color { box_bg = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]); }
            if let Some(c) = theme.frame_color { frame_color = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]); }
            if let Some(scf) = theme.show_character_frames { show_char_frames = scf; }
            if let Some(stf) = theme.show_textbox_frame { show_text_frame = stf; }
        }

        // --- 5. TEXTBOX UND PORTRÄTS ZEICHNEN ---
        let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(textbox_rect));

        egui::Frame::default()
            .fill(box_bg)
            // Nutzt jetzt die EIGENE Variable für den Textbox-Rahmen!
            .stroke(if show_text_frame { egui::Stroke::new(2.0, frame_color) } else { egui::Stroke::NONE })
            .inner_margin(0.0)
            .show(&mut text_ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // --- HIER IST DIE NEUE MATHEMATIK FÜR DIE PORTRÄTS ---
                // Anstatt die Breite vom Fenster zu nehmen, koppeln wir sie an die HÖHE der Textbox.
                // Faktor 1.0 = Ein perfektes Quadrat (z.B. 180x180 Pixel).
                // Faktor 0.8 = Etwas schmaler (Klassisches Hochkant-Porträt).
                let portrait_width = textbox_height * 0.8;

                // Der Text bekommt den restlichen Platz. (max(100.0) verhindert Abstürze bei extrem winzigen Fenstern)
                let text_width = (textbox_rect.width() - (portrait_width * 2.0)).max(100.0);

                // --- DEN RAHMEN FÜR CHARAKTERE DEFINIEREN ---
                let portrait_frame = if show_char_frames {
                    egui::Frame::default()
                        .fill(egui::Color32::from_black_alpha(100))
                        .stroke(egui::Stroke::new(3.0, frame_color))
                        .corner_radius(10.0)
                        .inner_margin(5.0)
                } else {
                    egui::Frame::default()
                };

                ui.horizontal(|ui| {

                    // --- LINKE SPALTE (Aktiver Sprecher) ---
                    // Wir rufen einfach unsere neue Funktion auf! (.clone() stellt sicher, dass wir den Rahmen wiederverwenden können)
                    self.draw_portrait_column(ui, active_char, portrait_frame.clone(), portrait_width, textbox_height);

                    // --- MITTLERE SPALTE (Text) ---
                    let (mid_rect, _) = ui.allocate_exact_size(egui::vec2(text_width, textbox_height), egui::Sense::hover());
                    let mut mid_ui = ui.new_child(egui::UiBuilder::new().max_rect(mid_rect));

                    egui::Frame::default().inner_margin(20.0).show(&mut mid_ui, |ui| {
                        ui.vertical(|ui| {
                            if let Some(speaker) = &self.current_speaker {
                                ui.heading(egui::RichText::new(speaker).color(egui::Color32::from_rgb(100, 200, 255)));
                            } else {
                                ui.heading(egui::RichText::new("Erzähler").italics().color(egui::Color32::GRAY));
                            }

                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(&self.current_text).size(20.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                                if ui.button("Weiter ➡").clicked() || ui.ctx().input(|i| i.key_pressed(egui::Key::Space)) {
                                    advance_chapter = true;
                                }
                                ui.label(egui::RichText::new("(Leertaste)").small());
                            });
                        });
                    });

                    // --- RECHTE SPALTE (Inaktiver Sprecher) ---
                    self.draw_portrait_column(ui, inactive_char, portrait_frame, portrait_width, textbox_height);

                });
            });

        ui.ctx().request_repaint();

        if advance_chapter {
            self.next_action();
        }
    }
}