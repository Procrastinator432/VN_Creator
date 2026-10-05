use crate::mcp::protocol::{CallToolResult, ToolDefinition};
use crate::mcp::validation::validate_chapter_data;
use crate::models::{Action, Chapter, Theme};
use crate::settings::Settings;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_chapters".to_string(),
            description: "Durchsucht das Arbeitsverzeichnis nach Visual Novel Kapiteln (.json) und gibt Metadaten (Titel, Aktionen-Anzahl, Charaktere, Verzweigungen) zurück.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "directory": {
                        "type": "string",
                        "description": "Optionales Verzeichnis, in dem nach Kapiteln gesucht wird. Standard ist das aktuelle Projektverzeichnis."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "read_chapter".to_string(),
            description: "Liest ein Visual Novel Kapitel ein und gibt die vollständige JSON-Struktur sowie eine strukturierte Übersicht (Szenen, Charaktere, Dialoge, Entscheidungen) zurück.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relativer oder absoluter Pfad zur Kapitel-JSON-Datei (z.B. 'test_chapter.json')."
                    },
                    "outline_only": {
                        "type": "boolean",
                        "description": "Wenn true, wird nur eine komprimierte Szenen-Übersicht statt der vollständigen JSON-Aktionen ausgegeben."
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "create_chapter".to_string(),
            description: "Erstellt ein neues Visual Novel Kapitel und speichert es als formatierte JSON-Datei.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Zieldateipfad für das neue Kapitel (z.B. 'kapitel_2.json')."
                    },
                    "title": {
                        "type": "string",
                        "description": "Titel des Kapitels."
                    },
                    "theme": {
                        "type": "object",
                        "description": "Optionales Farb- und Rahmentheme.",
                        "properties": {
                            "textbox_color": { "type": "array", "items": { "type": "integer" }, "description": "[r, g, b, a]" },
                            "frame_color": { "type": "array", "items": { "type": "integer" }, "description": "[r, g, b, a]" },
                            "show_character_frames": { "type": "boolean" },
                            "show_textbox_frame": { "type": "boolean" },
                            "character_frame_image": { "type": "string", "description": "Optionaler Bildpfad für Rahmen der Charaktere (z.B. 'assets/pictures/frames/frame.png')" }
                        }
                    },
                    "actions": {
                        "type": "array",
                        "description": "Optionale initiale Liste von Aktionen (Dialogue, SetBackground, ShowCharacter, Choice, etc.)."
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "Falls true, wird eine existierende Datei überschrieben. Standard: false."
                    }
                },
                "required": ["path", "title"]
            }),
        },
        ToolDefinition {
            name: "save_chapter".to_string(),
            description: "Speichert oder überschreibt ein komplettes Visual Novel Kapitel als formatierte JSON-Datei.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Dateipfad zur Kapitel-JSON-Datei."
                    },
                    "chapter": {
                        "type": "object",
                        "description": "Das vollständige Chapter-Objekt mit title, theme und actions.",
                        "required": ["title", "actions"],
                        "properties": {
                            "title": { "type": "string" },
                            "theme": { "type": "object" },
                            "actions": { "type": "array" }
                        }
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "Ob eine existierende Datei überschrieben werden darf. Standard: true."
                    }
                },
                "required": ["path", "chapter"]
            }),
        },
        ToolDefinition {
            name: "add_actions".to_string(),
            description: "Fügt einem existierenden Kapitel eine oder mehrere Aktionen (z.B. Dialoge, Choices, Labels, Musik) hinzu (am Ende angehängt oder an bestimmter Position eingefügt).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Pfad zur Kapitel-JSON-Datei."
                    },
                    "actions": {
                        "type": "array",
                        "description": "Liste der hinzuzufügenden Aktionen (SetBackground, ShowCharacter, HideCharacter, Dialogue, PlayMusic, PlaySound, Label, Jump, Choice)."
                    },
                    "insert_at": {
                        "type": "integer",
                        "description": "Optionaler 0-basierter Index. Wenn weggelassen, werden die Aktionen ans Ende angehängt."
                    }
                },
                "required": ["path", "actions"]
            }),
        },
        ToolDefinition {
            name: "validate_chapter".to_string(),
            description: "Validiert ein Kapitel auf logische Fehler (nicht existierende Sprungziele in Jump/Choice, doppelte oder unbenutzte Labels, fehlende Mediendateien, leere Texte) und liefert Statistiken.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optionaler Pfad zur Kapitel-JSON-Datei, die von der Festplatte gelesen werden soll."
                    },
                    "chapter": {
                        "type": "object",
                        "description": "Optionales inline Chapter-Objekt zur direkten Prüfung ohne Festplattenzugriff."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "list_assets".to_string(),
            description: "Listet alle verfügbaren Medien-Assets im Projektordner 'assets/' auf (Hintergründe, Charakter-Sprites, Musik, SFX, Stimmen).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "enum": ["all", "backgrounds", "characters", "frames", "music", "voices", "sfx"],
                        "description": "Filter für Asset-Kategorien. Standard ist 'all'."
                    },
                    "base_dir": {
                        "type": "string",
                        "description": "Basisverzeichnis. Standard: Projekt-Arbeitsverzeichnis."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "launch_player".to_string(),
            description: "Startet die VN_Creator Anwendung im Player-Modus mit dem angegebenen Kapitel in einem separaten Fenster, damit der Nutzer die Geschichte direkt testen kann.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Pfad zur Kapitel-JSON-Datei, die abgespielt werden soll."
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "launch_editor".to_string(),
            description: "Startet die VN_Creator Anwendung im Editor-Modus (optional mit bereits geladenem Kapitel) in einem separaten Fenster.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optionaler Pfad zur Kapitel-JSON-Datei, die im Editor geöffnet werden soll."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "get_settings".to_string(),
            description: "Gibt die aktuellen Einstellungen der App (Lautstärke, Tastenkürzel) zurück.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

#[derive(Serialize)]
struct ChapterSummaryItem {
    path: String,
    title: String,
    actions_count: usize,
    dialogue_count: usize,
    characters: Vec<String>,
    has_choices: bool,
    has_audio: bool,
}

pub fn handle_tool_call(name: &str, arguments: &Value) -> CallToolResult {
    match name {
        "list_chapters" => tool_list_chapters(arguments),
        "read_chapter" => tool_read_chapter(arguments),
        "create_chapter" => tool_create_chapter(arguments),
        "save_chapter" => tool_save_chapter(arguments),
        "add_actions" => tool_add_actions(arguments),
        "validate_chapter" => tool_validate_chapter(arguments),
        "list_assets" => tool_list_assets(arguments),
        "launch_player" => tool_launch_player(arguments),
        "launch_editor" => tool_launch_editor(arguments),
        "get_settings" => tool_get_settings(arguments),
        unknown => CallToolResult::error(format!("Unbekanntes Tool: '{}'", unknown)),
    }
}

fn tool_list_chapters(args: &Value) -> CallToolResult {
    let dir_str = args.get("directory").and_then(|v| v.as_str()).unwrap_or(".");
    let target_dir = Path::new(dir_str);

    if !target_dir.exists() {
        return CallToolResult::error(format!("Verzeichnis nicht gefunden: '{}'", dir_str));
    }

    let mut found_chapters = Vec::new();
    scan_for_chapters(target_dir, &mut found_chapters);

    CallToolResult::json(&json!({
        "directory": dir_str,
        "count": found_chapters.len(),
        "chapters": found_chapters,
    }))
}

fn scan_for_chapters(dir: &Path, list: &mut Vec<ChapterSummaryItem>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "assets" {
                continue;
            }
            scan_for_chapters(&path, list);
        } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "settings.json" {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(chapter) = serde_json::from_str::<Chapter>(&content) {
                    let mut characters = Vec::new();
                    let mut dialogue_count = 0;
                    let mut has_choices = false;
                    let mut has_audio = false;

                    for act in &chapter.actions {
                        match act {
                            Action::Dialogue { speaker_name, audio_path, .. } => {
                                dialogue_count += 1;
                                if let Some(spk) = speaker_name {
                                    if !spk.is_empty() && !characters.contains(spk) {
                                        characters.push(spk.clone());
                                    }
                                }
                                if audio_path.is_some() {
                                    has_audio = true;
                                }
                            }
                            Action::ShowCharacter { character_id, .. } => {
                                if !characters.contains(character_id) {
                                    characters.push(character_id.clone());
                                }
                            }
                            Action::Choice { .. } => has_choices = true,
                            Action::PlayMusic { .. } | Action::PlaySound { .. } => has_audio = true,
                            _ => {}
                        }
                    }

                    list.push(ChapterSummaryItem {
                        path: path.to_string_lossy().replace('\\', "/"),
                        title: chapter.title,
                        actions_count: chapter.actions.len(),
                        dialogue_count,
                        characters,
                        has_choices,
                        has_audio,
                    });
                }
            }
        }
    }
}

