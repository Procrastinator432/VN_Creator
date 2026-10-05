use eframe::egui;
use crate::models::{Chapter, Action, Theme, ChoiceOption};
use crate::settings::Settings;
use std::path::PathBuf;
use std::collections::HashMap;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};

pub struct VnEditor {
    pub chapter: Chapter,
    pub collapsed_states: Vec<bool>,
    pub selected_action: Option<usize>,
    _audio_stream: MixerDeviceSink,
    audio_player: Player,
    audio_durations: HashMap<String, String>,
}

impl VnEditor {
    pub fn new() -> Self {
        let stream_handle = DeviceSinkBuilder::open_default_sink().expect("Kein Audio-Gerät!");
        let player = Player::connect_new(stream_handle.mixer());
        Self {
            chapter: Chapter {
                title: String::new(),
                theme: None,
                actions: vec![]
            },
            collapsed_states: vec![],
            selected_action: None,
            _audio_stream: stream_handle,
            audio_player: player,
            audio_durations: HashMap::new(),
        }
    }

    pub fn with_chapter(chapter: Chapter) -> Self {
        let mut editor = Self::new();
        editor.collapsed_states = vec![false; chapter.actions.len()];
        editor.chapter = chapter;
        editor
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

    fn pick_asset_path(default_folder: &str) -> Option<String> {
        let start_dir = PathBuf::from(default_folder);
        let mut dialog = rfd::FileDialog::new();

        if start_dir.exists() {
            dialog = dialog.set_directory(&start_dir);
        }

        if default_folder.contains("pictures") {
            dialog = dialog.add_filter("Bilder", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "ico", "tiff", "tga"]);
        } else if default_folder.contains("audio") {
            dialog = dialog.add_filter("Audio", &["mp3", "wav", "ogg"]);
        }

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

    fn optional_path_edit_with_browser(ui: &mut egui::Ui, label: &str, value: &mut Option<String>, default_folder: &str) {
        let mut current_text = value.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(&mut current_text);
            if ui.button("📂 Wahl").clicked() {
                if let Some(new_path) = Self::pick_asset_path(default_folder) {
                    current_text = new_path;
                }
            }
            if value.is_some() && ui.button("🗑 Reset").clicked() {
                current_text.clear();
            }
        });
        *value = if current_text.trim().is_empty() { None } else { Some(current_text.clone()) };

