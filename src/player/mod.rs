use eframe::egui;
use serde::{Deserialize, Serialize};
// NEU: 'Source' importiert, damit wir die Musik in Dauerschleife (.repeat_infinite()) abspielen können!
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

// --- DATENMODEL ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
    pub title: String,
    pub theme: Option<Theme>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Theme {
    pub textbox_color: Option<[u8; 4]>,
    pub frame_color: Option<[u8; 4]>,
    pub show_character_frames: Option<bool>,
    pub show_textbox_frame: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    SetBackground { image_path: String },
    ShowCharacter { character_id: String, image_path: String },
    Dialogue { speaker_name: Option<String>, text: String, audio_path: Option<String> },
    PlayMusic { audio_path: String }, // NEU: Musik als Aktion
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
    current_bg_aspect: f32,

    active_characters: Vec<ActiveCharacter>,

    current_speaker: Option<String>,
    current_text: String,

    is_loading: bool,
    preload_uris: Vec<String>,

    is_finished: bool,

    // --- AUDIO SYSTEM ---
    _audio_stream: MixerDeviceSink,
    audio_player: Player, // Für Dialoge/Sprache
    bgm_player: Player,   // NEU: Für Hintergrundmusik
}

impl VnPlayer {
    pub fn new(chapter: Chapter) -> Self {
        let stream_handle = DeviceSinkBuilder::open_default_sink()
            .expect("Konnte kein Audio-Gerät finden!");

        // Wir erstellen direkt ZWEI getrennte Player, die auf demselben Mixer liegen!
        let voice_player = Player::connect_new(stream_handle.mixer());
        let bgm_player = Player::connect_new(stream_handle.mixer());

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
            current_bg_aspect: 16.0 / 9.0,
            active_characters: vec![],
            current_speaker: None,
            current_text: String::new(),
            is_loading: true,
            preload_uris,
            is_finished: false,
            _audio_stream: stream_handle,
            audio_player: voice_player, // Spieler 1 für Voice
            bgm_player,                 // Spieler 2 für Musik
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
                // --- FIX: DIE FEHLENDE MUSIK LOGIK WURDE HINZUGEFÜGT ---
                Action::PlayMusic { audio_path } => {
                    // Egal was passiert: Wir stoppen alte Musik, indem wir den BGM-Player erneuern
                    self.bgm_player = Player::connect_new(self._audio_stream.mixer());

                    // Wenn der Pfad nicht leer ist, starten wir das neue Lied
                    if !audio_path.trim().is_empty() {
                        if let Ok(file) = std::fs::File::open(audio_path) {
                            if let Ok(source) = Decoder::try_from(file) {
                                // .repeat_infinite() zwingt den Player, den Track endlos zu wiederholen!
                                self.bgm_player.append(source.repeat_infinite());
                            }
                        }
                    }
                    // Danach machen wir direkt weiter, da Musik das Spiel nicht stoppt!
                    self.current_index += 1;
                }
                Action::Dialogue { speaker_name, text, audio_path } => {
                    self.current_speaker = speaker_name.clone();
                    self.current_text = text.clone();

                    // Bei jedem neuen Satz erneuern wir den VOICE-Player, damit sich Stimmen nicht überlagern
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
                let (_, inner_rect) = ui.allocate_space(available_size);

                let mut image_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                image_ui.set_clip_rect(inner_rect);

                if let Ok(egui::load::TexturePoll::Ready { texture, .. }) =
                    ui.ctx().try_load_texture(&uri, egui::TextureOptions::default(), egui::SizeHint::default())
                {
                    let image_aspect = texture.size[0] / texture.size[1];
                    let frame_aspect = inner_rect.width() / inner_rect.height();
                    let mut target_size = inner_rect.size();

                    if frame_aspect > image_aspect {
                        target_size.y = inner_rect.width() / image_aspect;
                    } else {
                        target_size.x = inner_rect.height() * image_aspect;
                    }

                    let image_rect = egui::Rect::from_min_size(inner_rect.min, target_size);
                    let image = egui::Image::new(&uri).fit_to_exact_size(target_size).maintain_aspect_ratio(false);
                    image_ui.put(image_rect, image);
                } else {
                    let mut spinner_ui = image_ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                    spinner_ui.centered_and_justified(|ui| { ui.spinner(); });
                }
            });
        }
    }
}

// --- EGUI BENUTZEROBERFLÄCHE ---
impl VnPlayer {
    pub fn ui(&mut self, ui: &mut egui::Ui) {

        if self.is_loading {
            let mut loaded_count = 0;
            for uri in &self.preload_uris {
                if let Ok(egui::load::TexturePoll::Ready { .. }) = ui.ctx().try_load_texture(
                    uri, egui::TextureOptions::default(), egui::SizeHint::default()
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

        if self.is_finished {
            ui.centered_and_justified(|ui| {
                ui.heading("Kapitel beendet.");
            });
            return;
        }

        let mut advance_chapter = false;
        let full_rect = ui.max_rect();

        if let Some(bg) = &self.current_background {
            let uri = format!("file://{}", bg);
            let mut bg_ui = ui.new_child(egui::UiBuilder::new().max_rect(full_rect));
            bg_ui.set_clip_rect(full_rect);

            let bg_aspect = self.current_bg_aspect;
            let win_aspect = full_rect.width() / full_rect.height();
            let mut target_size = full_rect.size();

            if win_aspect > bg_aspect {
                target_size.y = full_rect.width() / bg_aspect;
            } else {
                target_size.x = full_rect.height() * bg_aspect;
            }

            bg_ui.centered_and_justified(|ui| {
                let image = egui::Image::new(&uri).fit_to_exact_size(target_size).maintain_aspect_ratio(false);
                ui.add(image);
            });
        }

        let textbox_height = (full_rect.height() * 0.30).max(180.0);
        let mut textbox_rect = full_rect;
        textbox_rect.set_top(full_rect.bottom() - textbox_height);

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

        let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(textbox_rect));

        egui::Frame::default()
            .fill(box_bg)
            .stroke(if show_text_frame { egui::Stroke::new(2.0, frame_color) } else { egui::Stroke::NONE })
            .inner_margin(0.0)
            .show(&mut text_ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                let portrait_width = textbox_height * 0.8;
                let text_width = (textbox_rect.width() - (portrait_width * 2.0)).max(100.0);

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
                    self.draw_portrait_column(ui, active_char, portrait_frame.clone(), portrait_width, textbox_height);

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

                    self.draw_portrait_column(ui, inactive_char, portrait_frame, portrait_width, textbox_height);
                });
            });

        ui.ctx().request_repaint();

        if advance_chapter {
            self.next_action();
        }
    }
}