fn tool_read_chapter(args: &Value) -> CallToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return CallToolResult::error("Fehlender Parameter: 'path'"),
    };

    let p = Path::new(path_str);
    if !p.exists() {
        return CallToolResult::error(format!("Kapiteldatei nicht gefunden: '{}'", path_str));
    }

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Konnte Datei '{}' nicht lesen: {}", path_str, e)),
    };

    let chapter: Chapter = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Fehler beim Parsen von '{}': {}", path_str, e)),
    };

    let outline_only = args.get("outline_only").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut outline = Vec::new();
    for (idx, action) in chapter.actions.iter().enumerate() {
        let desc = match action {
            Action::SetBackground { image_path } => format!("[#{}] Hintergrund setzen: {}", idx, image_path),
            Action::ShowCharacter { character_id, image_path } => {
                format!("[#{}] Charakter anzeigen: {} ({})", idx, character_id, image_path)
            }
            Action::HideCharacter { character_id } => format!("[#{}] Charakter verbergen: {}", idx, character_id),
            Action::Dialogue { speaker_name, text, .. } => {
                let speaker = speaker_name.as_deref().unwrap_or("Erzähler");
                let snippet: String = text.chars().take(60).collect();
                format!("[#{}] {}: \"{}\"", idx, speaker, snippet)
            }
            Action::PlayMusic { audio_path } => format!("[#{}] Musik abspielen: {}", idx, audio_path),
            Action::PlaySound { audio_path } => format!("[#{}] Sound abspielen: {}", idx, audio_path),
            Action::Label { name } => format!("[#{}] --- LABEL: {} ---", idx, name),
            Action::Jump { target_label } => format!("[#{}] Sprung zu Label: {}", idx, target_label),
            Action::Choice { question, options } => {
                let opts: Vec<String> = options.iter().map(|o| format!("'{}' -> {}", o.text, o.target_label)).collect();
                format!("[#{}] Auswahl: \"{}\" [{}]", idx, question, opts.join(", "))
            }
        };
        outline.push(desc);
    }

    if outline_only {
        CallToolResult::json(&json!({
            "title": chapter.title,
            "actions_count": chapter.actions.len(),
            "outline": outline
        }))
    } else {
        CallToolResult::json(&json!({
            "path": path_str,
            "chapter": chapter,
            "outline": outline
        }))
    }
}

