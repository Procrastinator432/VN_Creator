use eframe::egui;
use crate::player::{Chapter, Action, Theme};
use std::path::PathBuf;
use std::collections::HashMap; // NEU: Um sich die Audio-Längen zu merken
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source}; // NEU: 'Source' für die Länge

pub struct VnEditor {
    pub chapter: Chapter,
    pub collapsed_states: Vec<bool>,
    _audio_stream: MixerDeviceSink,
    audio_player: Player,
    audio_durations: HashMap<String, String>, // NEU: Cache für die Längen
}

impl VnEditor {
    pub fn new() -> Self {
        let stream_handle = DeviceSinkBuilder::open_default_sink().expect("Kein Audio-Gerät!");
        let player = Player::connect_new(stream_handle.mixer());
        Self {
            chapter: Chapter {
                title: String::new(),
                theme: None,
                // bg_music: None ist hier weg, da es jetzt eine Karte ist!
                actions: vec![]
            },
            collapsed_states: vec![],
            _audio_stream: stream_handle,
            audio_player: player,
            audio_durations: HashMap::new(), // Startet leer
        }
    }

    fn color_edit(ui: &mut egui::Ui, label: &str, color_opt: &mut Option<[u8; 4]>) {
        ui.horizontal(|ui| {
            ui.label(label);
            let array = color_opt.unwrap_or([200, 200, 200, 255]);
            let mut egui_color = egui::Color32::from_rgba_unmultiplied(array[0], array[1], array[2], array[3]);
            if ui.color_edit_button_srgba(&mut egui_color).changed() {
                *color_opt = Some([egui_color.r(), egui_color.g(), egui_color.b(), egui_color.a()]);
            }
            if color_opt.is_some() && ui.button("🗑 Reset").clicked() {
                *color_opt = None;
            }
        });
    }

    fn optional_text_edit(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut current_text = value.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut current_text).changed() {
                *value = if current_text.trim().is_empty() { None } else { Some(current_text) };
            }
        });
    }

    // --- NEU: Zentrale Datei-Auswahl (DRY Fix) ---
    fn pick_asset_path(default_folder: &str) -> Option<String> {
        let start_dir = PathBuf::from(default_folder);
        let mut dialog = rfd::FileDialog::new();

        if start_dir.exists() {
            dialog = dialog.set_directory(&start_dir);
        }

        // Filter automatisch anhand des Ordnernamens setzen
        if default_folder.contains("pictures") {
            dialog = dialog.add_filter("Bilder", &["png", "jpg", "jpeg", "webp"]);
        } else if default_folder.contains("audio") {
            dialog = dialog.add_filter("Audio", &["mp3", "wav", "ogg"]);
        }

        // Wenn eine Datei gewählt wurde, relativen Pfad zurückgeben
        if let Some(picked_path) = dialog.pick_file() {
            let path_str = picked_path.to_string_lossy().to_string();
            return if let Some(index) = path_str.find("assets") {
                Some(path_str[index..].replace("\\", "/"))
            } else {
                Some(path_str.replace("\\", "/"))
            }
        }
        None
    }

    fn path_edit_with_browser(ui: &mut egui::Ui, label: &str, path_string: &mut String, default_folder: &str) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(path_string);

            // Viel sauberer: Wir rufen nur noch unsere Hilfsfunktion auf!
            if ui.button("📂 Wahl").clicked() {
                if let Some(new_path) = Self::pick_asset_path(default_folder) {
                    *path_string = new_path;
                }
            }
        });

        if default_folder.contains("pictures") && !path_string.trim().is_empty() {
            ui.add_space(5.0);
            let screen_height = ui.ctx().content_rect().height();
            let dynamic_height = (screen_height * 0.15).clamp(60.0, 250.0);

            ui.horizontal(|ui| {
                ui.set_min_height(dynamic_height);
                ui.add(egui::Image::new(&format!("file://{}", path_string))
                    .max_height(dynamic_height)
                    .corner_radius(4.0)
                );
            });
        }
    }

    fn audio_path_edit_with_browser(ui: &mut egui::Ui, label: &str, path_string: &mut String, default_folder: &str, stream: &MixerDeviceSink, player: &mut Player, durations: &mut HashMap<String, String>) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(path_string);

            // Auch hier: Einzeiler statt Code-Mauer!
            if ui.button("📂 Wahl").clicked() {
                if let Some(new_path) = Self::pick_asset_path(default_folder) {
                    *path_string = new_path;
                }
            }

            if !path_string.trim().is_empty() {
                if ui.button("▶").clicked() {
                    if let Ok(file) = std::fs::File::open(&*path_string) {
                        *player = Player::connect_new(stream.mixer());
                        if let Ok(src) = Decoder::try_from(file) { player.append(src); }
                    }
                }

                if !durations.contains_key(path_string) {
                    let dur_str = if let Ok(file) = std::fs::File::open(&*path_string) {
                        if let Ok(src) = Decoder::try_from(file) {
                            if let Some(d) = src.total_duration() { format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60) }
                            else { "??:??".to_string() }
                        } else { "Err".to_string() }
                    } else { "Err".to_string() };
                    durations.insert(path_string.clone(), dur_str);
                }

                if let Some(d_str) = durations.get(path_string) {
                    ui.label(egui::RichText::new(format!("⏱ {}", d_str)).weak());
                }
            }
        });
    }

    // --- ANGEPASST: Optionale Pfad-Eingabe für Voice-Lines (jetzt auch mit Längenanzeige) ---
    fn optional_audio_path_edit_with_browser(ui: &mut egui::Ui, label: &str, value: &mut Option<String>, default_folder: &str, stream: &MixerDeviceSink, player: &mut Player, durations: &mut HashMap<String, String>) {
        let mut current_text = value.clone().unwrap_or_default();
        Self::audio_path_edit_with_browser(ui, label, &mut current_text, default_folder, stream, player, durations);
        *value = if current_text.trim().is_empty() { None } else { Some(current_text) };
    }

    fn process_asset_path(original_path: &mut String, target_folder: &str) {
        if original_path.trim().is_empty() { return; }
        let path = std::path::Path::new(original_path.as_str());
        if original_path.starts_with("assets/") || original_path.starts_with("assets\\") { return; }
        if path.exists() {
            let _ = std::fs::create_dir_all(target_folder);
            if let Some(file_name) = path.file_name() {
                let mut target = PathBuf::from(target_folder);
                target.push(file_name);
                if let Ok(_) = std::fs::copy(path, &target) { *original_path = target.to_string_lossy().replace("\\", "/"); }
            }
        }
    }

    fn process_optional_asset_path(original_path: &mut Option<String>, target_folder: &str) {
        if let Some(path_str) = original_path {
            let mut temp = path_str.clone();
            Self::process_asset_path(&mut temp, target_folder);
            *original_path = Some(temp);
        }
    }
}

