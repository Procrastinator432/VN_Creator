use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct KeyShortcut {
    pub key: Option<egui::Key>,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyShortcut {
    pub fn new(key: egui::Key) -> Self {
        Self { key: Some(key), ctrl: false, shift: false, alt: false }
    }
    
    pub fn with_modifiers(key: egui::Key, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self { key: Some(key), ctrl, shift, alt }
    }
    
    pub fn is_pressed(&self, ui: &egui::Ui) -> bool {
        if let Some(k) = self.key {
            ui.ctx().input(|i| {
                i.key_pressed(k) &&
                i.modifiers.ctrl == self.ctrl &&
                i.modifiers.shift == self.shift &&
                i.modifiers.alt == self.alt
            })
        } else {
            false
        }
    }
    
    pub fn display_name(&self) -> String {
        if let Some(k) = self.key {
            let mut parts = vec![];
            if self.ctrl { parts.push("Strg"); }
            if self.shift { parts.push("Shift"); }
            if self.alt { parts.push("Alt"); }
            
            let key_name = match k {
                egui::Key::Space => "Leertaste",
                egui::Key::ArrowRight => "Pfeil Rechts",
                egui::Key::ArrowLeft => "Pfeil Links",
                egui::Key::ArrowUp => "Pfeil Hoch",
                egui::Key::ArrowDown => "Pfeil Runter",
                egui::Key::Escape => "Esc",
                egui::Key::Enter => "Enter",
                egui::Key::Delete => "Entf",
                _ => {
                    let prefix = if parts.is_empty() { String::new() } else { parts.join(" + ") + " + " };
                    return format!("{}{:?}", prefix, k);
                }
            };
            
            parts.push(key_name);
            parts.join(" + ")
        } else {
            "Nicht belegt".to_string()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keybindings {
    pub global_menu: KeyShortcut,
    pub global_back: KeyShortcut,
    
    pub player_advance: KeyShortcut,
    pub player_skip: KeyShortcut,
    pub player_log: KeyShortcut,
    
    pub main_start: KeyShortcut,
    pub main_editor: KeyShortcut,
    pub main_settings: KeyShortcut,
    pub main_help: KeyShortcut,
    
    pub editor_add_bg: KeyShortcut,
    pub editor_add_char: KeyShortcut,
    pub editor_add_hide_char: KeyShortcut,
    pub editor_add_dialogue: KeyShortcut,
    pub editor_add_music: KeyShortcut,
    pub editor_add_sfx: KeyShortcut,
    pub editor_add_label: KeyShortcut,
    pub editor_add_jump: KeyShortcut,
    pub editor_add_choice: KeyShortcut,
    pub editor_save: KeyShortcut,
    
    pub editor_select_up: KeyShortcut,
    pub editor_select_down: KeyShortcut,
    pub editor_move_up: KeyShortcut,
    pub editor_move_down: KeyShortcut,
    pub editor_delete_action: KeyShortcut,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            global_menu: KeyShortcut::new(egui::Key::Escape),
            global_back: KeyShortcut::new(egui::Key::Escape),
            
            player_advance: KeyShortcut::new(egui::Key::Space),
            player_skip: KeyShortcut::new(egui::Key::Space), // Jetzt standardmäßig Space
            player_log: KeyShortcut::new(egui::Key::L),
            
            main_start: KeyShortcut::default(),
            main_editor: KeyShortcut::default(),
            main_settings: KeyShortcut::default(),
            main_help: KeyShortcut::default(),
            
            editor_add_bg: KeyShortcut::default(),
            editor_add_char: KeyShortcut::default(),
            editor_add_hide_char: KeyShortcut::default(),
            editor_add_dialogue: KeyShortcut::default(),
            editor_add_music: KeyShortcut::default(),
            editor_add_sfx: KeyShortcut::default(),
            editor_add_label: KeyShortcut::default(),
            editor_add_jump: KeyShortcut::default(),
            editor_add_choice: KeyShortcut::default(),
            editor_save: KeyShortcut::new(egui::Key::S), // Beispiel-Shortcut
            
            editor_select_up: KeyShortcut::new(egui::Key::ArrowUp),
            editor_select_down: KeyShortcut::new(egui::Key::ArrowDown),
            editor_move_up: KeyShortcut::with_modifiers(egui::Key::ArrowUp, true, false, false),
            editor_move_down: KeyShortcut::with_modifiers(egui::Key::ArrowDown, true, false, false),
            editor_delete_action: KeyShortcut::new(egui::Key::Delete),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub volume_master: f32,
    pub volume_bgm: f32,
    pub volume_sfx: f32,
    pub volume_voice: f32,
    pub text_speed_multiplier: f32,
    
    #[serde(default)]
    pub keybindings: Keybindings,
    
    #[serde(skip)]
    pub active_tab: usize,
    
    #[serde(skip)]
    pub waiting_for_key: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { 
            volume_master: 1.0, volume_bgm: 1.0, volume_sfx: 1.0, volume_voice: 1.0, text_speed_multiplier: 1.0,
            keybindings: Keybindings::default(),
            active_tab: 0,
            waiting_for_key: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        if let Ok(data) = std::fs::read_to_string("settings.json") {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }
    
    pub fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("settings.json", data);
        }
    }
    
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.set_min_width(500.0);
        ui.heading("⚙ Einstellungen");
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            if ui.selectable_label(self.active_tab == 0, "Audio & Text").clicked() { self.active_tab = 0; }
            if ui.selectable_label(self.active_tab == 1, "Tastenbelegung").clicked() { self.active_tab = 1; }
        });
        ui.separator();
        
        if self.active_tab == 0 {
            if ui.add(egui::Slider::new(&mut self.volume_master, 0.0..=1.0).text("Master Lautstärke")).changed() { changed = true; }
            if ui.add(egui::Slider::new(&mut self.volume_bgm, 0.0..=1.0).text("Musik Lautstärke")).changed() { changed = true; }
            if ui.add(egui::Slider::new(&mut self.volume_sfx, 0.0..=1.0).text("SFX Lautstärke")).changed() { changed = true; }
            if ui.add(egui::Slider::new(&mut self.volume_voice, 0.0..=1.0).text("Voice Lautstärke")).changed() { changed = true; }
            if ui.add(egui::Slider::new(&mut self.text_speed_multiplier, 0.1..=3.0).text("Textgeschw.")).changed() { changed = true; }
        } else if self.active_tab == 1 {
            egui::ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                if let Some(target) = &self.waiting_for_key {
                    ui.label(egui::RichText::new("Drücke eine beliebige Taste (auch mit Strg, Shift oder Alt möglich)...").color(egui::Color32::RED).strong());
                    ui.label(egui::RichText::new("(ESC zum Abbrechen)").weak());
                    
                    if let Some((key, mods)) = ui.ctx().input(|i| {
                        i.events.iter().find_map(|e| {
                            if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                                Some((*key, *modifiers))
                            } else {
                                None
                            }
                        })
                    }) {
                        if key == egui::Key::Escape && !mods.ctrl && !mods.shift && !mods.alt {
                            self.waiting_for_key = None;
                        } else {
                            let mut shortcut = KeyShortcut::new(key);
                            shortcut.ctrl = mods.ctrl || mods.mac_cmd || mods.command;
                            shortcut.shift = mods.shift;
                            shortcut.alt = mods.alt;
                            
                            let t = target.clone();
                            self.assign_key(&t, shortcut);
                            self.waiting_for_key = None;
                            changed = true;
                        }
                    }
                } else {
                    let binds = self.keybindings.clone();
                    
                    ui.heading("Globale Tasten");
                    self.draw_binding(ui, "Menü öffnen/schließen", "global_menu", binds.global_menu);
                    self.draw_binding(ui, "Hauptmenü / Zurück", "global_back", binds.global_back);
                    ui.separator();
                    
                    ui.heading("Spiel");
                    self.draw_binding(ui, "Text weiter", "player_advance", binds.player_advance);
                    self.draw_binding(ui, "Text überspringen", "player_skip", binds.player_skip);
                    self.draw_binding(ui, "Logbuch öffnen", "player_log", binds.player_log);
                    ui.separator();
                    
                    ui.heading("Hauptmenü");
                    self.draw_binding(ui, "Spiel starten", "main_start", binds.main_start);
                    self.draw_binding(ui, "Editor öffnen", "main_editor", binds.main_editor);
                    self.draw_binding(ui, "Einstellungen", "main_settings", binds.main_settings);
                    self.draw_binding(ui, "Hilfe", "main_help", binds.main_help);
                    ui.separator();
                    
                    ui.heading("Editor: Navigation");
                    self.draw_binding(ui, "Speichern", "editor_save", binds.editor_save);
                    self.draw_binding(ui, "Auswahl hoch", "editor_select_up", binds.editor_select_up);
                    self.draw_binding(ui, "Auswahl runter", "editor_select_down", binds.editor_select_down);
                    self.draw_binding(ui, "Aktion nach oben verschieben", "editor_move_up", binds.editor_move_up);
                    self.draw_binding(ui, "Aktion nach unten verschieben", "editor_move_down", binds.editor_move_down);
                    self.draw_binding(ui, "Aktion löschen", "editor_delete_action", binds.editor_delete_action);
                    ui.separator();
                    
                    ui.heading("Editor: Aktionen hinzufügen");
                    self.draw_binding(ui, "Hintergrund", "editor_add_bg", binds.editor_add_bg);
                    self.draw_binding(ui, "Charakter anzeigen", "editor_add_char", binds.editor_add_char);
                    self.draw_binding(ui, "Charakter ausblenden", "editor_add_hide_char", binds.editor_add_hide_char);
                    self.draw_binding(ui, "Dialog", "editor_add_dialogue", binds.editor_add_dialogue);
                    self.draw_binding(ui, "Musik", "editor_add_music", binds.editor_add_music);
                    self.draw_binding(ui, "SFX", "editor_add_sfx", binds.editor_add_sfx);
                    self.draw_binding(ui, "Label", "editor_add_label", binds.editor_add_label);
                    self.draw_binding(ui, "Jump", "editor_add_jump", binds.editor_add_jump);
                    self.draw_binding(ui, "Choice", "editor_add_choice", binds.editor_add_choice);
                }
            });
        }
        
        if changed { self.save(); }
        changed
    }

    fn draw_binding(&mut self, ui: &mut egui::Ui, label: &str, target_id: &str, current_shortcut: KeyShortcut) {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if current_shortcut.key.is_some()
                    && ui.button("❌").clicked() {
                        self.assign_key(target_id, KeyShortcut::default());
                    }
                if ui.button(current_shortcut.display_name()).clicked() {
                    self.waiting_for_key = Some(target_id.to_string());
                }
            });
        });
    }

    fn assign_key(&mut self, target_id: &str, shortcut: KeyShortcut) {
        match target_id {
            "global_menu" => self.keybindings.global_menu = shortcut,
            "global_back" => self.keybindings.global_back = shortcut,
            
            "player_advance" => self.keybindings.player_advance = shortcut,
            "player_skip" => self.keybindings.player_skip = shortcut,
            "player_log" => self.keybindings.player_log = shortcut,
            
            "main_start" => self.keybindings.main_start = shortcut,
            "main_editor" => self.keybindings.main_editor = shortcut,
            "main_settings" => self.keybindings.main_settings = shortcut,
            "main_help" => self.keybindings.main_help = shortcut,
            
            "editor_add_bg" => self.keybindings.editor_add_bg = shortcut,
            "editor_add_char" => self.keybindings.editor_add_char = shortcut,
            "editor_add_hide_char" => self.keybindings.editor_add_hide_char = shortcut,
            "editor_add_dialogue" => self.keybindings.editor_add_dialogue = shortcut,
            "editor_add_music" => self.keybindings.editor_add_music = shortcut,
            "editor_add_sfx" => self.keybindings.editor_add_sfx = shortcut,
            "editor_add_label" => self.keybindings.editor_add_label = shortcut,
            "editor_add_jump" => self.keybindings.editor_add_jump = shortcut,
            "editor_add_choice" => self.keybindings.editor_add_choice = shortcut,
            "editor_save" => self.keybindings.editor_save = shortcut,
            
            "editor_select_up" => self.keybindings.editor_select_up = shortcut,
            "editor_select_down" => self.keybindings.editor_select_down = shortcut,
            "editor_move_up" => self.keybindings.editor_move_up = shortcut,
            "editor_move_down" => self.keybindings.editor_move_down = shortcut,
            "editor_delete_action" => self.keybindings.editor_delete_action = shortcut,
            _ => {}
        }
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default_and_serialization() {
        let settings = Settings::default();
        assert_eq!(settings.volume_master, 1.0);
        assert_eq!(settings.volume_bgm, 1.0);
        assert_eq!(settings.volume_sfx, 1.0);
        assert_eq!(settings.volume_voice, 1.0);

        let json = serde_json::to_string(&settings).expect("Serialization failed");
        let deserialized: Settings = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(deserialized.volume_master, 1.0);
        assert_eq!(deserialized.keybindings.global_menu.key, Some(egui::Key::Escape));
    }

    #[test]
    fn test_key_shortcut_display_name() {
        let sc = KeyShortcut::new(egui::Key::Space);
        assert_eq!(sc.display_name(), "Leertaste");

        let sc_ctrl = KeyShortcut::with_modifiers(egui::Key::S, true, false, false);
        assert!(sc_ctrl.display_name().contains("Strg"));
        assert!(sc_ctrl.display_name().contains("S"));

        let empty = KeyShortcut::default();
        assert_eq!(empty.display_name(), "Nicht belegt");
    }
}