fn tool_create_chapter(args: &Value) -> CallToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return CallToolResult::error("Fehlender Parameter: 'path'"),
    };
    let title = match args.get("title").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return CallToolResult::error("Fehlender Parameter: 'title'"),
    };

    let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(false);
    let target_path = Path::new(path_str);

    if target_path.exists() && !overwrite {
        return CallToolResult::error(format!(
            "Datei '{}' existiert bereits. Setze 'overwrite: true', um sie zu überschreiben.",
            path_str
        ));
    }

    let theme: Option<Theme> = args
        .get("theme")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let actions: Vec<Action> = args
        .get("actions")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let chapter = Chapter {
        title,
        theme,
        actions,
    };

    if let Some(parent) = target_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let json_text = match serde_json::to_string_pretty(&chapter) {
        Ok(t) => t,
        Err(e) => return CallToolResult::error(format!("Serialisierungsfehler: {}", e)),
    };

    match fs::write(target_path, json_text) {
        Ok(_) => CallToolResult::json(&json!({
            "success": true,
            "path": path_str,
            "title": chapter.title,
            "actions_count": chapter.actions.len(),
            "message": format!("Kapitel '{}' erfolgreich erstellt.", path_str)
        })),
        Err(e) => CallToolResult::error(format!("Konnte Datei '{}' nicht speichern: {}", path_str, e)),
    }
}