impl VnEditor {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // audio_durations destrukturieren
        let VnEditor { chapter, collapsed_states, _audio_stream, audio_player, audio_durations } = self;

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                if ui.button("💾 Speichern").clicked() {
                    let safe_title: String = chapter.title.trim().replace(" ", "_").chars().filter(|c| !r#"<>:"/\|?*"#.contains(*c)).collect();
                    let name = if safe_title.is_empty() { "neues_kapitel.json".to_string() } else { format!("{}.json", safe_title) };
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).set_file_name(&name).save_file() {

                        for action in &mut chapter.actions {
                            match action {
                                Action::SetBackground { image_path } => Self::process_asset_path(image_path, "assets/pictures/backgrounds"),
                                Action::ShowCharacter { image_path, .. } => Self::process_asset_path(image_path, "assets/pictures/characters"),
                                Action::Dialogue { audio_path, .. } => Self::process_optional_asset_path(audio_path, "assets/audio/voices"),
                                Action::PlayMusic { audio_path } => Self::process_asset_path(audio_path, "assets/audio/music"), // NEU
                            }
                        }
                        let _ = std::fs::write(&path, serde_json::to_string_pretty(&chapter).unwrap());
                    }
                }
                if ui.button("📂 Laden").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        if let Ok(s) = std::fs::read_to_string(&path) {
                            if let Ok(loaded) = serde_json::from_str::<Chapter>(&s) {
                                *chapter = loaded;
                                *collapsed_states = vec![false; chapter.actions.len()];
                                audio_durations.clear(); // Cache leeren beim Laden!
                            }
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Visual Novel Editor");
            ui.separator();

            // KAPITEL TITEL
            ui.horizontal(|ui| {
                ui.label("Kapitel Titel:");
                ui.add(egui::TextEdit::singleline(&mut chapter.title).hint_text("Neues Kapitel"));
            });

            ui.add_space(10.0);

            // VISUELLER THEME-EDITOR
            ui.collapsing("🎨 Kapitel-Design (Theme)", |ui| {

                let mut delete_theme = false;

                if chapter.theme.is_none() {
                    if ui.button("✨ Eigenes Theme erstellen").clicked() {
                        chapter.theme = Some(Theme { textbox_color: None, frame_color: None, show_character_frames: None, show_textbox_frame: None });
                    }
                } else {
                    if let Some(theme) = &mut chapter.theme {
                        ui.vertical(|ui| {
                            Self::color_edit(ui, "Textbox Farbe:", &mut theme.textbox_color);
                            Self::color_edit(ui, "Rahmen Farbe:", &mut theme.frame_color);

                            ui.separator();

                            let mut show_char = theme.show_character_frames.unwrap_or(false);
                            if ui.checkbox(&mut show_char, "Charakter-Rahmen anzeigen").changed() { theme.show_character_frames = Some(show_char); }

                            let mut show_box = theme.show_textbox_frame.unwrap_or(false);
                            if ui.checkbox(&mut show_box, "Textbox-Rahmen anzeigen").changed() { theme.show_textbox_frame = Some(show_box); }

                            if ui.button("🗑 Theme löschen (Standard nutzen)").clicked() {
                                delete_theme = true;
                            }
                        });
                    }

                    if delete_theme {
                        chapter.theme = None;
                    }
                }
            });

            ui.add_space(20.0);

            // TIMELINE UND AKTIONEN
            ui.horizontal(|ui| {
                ui.heading("Aktionen");
                if ui.button("➕ BG").clicked() { chapter.actions.push(Action::SetBackground { image_path: String::new() }); }
                if ui.button("➕ Char").clicked() { chapter.actions.push(Action::ShowCharacter { character_id: String::new(), image_path: String::new() }); }
                if ui.button("➕ Dialog").clicked() { chapter.actions.push(Action::Dialogue { speaker_name: None, text: String::new(), audio_path: None }); }
                if ui.button("➕ Musik").clicked() { chapter.actions.push(Action::PlayMusic { audio_path: String::new() }); } // NEU
            });

            let mut action_to_delete = None;
            let mut action_to_swap = None;
            if collapsed_states.len() != chapter.actions.len() { collapsed_states.resize(chapter.actions.len(), false); }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for index in 0..chapter.actions.len() {

                    ui.push_id(index, |ui| {
                        let is_collapsed = collapsed_states[index];
                        egui::Frame::default().stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(80))).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("☰");
                                if ui.button(if is_collapsed { "+" } else { "-" }).clicked() { collapsed_states[index] = !is_collapsed; }
                                match &chapter.actions[index] {
                                    Action::SetBackground { .. } => ui.label("🌄 Hintergrund"),
                                    Action::ShowCharacter { .. } => ui.label("👤 Charakter"),
                                    Action::Dialogue { .. } => ui.label("💬 Dialog"),
                                    Action::PlayMusic { .. } => ui.label("🎵 Musik"), // NEU
                                };
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("❌").clicked() { action_to_delete = Some(index); }
                                    if index < chapter.actions.len() - 1 && ui.button("⬇").clicked() { action_to_swap = Some((index, index+1)); }
                                    if index > 0 && ui.button("⬆").clicked() { action_to_swap = Some((index, index-1)); }
                                });
                            });
                            if !is_collapsed {
                                ui.separator();
                                match &mut chapter.actions[index] {
                                    Action::SetBackground { image_path } => Self::path_edit_with_browser(ui, "Pfad:", image_path, "assets/pictures/backgrounds"),
                                    Action::ShowCharacter { character_id, image_path } => {
                                        ui.text_edit_singleline(character_id);
                                        Self::path_edit_with_browser(ui, "Pfad:", image_path, "assets/pictures/characters");
                                    }
                                    Action::Dialogue { speaker_name, text, audio_path } => {
                                        Self::optional_text_edit(ui, "Sprecher:", speaker_name);

                                        let screen_height = ui.ctx().content_rect().height();
                                        let dialog_height = (screen_height * 0.10).max(60.0);

                                        ui.add_sized(
                                            egui::vec2(ui.available_width(), dialog_height),
                                            egui::TextEdit::multiline(text)
                                        );

                                        // NUTZT JETZT DIE NEUE FUNKTION MIT LÄNGENANZEIGE
                                        Self::optional_audio_path_edit_with_browser(ui, "Voice:", audio_path, "assets/audio/voices", _audio_stream, audio_player, audio_durations);
                                    }
                                    // NEU: Musik-Karte
                                    Action::PlayMusic { audio_path } => {
                                        Self::audio_path_edit_with_browser(ui, "Musik-Pfad:", audio_path, "assets/audio/music", _audio_stream, audio_player, audio_durations);
                                    }
                                }
                            }
                        });
                        ui.add_space(5.0);
                    });
                }
            });

            if let Some((i, j)) = action_to_swap { chapter.actions.swap(i, j); collapsed_states.swap(i, j); }
            if let Some(idx) = action_to_delete { chapter.actions.remove(idx); collapsed_states.remove(idx); }
        });
    }
}