use eframe::egui;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use crate::models::{Chapter, Action, ActiveCharacter, ChoiceOption, SaveState};
use crate::settings::Settings;

pub struct VnPlayer {
    chapter: Chapter,
    current_index: usize,

    current_background: Option<String>,
    current_bg_aspect: f32,

    active_characters: Vec<ActiveCharacter>,

    current_speaker: Option<String>,
    current_text: String,
    
    text_elapsed_time: f32,
    text_target_duration: f32,

    is_loading: bool,
    preload_uris: Vec<String>,

    is_finished: bool,

    history: Vec<(Option<String>, String)>,
    show_history: bool,
    
    waiting_for_choice: Option<(String, Vec<ChoiceOption>)>,

    _audio_stream: MixerDeviceSink,
    audio_player: Player,
    bgm_player: Player,
    sfx_player: Player,
    
    current_bgm: Option<String>,
    pub settings: Settings,
    pub is_in_menu: bool,
    
    bg_fade: f32,
    char_fade: f32,
}

impl VnPlayer {
    pub fn new(chapter: Chapter) -> Self {
        let stream_handle = DeviceSinkBuilder::open_default_sink()
            .expect("Konnte kein Audio-Gerät finden!");

        let voice_player = Player::connect_new(stream_handle.mixer());
        let bgm_player = Player::connect_new(stream_handle.mixer());
        let sfx_player = Player::connect_new(stream_handle.mixer());

        let mut preload_uris = Vec::new();
        if let Some(theme) = &chapter.theme {
            if let Some(frame_path) = &theme.character_frame_image {
                let uri = format!("file://{}", frame_path);
                if !preload_uris.contains(&uri) { preload_uris.push(uri); }
            }
        }
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
            text_elapsed_time: 0.0,
            text_target_duration: 0.0,
            is_loading: true,
            preload_uris,
            is_finished: false,
            history: Vec::new(),
            show_history: false,
            waiting_for_choice: None,
            _audio_stream: stream_handle,
            audio_player: voice_player,
            bgm_player,
            sfx_player,
            current_bgm: None,
            settings: Settings::load(),
            is_in_menu: false,
            bg_fade: 1.0,
            char_fade: 1.0,
        };