fn tool_save_chapter(args: &Value) -> CallToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return CallToolResult::error("Fehlender Parameter: 'path'"),
    };
    let chapter_val = match args.get("chapter") {
        Some(c) => c,
        None => return CallToolResult::error("Fehlender Parameter: 'chapter'"),
    };

    let chapter: Chapter = match serde_json::from_value(chapter_val.clone()) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Ungültiges Chapter-Format: {}", e)),
    };

    let target_path = Path::new(path_str);
    let overwrite = args.get("overwrite").and_then(|v| v.as_bool()).unwrap_or(true);

    if target_path.exists() && !overwrite {
        return CallToolResult::error(format!("Datei '{}' existiert bereits und overwrite ist false.", path_str));
    }

    if let Some(parent) = target_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    let json_text = match serde_json::to_string_pretty(&chapter) {
        Ok(t) => t,
        Err(e) => return CallToolResult::error(format!("Serialisierungsfehler: {}", e)),
    };

    match fs::write(target_path, json_text) {
        Ok(_) => CallToolResult::json(&json!({
            "success": true,
            "path": path_str,
            "title": chapter.title,
            "actions_count": chapter.actions.len(),
            "message": format!("Kapitel '{}' erfolgreich gespeichert.", path_str)
        })),
        Err(e) => CallToolResult::error(format!("Fehler beim Schreiben von '{}': {}", path_str, e)),
    }
}

fn tool_add_actions(args: &Value) -> CallToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return CallToolResult::error("Fehlender Parameter: 'path'"),
    };
    let actions_val = match args.get("actions") {
        Some(a) => a,
        None => return CallToolResult::error("Fehlender Parameter: 'actions'"),
    };

    let new_actions: Vec<Action> = match serde_json::from_value(actions_val.clone()) {
        Ok(a) => a,
        Err(e) => return CallToolResult::error(format!("Ungültiges Actions-Format: {}", e)),
    };

    let target_path = Path::new(path_str);
    if !target_path.exists() {
        return CallToolResult::error(format!("Kapiteldatei '{}' nicht gefunden.", path_str));
    }

    let content = match fs::read_to_string(target_path) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Fehler beim Lesen: {}", e)),
    };

    let mut chapter: Chapter = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => return CallToolResult::error(format!("Fehler beim Parsen: {}", e)),
    };

    let added_count = new_actions.len();
    if let Some(insert_at) = args.get("insert_at").and_then(|v| v.as_u64()).map(|u| u as usize) {
        let insert_idx = insert_at.min(chapter.actions.len());
        for (offset, act) in new_actions.into_iter().enumerate() {
            chapter.actions.insert(insert_idx + offset, act);
        }
    } else {
        chapter.actions.extend(new_actions);
    }

    let json_text = match serde_json::to_string_pretty(&chapter) {
        Ok(t) => t,
        Err(e) => return CallToolResult::error(format!("Serialisierungsfehler: {}", e)),
    };

    match fs::write(target_path, json_text) {
        Ok(_) => CallToolResult::json(&json!({
            "success": true,
            "path": path_str,
            "added_actions_count": added_count,
            "total_actions": chapter.actions.len(),
            "message": format!("{} Aktion(en) zu '{}' hinzugefügt.", added_count, path_str)
        })),
        Err(e) => CallToolResult::error(format!("Fehler beim Speichern: {}", e)),
    }
}

fn tool_validate_chapter(args: &Value) -> CallToolResult {
    let chapter = if let Some(path_str) = args.get("path").and_then(|v| v.as_str()) {
        let p = Path::new(path_str);
        if !p.exists() {
            return CallToolResult::error(format!("Kapiteldatei nicht gefunden: '{}'", path_str));
        }
        let content = match fs::read_to_string(p) {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("Konnte Datei nicht lesen: {}", e)),
        };
        match serde_json::from_str::<Chapter>(&content) {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("JSON Parse-Fehler in '{}': {}", path_str, e)),
        }
    } else if let Some(chapter_val) = args.get("chapter") {
        match serde_json::from_value::<Chapter>(chapter_val.clone()) {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("Ungültiges Chapter-Objekt: {}", e)),
        }
    } else {
        return CallToolResult::error("Entweder 'path' oder 'chapter' muss für die Validierung angegeben werden.");
    };

    let result = validate_chapter_data(&chapter, Some(Path::new(".")));
    CallToolResult::json(&result)
}

#[derive(Serialize)]
struct AssetInventory {
    backgrounds: Vec<String>,
    characters: Vec<String>,
    frames: Vec<String>,
    music: Vec<String>,
    voices: Vec<String>,
    sfx: Vec<String>,
    other: Vec<String>,
    total_count: usize,
}

