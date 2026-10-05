use crate::models::{Action, Chapter};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: String, // "error" | "warning" | "info"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_index: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterStats {
    pub total_actions: usize,
    pub dialogue_count: usize,
    pub total_words: usize,
    pub character_count: usize,
    pub characters: Vec<String>,
    pub choices_count: usize,
    pub labels_count: usize,
    pub jumps_count: usize,
    pub backgrounds_used: Vec<String>,
    pub music_used: Vec<String>,
    pub sfx_used: Vec<String>,
    pub voice_lines_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors_count: usize,
    pub warnings_count: usize,
    pub issues: Vec<ValidationIssue>,
    pub stats: ChapterStats,
}

pub fn validate_chapter_data(chapter: &Chapter, base_dir: Option<&Path>) -> ValidationResult {
    let mut issues = Vec::new();
    let mut defined_labels: HashMap<String, usize> = HashMap::new();
    let mut jump_targets: Vec<(String, usize)> = Vec::new();
    let mut choice_targets: Vec<(String, String, usize)> = Vec::new();
    let mut characters_set: HashSet<String> = HashSet::new();
    let mut active_characters: HashSet<String> = HashSet::new();
    let mut backgrounds: HashSet<String> = HashSet::new();
    let mut music_tracks: HashSet<String> = HashSet::new();
    let mut sfx_tracks: HashSet<String> = HashSet::new();
    let mut dialogue_count = 0;
    let mut voice_count = 0;
    let mut total_words = 0;
    let mut jumps_count = 0;
    let mut choices_count = 0;

    if chapter.title.trim().is_empty() {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            action_index: None,
            message: "Kapitel hat keinen Titel (title ist leer).".to_string(),
        });
    }

    if chapter.actions.is_empty() {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            action_index: None,
            message: "Kapitel enthält keine Aktionen (actions ist leer).".to_string(),
        });
    }

    if let Some(theme) = &chapter.theme
        && let Some(frame_img) = &theme.character_frame_image
            && !frame_img.trim().is_empty() {
                check_asset_exists(frame_img, 0, "Charakter-Rahmen (Theme)", base_dir, &mut issues);
            }

    for (idx, action) in chapter.actions.iter().enumerate() {
        match action {
            Action::Label { name } => {
                let trimmed = name.trim();
                if trimmed.is_empty() {
                    issues.push(ValidationIssue {
                        severity: "error".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: Label hat einen leeren Namen.", idx),
                    });
                } else if let Some(prev_idx) = defined_labels.get(trimmed) {
                    issues.push(ValidationIssue {
                        severity: "error".to_string(),
                        action_index: Some(idx),
                        message: format!(
                            "Aktion #{}: Doppeltes Label '{}' (bereits bei Aktion #{} definiert).",
                            idx, trimmed, prev_idx
                        ),
                    });
                } else {
                    defined_labels.insert(trimmed.to_string(), idx);
                }
            }
            Action::Jump { target_label } => {
                jumps_count += 1;
                jump_targets.push((target_label.clone(), idx));

                // Warning if immediately followed by an action other than Label
                if idx + 1 < chapter.actions.len()
                    && !matches!(chapter.actions[idx + 1], Action::Label { .. }) {
                        issues.push(ValidationIssue {
                            severity: "info".to_string(),
                            action_index: Some(idx + 1),
                            message: format!(
                                "Aktion #{}: Folgt direkt auf einen unbedingten Jump (Aktion #{}) ohne Label dazwischen. Kann im linearen Ablauf übersprungen werden.",
                                idx + 1, idx
                            ),
                        });
                    }
            }
            Action::Choice { question, options } => {
                choices_count += 1;
                if question.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: Choice-Frage ist leer.", idx),
                    });
                }
                if options.is_empty() {
                    issues.push(ValidationIssue {
                        severity: "error".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: Choice enthält keine Optionen.", idx),
                    });
                }
                for opt in options {
                    choice_targets.push((question.clone(), opt.target_label.clone(), idx));
                }
            }
            Action::Dialogue { speaker_name, text, audio_path } => {
                dialogue_count += 1;
                let word_count = text.split_whitespace().count();
                total_words += word_count;
                if text.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: Dialogtext ist leer.", idx),
                    });
                }
                if let Some(speaker) = speaker_name
                    && !speaker.trim().is_empty() {
                        characters_set.insert(speaker.clone());
                    }
                if let Some(audio) = audio_path
                    && !audio.trim().is_empty() {
                        voice_count += 1;
                        check_asset_exists(audio, idx, "Voice-Audio", base_dir, &mut issues);
                    }
            }
            Action::SetBackground { image_path } => {
                if image_path.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: SetBackground Bildpfad ist leer.", idx),
                    });
                } else {
                    backgrounds.insert(image_path.clone());
                    check_asset_exists(image_path, idx, "Hintergrundbild", base_dir, &mut issues);
                }
            }
            Action::ShowCharacter { character_id, image_path } => {
                if character_id.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: ShowCharacter character_id ist leer.", idx),
                    });
                } else {
                    characters_set.insert(character_id.clone());
                    active_characters.insert(character_id.clone());
                }
                if image_path.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: ShowCharacter Bildpfad ist leer.", idx),
                    });
                } else {
                    check_asset_exists(image_path, idx, "Charakter-Sprite", base_dir, &mut issues);
                }
            }
            Action::HideCharacter { character_id } => {
                if character_id.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: HideCharacter character_id ist leer.", idx),
                    });
                } else if !active_characters.remove(character_id) {
                    issues.push(ValidationIssue {
                        severity: "info".to_string(),
                        action_index: Some(idx),
                        message: format!(
                            "Aktion #{}: HideCharacter für '{}', aber Charakter wurde vorher nicht eingeblendet.",
                            idx, character_id
                        ),
                    });
                }
            }
            Action::PlayMusic { audio_path } => {
                if audio_path.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: PlayMusic Audiopfad ist leer.", idx),
                    });
                } else {
                    music_tracks.insert(audio_path.clone());
                    check_asset_exists(audio_path, idx, "Musik", base_dir, &mut issues);
                }
            }
            Action::PlaySound { audio_path } => {
                if audio_path.trim().is_empty() {
                    issues.push(ValidationIssue {
                        severity: "warning".to_string(),
                        action_index: Some(idx),
                        message: format!("Aktion #{}: PlaySound Audiopfad ist leer.", idx),
                    });
                } else {
                    sfx_tracks.insert(audio_path.clone());
                    check_asset_exists(audio_path, idx, "SFX", base_dir, &mut issues);
                }
            }
        }
    }

    let mut referenced_labels: HashSet<String> = HashSet::new();

    // Check Jump targets
    for (target, idx) in jump_targets {
        if !defined_labels.contains_key(&target) {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                action_index: Some(idx),
                message: format!(
                    "Aktion #{}: Jump verweist auf nicht existierendes Label '{}'.",
                    idx, target
                ),
            });
        } else {
            referenced_labels.insert(target);
        }
    }

    // Check Choice targets
    for (_q, target, idx) in choice_targets {
        if !defined_labels.contains_key(&target) {
            issues.push(ValidationIssue {
                severity: "error".to_string(),
                action_index: Some(idx),
                message: format!(
                    "Aktion #{}: Choice-Option verweist auf nicht existierendes Label '{}'.",
                    idx, target
                ),
            });
        } else {
            referenced_labels.insert(target);
        }
    }

    // Check for unused labels
    for (label, idx) in &defined_labels {
        if !referenced_labels.contains(label) {
            issues.push(ValidationIssue {
                severity: "warning".to_string(),
                action_index: Some(*idx),
                message: format!(
                    "Aktion #{}: Label '{}' wird nirgendwo durch Jump oder Choice angesprungen.",
                    idx, label
                ),
            });
        }
    }

    let errors_count = issues.iter().filter(|i| i.severity == "error").count();
    let warnings_count = issues.iter().filter(|i| i.severity == "warning").count();

    let mut characters_vec: Vec<String> = characters_set.into_iter().collect();
    characters_vec.sort();

    let mut backgrounds_vec: Vec<String> = backgrounds.into_iter().collect();
    backgrounds_vec.sort();

    let mut music_vec: Vec<String> = music_tracks.into_iter().collect();
    music_vec.sort();

    let mut sfx_vec: Vec<String> = sfx_tracks.into_iter().collect();
    sfx_vec.sort();

    ValidationResult {
        valid: errors_count == 0,
        errors_count,
        warnings_count,
        issues,
        stats: ChapterStats {
            total_actions: chapter.actions.len(),
            dialogue_count,
            total_words,
            character_count: characters_vec.len(),
            characters: characters_vec,
            choices_count,
            labels_count: defined_labels.len(),
            jumps_count,
            backgrounds_used: backgrounds_vec,
            music_used: music_vec,
            sfx_used: sfx_vec,
            voice_lines_count: voice_count,
        },
    }
}

