use serde::{Deserialize, Serialize};

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
    HideCharacter { character_id: String },
    Dialogue { speaker_name: Option<String>, text: String, audio_path: Option<String> },
    PlayMusic { audio_path: String },
    PlaySound { audio_path: String },
    Label { name: String },
    Jump { target_label: String },
    Choice { question: String, options: Vec<ChoiceOption> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChoiceOption {
    pub text: String,
    pub target_label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveCharacter {
    pub id: String,
    pub image_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveState {
    pub chapter: Chapter,
    pub current_index: usize,
    pub current_background: Option<String>,
    pub current_bg_aspect: f32,
    pub active_characters: Vec<ActiveCharacter>,
    pub current_speaker: Option<String>,
    pub current_text: String,
    pub history: Vec<(Option<String>, String)>,
    pub current_bgm: Option<String>,
}