fn tool_list_assets(args: &Value) -> CallToolResult {
    let base_dir_str = args.get("base_dir").and_then(|v| v.as_str()).unwrap_or(".");
    let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("all");

    let assets_dir = Path::new(base_dir_str).join("assets");
    if !assets_dir.exists() {
        return CallToolResult::json(&json!({
            "status": "warning",
            "message": "Ordner 'assets' existiert noch nicht im Arbeitsverzeichnis.",
            "assets": []
        }));
    }

    let mut inventory = AssetInventory {
        backgrounds: Vec::new(),
        characters: Vec::new(),
        frames: Vec::new(),
        music: Vec::new(),
        voices: Vec::new(),
        sfx: Vec::new(),
        other: Vec::new(),
        total_count: 0,
    };

    scan_asset_files(&assets_dir, &assets_dir, &mut inventory);

    match category {
        "backgrounds" => CallToolResult::json(&json!({ "backgrounds": inventory.backgrounds })),
        "characters" => CallToolResult::json(&json!({ "characters": inventory.characters })),
        "frames" => CallToolResult::json(&json!({ "frames": inventory.frames })),
        "music" => CallToolResult::json(&json!({ "music": inventory.music })),
        "voices" => CallToolResult::json(&json!({ "voices": inventory.voices })),
        "sfx" => CallToolResult::json(&json!({ "sfx": inventory.sfx })),
        _ => CallToolResult::json(&inventory),
    }
}

fn scan_asset_files(current: &Path, root: &Path, inv: &mut AssetInventory) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_asset_files(&path, root, inv);
        } else if path.is_file() {
            let rel = path.to_string_lossy().replace('\\', "/");
            let lower = rel.to_lowercase();
            inv.total_count += 1;

            if lower.contains("/frame") || lower.contains("/rahmen") {
                inv.frames.push(rel);
            } else if lower.contains("/background") || lower.contains("/hintergrund") {
                inv.backgrounds.push(rel);
            } else if lower.contains("/character") || lower.contains("/charakter") || lower.contains("/sprites") {
                inv.characters.push(rel);
            } else if lower.contains("/music") || lower.contains("/musik") || lower.contains("/bgm") {
                inv.music.push(rel);
            } else if lower.contains("/voice") || lower.contains("/stimme") {
                inv.voices.push(rel);
            } else if lower.contains("/sfx") || lower.contains("/sounds") {
                inv.sfx.push(rel);
            } else {
                inv.other.push(rel);
            }
        }
    }
}

fn tool_launch_player(args: &Value) -> CallToolResult {
    let path_str = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return CallToolResult::error("Fehlender Parameter: 'path'"),
    };

    let p = Path::new(path_str);
    if !p.exists() {
        return CallToolResult::error(format!("Kapiteldatei '{}' existiert nicht.", path_str));
    }

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return CallToolResult::error(format!("Konnte Pfad zur aktuellen Executable nicht ermitteln: {}", e)),
    };

    let mut cmd = Command::new(&exe);
    cmd.arg("--player").arg(path_str);

    match cmd.spawn() {
        Ok(child) => CallToolResult::json(&json!({
            "status": "launched",
            "mode": "player",
            "chapter_path": path_str,
            "pid": child.id(),
            "message": format!("VN_Creator Player wurde für '{}' gestartet (PID: {}).", path_str, child.id())
        })),
        Err(e) => CallToolResult::error(format!("Fehler beim Starten der App: {}", e)),
    }
}

fn tool_launch_editor(args: &Value) -> CallToolResult {
    let path_opt = args.get("path").and_then(|v| v.as_str());

    if let Some(path_str) = path_opt {
        let p = Path::new(path_str);
        if !p.exists() {
            return CallToolResult::error(format!("Kapiteldatei '{}' existiert nicht.", path_str));
        }
    }

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return CallToolResult::error(format!("Konnte Pfad zur aktuellen Executable nicht ermitteln: {}", e)),
    };

    let mut cmd = Command::new(&exe);
    cmd.arg("--editor");
    if let Some(p) = path_opt {
        cmd.arg(p);
    }

    match cmd.spawn() {
        Ok(child) => CallToolResult::json(&json!({
            "status": "launched",
            "mode": "editor",
            "chapter_path": path_opt,
            "pid": child.id(),
            "message": format!("VN_Creator Editor wurde erfolgreich gestartet (PID: {}).", child.id())
        })),
        Err(e) => CallToolResult::error(format!("Fehler beim Starten des Editors: {}", e)),
    }
}

fn tool_get_settings(_args: &Value) -> CallToolResult {
    let settings = Settings::load();
    CallToolResult::json(&settings)
}