        vn_player.apply_volumes();
        vn_player.process_actions_until_dialogue();
        vn_player
    }

    pub fn from_save(save: SaveState) -> Self {
        let mut player = Self::new(save.chapter);
        player.current_index = save.current_index;
        player.current_background = save.current_background;
        player.current_bg_aspect = save.current_bg_aspect;
        player.active_characters = save.active_characters;
        player.current_speaker = save.current_speaker;
        player.current_text = save.current_text;
        player.history = save.history;
        player.current_bgm = save.current_bgm.clone();
        
        if let Some(bgm) = &player.current_bgm {
            if let Ok(file) = std::fs::File::open(bgm) {
                if let Ok(source) = Decoder::try_from(file) {
                    player.bgm_player.append(source.repeat_infinite());
                }
            }
        }
        
        player.is_loading = true;
        player
    }

    pub fn export_save(&self) -> SaveState {
        SaveState {
            chapter: self.chapter.clone(),
            current_index: self.current_index,
            current_background: self.current_background.clone(),
            current_bg_aspect: self.current_bg_aspect,
            active_characters: self.active_characters.clone(),
            current_speaker: self.current_speaker.clone(),
            current_text: self.current_text.clone(),
            history: self.history.clone(),
            current_bgm: self.current_bgm.clone(),
        }
    }

    fn apply_volumes(&self) {
        let master = self.settings.volume_master;
        self.bgm_player.set_volume(self.settings.volume_bgm * master);
        self.sfx_player.set_volume(self.settings.volume_sfx * master);
        self.audio_player.set_volume(self.settings.volume_voice * master);
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
                    self.bg_fade = 0.0;
                    self.current_index += 1;
                }
                Action::ShowCharacter { character_id, image_path } => {
                    self.active_characters.retain(|c| c.id != *character_id);
                    self.active_characters.push(ActiveCharacter {
                        id: character_id.clone(),
                        image_path: image_path.clone(),
                    });
                    self.char_fade = 0.0;
                    self.current_index += 1;
                }
                Action::HideCharacter { character_id } => {
                    self.active_characters.retain(|c| c.id != *character_id);
                    self.current_index += 1;
                }
                Action::PlayMusic { audio_path } => {
                    self.bgm_player = Player::connect_new(self._audio_stream.mixer());
                    self.apply_volumes();
                    if !audio_path.trim().is_empty() {
                        self.current_bgm = Some(audio_path.clone());
                        if let Ok(file) = std::fs::File::open(audio_path) {
                            if let Ok(source) = Decoder::try_from(file) {
                                self.bgm_player.append(source.repeat_infinite());
                            }
                        }
                    } else {
                        self.current_bgm = None;
                    }
                    self.current_index += 1;
                }
                Action::PlaySound { audio_path } => {
                    self.sfx_player = Player::connect_new(self._audio_stream.mixer());
                    self.apply_volumes();
                    if !audio_path.trim().is_empty() {
                        if let Ok(file) = std::fs::File::open(audio_path) {
                            if let Ok(source) = Decoder::try_from(file) {
                                self.sfx_player.append(source);
                            }
                        }
                    }
                    self.current_index += 1;
                }
                Action::Label { .. } => {
                    self.current_index += 1;
                }
                Action::Jump { target_label } => {
                    let mut found = false;
                    for (i, act) in self.chapter.actions.iter().enumerate() {
                        if let Action::Label { name } = act {
                            if name == target_label {
                                self.current_index = i;
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found { self.current_index += 1; }
                }
                Action::Choice { question, options } => {
                    self.waiting_for_choice = Some((question.clone(), options.clone()));
                    self.current_speaker = None;
                    self.current_text = question.clone();
                    self.text_elapsed_time = 0.0;
                    self.text_target_duration = 0.0;
                    self.history.push((None, question.clone()));
                    break;
                }
                Action::Dialogue { speaker_name, text, audio_path } => {
                    self.current_speaker = speaker_name.clone();
                    self.current_text = text.clone();
                    self.text_elapsed_time = 0.0;
                    self.history.push((speaker_name.clone(), text.clone()));
                    self.text_target_duration = text.len() as f32 * 0.035;

                    if let Some(path) = audio_path {
                        if let Ok(file) = std::fs::File::open(path) {
                            self.audio_player = Player::connect_new(self._audio_stream.mixer());
                            self.apply_volumes();
                            
                            if let Ok(file_for_dur) = std::fs::File::open(path) {
                                if let Ok(src) = Decoder::try_from(file_for_dur) {
                                    if let Some(duration) = src.total_duration() {
                                        let dur_secs = duration.as_secs_f32();
                                        if dur_secs >= 10.0 {
                                            self.text_target_duration = (dur_secs - 1.5_f32).max(0.1_f32);
                                        } else {
                                            self.text_target_duration = (dur_secs * 0.85_f32).max(0.1_f32);
                                        }
                                    }
                                }
                            }
                            if let Ok(source) = Decoder::try_from(file) {
                                self.audio_player.append(source);
                            }
                        }
                    }

                    self.text_target_duration /= self.settings.text_speed_multiplier;
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
        frame_image: Option<&str>,
        width: f32,
        height: f32,
        fade: f32,
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
                    let tint = egui::Color32::from_white_alpha((fade * 255.0) as u8);
                    let image = egui::Image::new(&uri).fit_to_exact_size(target_size).maintain_aspect_ratio(false).tint(tint);
                    image_ui.put(image_rect, image);
                } else {
                    let mut spinner_ui = image_ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                    spinner_ui.centered_and_justified(|ui| { ui.spinner(); });
                }
            });

            if let Some(frame_path) = frame_image {
                if !frame_path.trim().is_empty() {
                    let frame_uri = format!("file://{}", frame_path);
                    let tint = egui::Color32::from_white_alpha((fade * 255.0) as u8);
                    let overlay = egui::Image::new(&frame_uri)
                        .fit_to_exact_size(rect.size())
                        .maintain_aspect_ratio(false)
                        .tint(tint);
                    col_ui.put(rect, overlay);
                }
            }
        }
    }
}

impl VnPlayer {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let binds = self.settings.keybindings.clone();
        