fn check_asset_exists(
    asset_path: &str,
    idx: usize,
    asset_kind: &str,
    base_dir: Option<&Path>,
    issues: &mut Vec<ValidationIssue>,
) {
    let p = Path::new(asset_path);
    let mut exists = p.exists();

    if !exists
        && let Some(base) = base_dir {
            exists = base.join(p).exists();
        }

    if !exists {
        issues.push(ValidationIssue {
            severity: "warning".to_string(),
            action_index: Some(idx),
            message: format!(
                "Aktion #{}: {} Datei '{}' wurde auf der Festplatte nicht gefunden.",
                idx, asset_kind, asset_path
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Action, Chapter, ChoiceOption, Theme};

    #[test]
    fn test_valid_chapter() {
        let chapter = Chapter {
            title: "Test Kapitel".to_string(),
            theme: None,
            actions: vec![
                Action::Label { name: "start".to_string() },
                Action::Dialogue {
                    speaker_name: Some("Alice".to_string()),
                    text: "Hallo Welt!".to_string(),
                    audio_path: None,
                },
                Action::Jump { target_label: "start".to_string() },
            ],
        };

        let result = validate_chapter_data(&chapter, None);
        assert!(result.valid);
        assert_eq!(result.errors_count, 0);
        assert_eq!(result.stats.dialogue_count, 1);
        assert_eq!(result.stats.characters, vec!["Alice"]);
    }

    #[test]
    fn test_broken_jump_target() {
        let chapter = Chapter {
            title: "Broken Jump".to_string(),
            theme: None,
            actions: vec![
                Action::Jump { target_label: "non_existent_label".to_string() },
            ],
        };

        let result = validate_chapter_data(&chapter, None);
        assert!(!result.valid);
        assert_eq!(result.errors_count, 1);
        assert!(result.issues[0].message.contains("nicht existierendes Label 'non_existent_label'"));
    }

    #[test]
    fn test_duplicate_labels() {
        let chapter = Chapter {
            title: "Duplicate Labels".to_string(),
            theme: None,
            actions: vec![
                Action::Label { name: "scene1".to_string() },
                Action::Label { name: "scene1".to_string() },
                Action::Jump { target_label: "scene1".to_string() },
            ],
        };

        let result = validate_chapter_data(&chapter, None);
        assert!(!result.valid);
        assert_eq!(result.errors_count, 1);
        assert!(result.issues.iter().any(|i| i.message.contains("Doppeltes Label 'scene1'")));
    }

    #[test]
    fn test_broken_choice_target() {
        let chapter = Chapter {
            title: "Choice Test".to_string(),
            theme: None,
            actions: vec![
                Action::Choice {
                    question: "Was tust du?".to_string(),
                    options: vec![
                        ChoiceOption {
                            text: "Nach links gehen".to_string(),
                            target_label: "left".to_string(),
                        },
                    ],
                },
            ],
        };

        let result = validate_chapter_data(&chapter, None);
        assert!(!result.valid);
        assert!(result.issues.iter().any(|i| i.severity == "error" && i.message.contains("nicht existierendes Label 'left'")));
    }

    #[test]
    fn test_character_frame_image_warning() {
        let chapter = Chapter {
            title: "Frame Test".to_string(),
            theme: Some(Theme {
                textbox_color: None,
                frame_color: None,
                show_character_frames: None,
                show_textbox_frame: None,
                character_frame_image: Some("assets/pictures/frames/missing_frame.png".to_string()),
            }),
            actions: vec![
                Action::Dialogue {
                    speaker_name: None,
                    text: "Hallo!".to_string(),
                    audio_path: None,
                },
            ],
        };

        let result = validate_chapter_data(&chapter, None);
        assert!(result.valid); // Missing asset is a warning, not a hard error
        assert_eq!(result.warnings_count, 1);
        assert!(result.issues[0].message.contains("Charakter-Rahmen (Theme)"));
    }
}