        if default_folder.contains("pictures") && !current_text.trim().is_empty() {
            ui.add_space(5.0);
            let screen_height = ui.ctx().content_rect().height();
            let dynamic_height = (screen_height * 0.12).clamp(50.0, 180.0);

            ui.horizontal(|ui| {
                ui.set_min_height(dynamic_height);
                ui.add(egui::Image::new(&format!("file://{}", current_text))
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
    pub fn ui(&mut self, ui: &mut egui::Ui, settings: &Settings) {
        let mut trigger_save = false;
        let mut trigger_add_bg = false;
        let mut trigger_add_char = false;
        let mut trigger_add_hide_char = false;
        let mut trigger_add_dialogue = false;
        let mut trigger_add_music = false;
        let mut trigger_add_sfx = false;
        let mut trigger_add_label = false;
        let mut trigger_add_jump = false;
        let mut trigger_add_choice = false;
        let mut trigger_select_up = false;
        let mut trigger_select_down = false;
        let mut trigger_move_up = false;
        let mut trigger_move_down = false;
        let mut trigger_delete = false;
        
        let binds = &settings.keybindings;
        if binds.editor_save.is_pressed(ui) { trigger_save = true; }
        if binds.editor_add_bg.is_pressed(ui) { trigger_add_bg = true; }
        if binds.editor_add_char.is_pressed(ui) { trigger_add_char = true; }
        if binds.editor_add_hide_char.is_pressed(ui) { trigger_add_hide_char = true; }
        if binds.editor_add_dialogue.is_pressed(ui) { trigger_add_dialogue = true; }
        if binds.editor_add_music.is_pressed(ui) { trigger_add_music = true; }
        if binds.editor_add_sfx.is_pressed(ui) { trigger_add_sfx = true; }
        if binds.editor_add_label.is_pressed(ui) { trigger_add_label = true; }
        if binds.editor_add_jump.is_pressed(ui) { trigger_add_jump = true; }
        if binds.editor_add_choice.is_pressed(ui) { trigger_add_choice = true; }
        if binds.editor_select_up.is_pressed(ui) { trigger_select_up = true; }
        if binds.editor_select_down.is_pressed(ui) { trigger_select_down = true; }
        if binds.editor_move_up.is_pressed(ui) { trigger_move_up = true; }
        if binds.editor_move_down.is_pressed(ui) { trigger_move_down = true; }
        if binds.editor_delete_action.is_pressed(ui) { trigger_delete = true; }

        let VnEditor { chapter, collapsed_states, selected_action, _audio_stream, audio_player, audio_durations } = self;

        if trigger_select_up {
            if let Some(sel) = selected_action {
                if *sel > 0 { *sel -= 1; }
            } else if !chapter.actions.is_empty() {
                *selected_action = Some(chapter.actions.len() - 1);
            }
        }
        if trigger_select_down {
            if let Some(sel) = selected_action {
                if *sel < chapter.actions.len() - 1 { *sel += 1; }
            } else if !chapter.actions.is_empty() {
                *selected_action = Some(0);
            }
        }

        let mut action_to_delete = None;
        let mut action_to_swap = None;

        if trigger_move_up {
            if let Some(sel) = selected_action {
                if *sel > 0 { action_to_swap = Some((*sel, *sel - 1)); }
            }
        }
        if trigger_move_down {
            if let Some(sel) = selected_action {
                if *sel < chapter.actions.len() - 1 { action_to_swap = Some((*sel, *sel + 1)); }
            }
        }
        if trigger_delete {
            if let Some(sel) = selected_action {
                action_to_delete = Some(*sel);
                *selected_action = None;
            }
        }

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                if ui.button("💾 Speichern").clicked() || trigger_save {
                    let safe_title: String = chapter.title.trim().replace(" ", "_").chars().filter(|c| !r#"<>:"/\|?*"#.contains(*c)).collect();
                    let name = if safe_title.is_empty() { "neues_kapitel.json".to_string() } else { format!("{}.json", safe_title) };
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).set_file_name(&name).save_file() {
                        for action in &mut chapter.actions {
                            match action {
                                Action::SetBackground { image_path } => Self::process_asset_path(image_path, "assets/pictures/backgrounds"),
                                Action::ShowCharacter { image_path, .. } => Self::process_asset_path(image_path, "assets/pictures/characters"),
                                Action::HideCharacter { .. } => {},
                                Action::Label { .. } => {},
                                Action::Jump { .. } => {},
                                Action::Choice { .. } => {},
                                Action::Dialogue { audio_path, .. } => Self::process_optional_asset_path(audio_path, "assets/audio/voices"),
                                Action::PlayMusic { audio_path } => Self::process_asset_path(audio_path, "assets/audio/music"),
                                Action::PlaySound { audio_path } => Self::process_asset_path(audio_path, "assets/audio/sfx"),
                            }
                        }
                        if let Some(theme) = &mut chapter.theme {
                            Self::process_optional_asset_path(&mut theme.character_frame_image, "assets/pictures/frames");
                        }
                        if let Ok(json_str) = serde_json::to_string_pretty(&chapter) {
                            let _ = std::fs::write(&path, json_str);
                        }
                    }
                }
                if ui.button("📂 Laden").clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                        if let Ok(s) = std::fs::read_to_string(&path) {
                            if let Ok(loaded) = serde_json::from_str::<Chapter>(&s) {
                                *chapter = loaded;
                                *collapsed_states = vec![false; chapter.actions.len()];
                                *selected_action = None;
                                audio_durations.clear();
                            }
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Visual Novel Editor");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Kapitel Titel:");
                ui.add(egui::TextEdit::singleline(&mut chapter.title).hint_text("Neues Kapitel"));
            });

            ui.add_space(10.0);

            ui.collapsing("🎨 Kapitel-Design (Theme)", |ui| {
                let mut delete_theme = false;
                if chapter.theme.is_none() {
                    if ui.button("✨ Eigenes Theme erstellen").clicked() {
                        chapter.theme = Some(Theme {
                            textbox_color: None,
                            frame_color: None,
                            show_character_frames: None,
                            show_textbox_frame: None,
                            character_frame_image: None,
                        });
                    }
                } else {
                    if let Some(theme) = &mut chapter.theme {
                        ui.vertical(|ui| {
                            Self::color_edit(ui, "Textbox Farbe:", &mut theme.textbox_color);
                            Self::color_edit(ui, "Rahmen Farbe:", &mut theme.frame_color);
                            ui.separator();
                            let mut show_char = theme.show_character_frames.unwrap_or(false);
                            if ui.checkbox(&mut show_char, "Charakter-Rahmen anzeigen (Farbe)").changed() { theme.show_character_frames = Some(show_char); }
                            let mut show_box = theme.show_textbox_frame.unwrap_or(false);
                            if ui.checkbox(&mut show_box, "Textbox-Rahmen anzeigen").changed() { theme.show_textbox_frame = Some(show_box); }
                            ui.separator();
                            Self::optional_path_edit_with_browser(ui, "Charakter-Rahmen (Bild):", &mut theme.character_frame_image, "assets/pictures/frames");
                            ui.separator();
                            if ui.button("🗑 Theme löschen (Standard nutzen)").clicked() { delete_theme = true; }
                        });
                    }
                    if delete_theme { chapter.theme = None; }
                }
            });

            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.heading("Aktionen");
                
                let mut added_action = None;
                if ui.button("➕ BG").clicked() || trigger_add_bg { added_action = Some(Action::SetBackground { image_path: String::new() }); }
                if ui.button("➕ Char").clicked() || trigger_add_char { added_action = Some(Action::ShowCharacter { character_id: String::new(), image_path: String::new() }); }
                if ui.button("➖ Hide Char").clicked() || trigger_add_hide_char { added_action = Some(Action::HideCharacter { character_id: String::new() }); }
                if ui.button("➕ Dialog").clicked() || trigger_add_dialogue { added_action = Some(Action::Dialogue { speaker_name: None, text: String::new(), audio_path: None }); }
                if ui.button("➕ Musik").clicked() || trigger_add_music { added_action = Some(Action::PlayMusic { audio_path: String::new() }); }
                if ui.button("🔊 SFX").clicked() || trigger_add_sfx { added_action = Some(Action::PlaySound { audio_path: String::new() }); }
                if ui.button("🏷 Label").clicked() || trigger_add_label { added_action = Some(Action::Label { name: String::new() }); }
                if ui.button("↪ Jump").clicked() || trigger_add_jump { added_action = Some(Action::Jump { target_label: String::new() }); }
                if ui.button("❓ Choice").clicked() || trigger_add_choice { added_action = Some(Action::Choice { question: String::new(), options: vec![] }); }
                
                if let Some(act) = added_action {
                    if let Some(sel) = selected_action {
                        chapter.actions.insert(*sel + 1, act);
                        collapsed_states.insert(*sel + 1, false);
                        *sel += 1;
                    } else {
                        chapter.actions.push(act);
                        collapsed_states.push(false);
                        *selected_action = Some(chapter.actions.len() - 1);
                    }
                }
            });

            if collapsed_states.len() != chapter.actions.len() { collapsed_states.resize(chapter.actions.len(), false); }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for index in 0..chapter.actions.len() {
                    ui.push_id(index, |ui| {
                        let is_collapsed = collapsed_states[index];
                        let is_selected = Some(index) == *selected_action;
                        
                        let frame_stroke = if is_selected {
                            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255))
                        } else {
                            egui::Stroke::new(1.0, egui::Color32::from_gray(80))
                        };

                        let inner_response = egui::Frame::default()
                            .stroke(frame_stroke)
                            .corner_radius(6.0)
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("☰");
                                    if ui.button(if is_collapsed { "+" } else { "-" }).clicked() { collapsed_states[index] = !is_collapsed; }
                                    match &chapter.actions[index] {
                                        Action::SetBackground { .. } => ui.label("🌄 Hintergrund"),
                                        Action::ShowCharacter { .. } => ui.label("👤 Charakter"),
                                        Action::HideCharacter { .. } => ui.label("👻 Charakter verbergen"),
                                        Action::Dialogue { .. } => ui.label("💬 Dialog"),
                                        Action::PlayMusic { .. } => ui.label("🎵 Musik"),
                                        Action::PlaySound { .. } => ui.label("🔊 Soundeffekt"),
                                        Action::Label { .. } => ui.label("🏷 Label"),
                                        Action::Jump { .. } => ui.label("↪ Jump"),
                                        Action::Choice { .. } => ui.label("❓ Entscheidung"),
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
                                            ui.horizontal(|ui| { ui.label("ID:"); ui.text_edit_singleline(character_id); });
                                            Self::path_edit_with_browser(ui, "Pfad:", image_path, "assets/pictures/characters");
                                        }
                                        Action::HideCharacter { character_id } => {
                                            ui.horizontal(|ui| { ui.label("ID:"); ui.text_edit_singleline(character_id); });
                                        }
                                        Action::Label { name } => {
                                            ui.horizontal(|ui| { ui.label("Name:"); ui.text_edit_singleline(name); });
                                        }
                                        Action::Jump { target_label } => {
                                            ui.horizontal(|ui| { ui.label("Zu Label springen:"); ui.text_edit_singleline(target_label); });
                                        }
                                        Action::Choice { question, options } => {
                                            ui.horizontal(|ui| { ui.label("Frage:"); ui.text_edit_singleline(question); });
                                            ui.separator();
                                            let mut to_remove = None;
                                            for (idx, opt) in options.iter_mut().enumerate() {
                                                ui.horizontal(|ui| {
                                                    ui.label("Antwort:"); ui.text_edit_singleline(&mut opt.text);
                                                    ui.label("➡ Label:"); ui.text_edit_singleline(&mut opt.target_label);
                                                    if ui.button("❌").clicked() { to_remove = Some(idx); }
                                                });
                                            }
                                            if let Some(idx) = to_remove { options.remove(idx); }
                                            if ui.button("➕ Antwort hinzufügen").clicked() {
                                                options.push(ChoiceOption { text: String::new(), target_label: String::new() });
                                            }
                                        }
                                        Action::Dialogue { speaker_name, text, audio_path } => {
                                            Self::optional_text_edit(ui, "Sprecher:", speaker_name);
                                            let screen_height = ui.ctx().content_rect().height();
                                            let dialog_height = (screen_height * 0.10).max(60.0);
                                            ui.add_sized(egui::vec2(ui.available_width(), dialog_height), egui::TextEdit::multiline(text));
                                            Self::optional_audio_path_edit_with_browser(ui, "Voice:", audio_path, "assets/audio/voices", _audio_stream, audio_player, audio_durations);
                                        }
                                        Action::PlayMusic { audio_path } => {
                                            Self::audio_path_edit_with_browser(ui, "Musik-Pfad:", audio_path, "assets/audio/music", _audio_stream, audio_player, audio_durations);
                                        }
                                        Action::PlaySound { audio_path } => {
                                            Self::audio_path_edit_with_browser(ui, "SFX-Pfad:", audio_path, "assets/audio/sfx", _audio_stream, audio_player, audio_durations);
                                        }
                                    }
                                }
                            });
                        
                        if ui.interact(inner_response.response.rect, ui.id().with("select_click"), egui::Sense::click()).clicked() {
                            *selected_action = Some(index);
                        }
                        
                        ui.add_space(5.0);
                    });
                }
            });

            if let Some((i, j)) = action_to_swap { 
                chapter.actions.swap(i, j); 
                collapsed_states.swap(i, j); 
                if Some(i) == *selected_action { *selected_action = Some(j); }
                else if Some(j) == *selected_action { *selected_action = Some(i); }
            }
            if let Some(idx) = action_to_delete { 
                chapter.actions.remove(idx); 
                collapsed_states.remove(idx); 
                if Some(idx) == *selected_action { *selected_action = None; }
                else if let Some(sel) = selected_action {
                    if *sel > idx { *sel -= 1; }
                }
            }
        });
    }
}