        if self.is_in_menu {
            egui::Window::new("Pausenmenü")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        self.settings.ui(ui);
                        
                        ui.add_space(20.0);
                        if ui.button(egui::RichText::new("💾 Spielstand speichern").size(18.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("Save", &["json"]).set_file_name("save.json").save_file() {
                                let save = self.export_save();
                                if let Ok(json_str) = serde_json::to_string_pretty(&save) {
                                    let _ = std::fs::write(path, json_str);
                                }
                            }
                        }
                        if ui.button(egui::RichText::new("📂 Spielstand laden").size(18.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new().add_filter("Save", &["json"]).pick_file() {
                                if let Ok(s) = std::fs::read_to_string(path) {
                                    if let Ok(save) = serde_json::from_str::<SaveState>(&s) {
                                        *self = VnPlayer::from_save(save);
                                    }
                                }
                            }
                        }
                        ui.add_space(20.0);
                        if ui.button(egui::RichText::new("▶ Weiterlesen").size(20.0)).clicked() || binds.global_menu.is_pressed(ui) {
                            self.is_in_menu = false;
                        }
                    });
                });
            
            ui.ctx().request_repaint();
        }

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

        let dt = ui.input(|i| i.stable_dt);
        self.text_elapsed_time += dt;
        
        self.bg_fade = (self.bg_fade + dt * 2.0).min(1.0);
        self.char_fade = (self.char_fade + dt * 2.0).min(1.0);

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
                let tint = egui::Color32::from_white_alpha((self.bg_fade * 255.0) as u8);
                let image = egui::Image::new(&uri).fit_to_exact_size(target_size).maintain_aspect_ratio(false).tint(tint);
                ui.add(image);
            });
        }

        let log_rect = egui::Rect::from_min_size(full_rect.min + egui::vec2(20.0, 20.0), egui::vec2(140.0, 45.0));
        if ui.put(log_rect, egui::Button::new(egui::RichText::new("📜 Logbuch").size(18.0))).clicked() || binds.player_log.is_pressed(ui) {
            self.show_history = !self.show_history;
        }

        let menu_rect = egui::Rect::from_min_size(full_rect.min + egui::vec2(full_rect.width() - 160.0, 20.0), egui::vec2(140.0, 45.0));
        if ui.put(menu_rect, egui::Button::new(egui::RichText::new("⚙ Menü").size(18.0))).clicked() || (!self.is_in_menu && binds.global_menu.is_pressed(ui)) {
            self.is_in_menu = true;
            self.bgm_player.pause();
            self.sfx_player.pause();
            self.audio_player.pause();
        } else if !self.is_in_menu {
            self.bgm_player.play();
            self.sfx_player.play();
            self.audio_player.play();
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
        let mut char_frame_image = None;

        if let Some(theme) = &self.chapter.theme {
            if let Some(c) = theme.textbox_color { box_bg = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]); }
            if let Some(c) = theme.frame_color { frame_color = egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]); }
            if let Some(scf) = theme.show_character_frames { show_char_frames = scf; }
            if let Some(stf) = theme.show_textbox_frame { show_text_frame = stf; }
            if let Some(cfi) = &theme.character_frame_image { char_frame_image = Some(cfi.clone()); }
        }

        let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(textbox_rect));
        let mut skip_text = false;

        egui::Frame::default()
            .fill(box_bg)
            .stroke(if show_text_frame { egui::Stroke::new(2.0, frame_color) } else { egui::Stroke::NONE })
            .inner_margin(0.0)
            .show(&mut text_ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                let portrait_width = textbox_height * 0.8;
                let text_width = (textbox_rect.width() - (portrait_width * 2.0)).max(100.0);

                let portrait_frame = if char_frame_image.is_some() {
                    egui::Frame::default()
                        .fill(egui::Color32::from_black_alpha(80))
                        .inner_margin(8.0)
                } else if show_char_frames {
                    egui::Frame::default()
                        .fill(egui::Color32::from_black_alpha(100))
                        .stroke(egui::Stroke::new(3.0, frame_color))
                        .corner_radius(10.0)
                        .inner_margin(5.0)
                } else {
                    egui::Frame::default()
                };

                ui.horizontal(|ui| {
                    self.draw_portrait_column(
                        ui,
                        active_char,
                        portrait_frame.clone(),
                        char_frame_image.as_deref(),
                        portrait_width,
                        textbox_height,
                        self.char_fade,
                    );

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
                            
                            let mut visible_chars = self.current_text.len();
                            if self.text_target_duration > 0.0 && self.text_elapsed_time < self.text_target_duration {
                                let ratio = self.text_elapsed_time / self.text_target_duration;
                                visible_chars = (self.current_text.len() as f32 * ratio).floor() as usize;
                                ui.ctx().request_repaint();
                            }
                            
                            let visible_text: String = self.current_text.chars().take(visible_chars).collect();
                            ui.label(egui::RichText::new(&visible_text).size(20.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                                if self.waiting_for_choice.is_none() && !self.is_in_menu {
                                    
                                    let advance_pressed = binds.player_advance.is_pressed(ui);
                                    let skip_pressed = binds.player_skip.is_pressed(ui);

                                    if visible_chars < self.current_text.len() {
                                        if ui.button("⏭ Überspringen").clicked() || skip_pressed {
                                            skip_text = true;
                                        }
                                    } else {
                                        if ui.button("Weiter ➡").clicked() || advance_pressed {
                                            advance_chapter = true;
                                        }
                                    }
                                }
                            });
                        });
                    });

                    self.draw_portrait_column(
                        ui,
                        inactive_char,
                        portrait_frame,
                        char_frame_image.as_deref(),
                        portrait_width,
                        textbox_height,
                        self.char_fade,
                    );
                });
            });

        if skip_text {
            self.text_elapsed_time = self.text_target_duration;
        }

        if let Some((_, options)) = &self.waiting_for_choice {
            let mut choice_made = None;
            
            egui::Window::new("Entscheidung")
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, -50.0])
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE.inner_margin(20.0).show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            for opt in options {
                                ui.add_space(10.0);
                                if ui.add_sized([400.0, 50.0], egui::Button::new(egui::RichText::new(&opt.text).size(20.0))).clicked() {
                                    choice_made = Some(opt.target_label.clone());
                                }
                            }
                            ui.add_space(10.0);
                        });
                    });
                });

            if let Some(label) = choice_made {
                self.waiting_for_choice = None;
                let mut found = false;
                for (i, act) in self.chapter.actions.iter().enumerate() {
                    if let Action::Label { name } = act {
                        if name == &label {
                            self.current_index = i;
                            found = true;
                            break;
                        }
                    }
                }
                if !found { self.current_index += 1; }
                self.process_actions_until_dialogue();
            }
        }

        if self.show_history {
            egui::Window::new("Logbuch")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([800.0, 600.0])
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        for (speaker, text) in &self.history {
                            if let Some(s) = speaker {
                                ui.label(egui::RichText::new(s).color(egui::Color32::from_rgb(100, 200, 255)).strong().size(18.0));
                            } else {
                                ui.label(egui::RichText::new("Erzähler / Entscheidung").italics().color(egui::Color32::GRAY).size(18.0));
                            }
                            ui.label(egui::RichText::new(text).size(16.0));
                            ui.add_space(15.0);
                            ui.separator();
                        }
                    });
                    ui.add_space(10.0);
                    if ui.button("❌ Schließen").clicked() {
                        self.show_history = false;
                    }
                });
        }

        ui.ctx().request_repaint();

        if advance_chapter && !self.is_in_menu && self.waiting_for_choice.is_none() {
            self.next_action();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typewriter_duration_math() {
        let dur_secs = 12.0_f32;
        let target = (dur_secs - 1.5_f32).max(0.1_f32);
        assert_eq!(target, 10.5_f32);

        let dur_secs = 5.0_f32;
        let target = (dur_secs * 0.85_f32).max(0.1_f32);
        assert_eq!(target, 4.25_f32);
    }

    #[test]
    fn test_action_serialization_hide_char() {
        let action = Action::HideCharacter { character_id: "test_char".to_string() };
        let serialized = serde_json::to_string(&action).unwrap();
        assert!(serialized.contains("HideCharacter"));
        
        let deserialized: Action = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            Action::HideCharacter { character_id } => assert_eq!(character_id, "test_char"),
            _ => panic!("Wrong action type"),
        }
    }

    #[test]
    fn test_jump_resolution() {
        let chapter = Chapter {
            title: "Test".to_string(),
            theme: None,
            actions: vec![
                Action::Jump { target_label: "target_1".to_string() },
                Action::Dialogue { speaker_name: None, text: "Skip this".to_string(), audio_path: None },
                Action::Label { name: "target_1".to_string() },
                Action::Dialogue { speaker_name: None, text: "Reach this".to_string(), audio_path: None },
            ]
        };
        let player = VnPlayer::new(chapter);
        assert_eq!(player.current_text, "Reach this");
    }
}
