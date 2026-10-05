mod models;
mod settings;
mod player;
mod editor;
pub mod mcp;

use player::VnPlayer;
use editor::VnEditor;
use settings::Settings;
use models::Chapter;
use eframe::egui;

// --- HILFE SYSTEM ---
struct HelpChapter {
    title: &'static str,
    content: &'static str,
}

struct HelpState {
    search_query: String,
    selected_chapter: usize,
    chapters: Vec<HelpChapter>,
}

impl HelpState {
    fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_chapter: 0,
            chapters: vec![
                HelpChapter {
                    title: "1. Grundlagen (Spielen & Speichern)",
                    content: "Willkommen im Visual Novel Studio!\n\nUm ein Spiel zu starten, klicke im Hauptmenü auf 'Spiel laden & starten' und wähle eine JSON-Datei (wie z.B. 'test_chapter.json').\nWährend des Lesens kannst du oben rechts auf 'Menü' klicken, um das Spiel zu pausieren, Einstellungen zu ändern oder deinen aktuellen Fortschritt in einem neuen Savegame abzuspeichern. Das Logbuch oben links lässt dich vergangene Dialoge nachlesen."
                },
                HelpChapter {
                    title: "2. Editor: Kapitel erstellen",
                    content: "Mit 'Neuen Editor öffnen' kannst du eigene Geschichten schreiben.\nDu kannst oben einen Titel festlegen und unter 'Kapitel-Design' die Farben für die Textboxen anpassen.\nFüge über die Buttons Aktionen (wie Dialoge oder Bilder) zu deiner Zeitleiste hinzu. Über die ⬆/⬇ Pfeile (oder Tastenkürzel Strg+Pfeiltasten) kannst du Aktionen sortieren. Vergiss nicht, regelmäßig oben links zu speichern!"
                },
                HelpChapter {
                    title: "3. Medien (Bilder & Audio)",
                    content: "Das Studio kopiert deine gewählten Dateien automatisch in den Ordner 'assets'.\n- Hintergrundbilder (16:9 empfohlen) und Charaktere (mit transparentem Hintergrund) können via 'Wahl' Button eingefügt werden.\n- Audio-Dateien (MP3, WAV, OGG) können für Musik (Dauerschleife), SFX (einmalig) oder Voice (pro Textbox) genutzt werden.\nDie Text-Geschwindigkeit richtet sich automatisch nach der Länge der Voice-Datei."
                },
                HelpChapter {
                    title: "4. Entscheidungen & Branching",
                    content: "Um deine Story nicht-linear zu machen, nutze Labels, Jumps und Choices.\n- Label: Setzt eine unsichtbare Markierung (z.B. 'ende_gut').\n- Jump: Springt sofort und unsichtbar zu einem bestimmten Label.\n- Choice: Stellt dem Spieler eine Frage und zeigt Buttons für Antworten. Jede Antwort hat ein 'Ziel-Label', zu dem gesprungen wird, sobald der Spieler sie anklickt."
                },
            ],
        }
    }
}

// --- ZUSTANDS-MASCHINE ---
enum AppState {
    MainMenu,
    Playing(Box<VnPlayer>),
    Editing(Box<VnEditor>),
    SettingsMenu,
    HelpMenu(HelpState),
}

struct VisualNovelApp {
    state: AppState,
    settings: Settings,
    show_exit_warning: bool,
    skip_exit_warning: bool,
    error_message: Option<String>,
}

impl VisualNovelApp {
    fn new(initial_state: AppState) -> Self {
        Self {
            state: initial_state,
            settings: Settings::load(),
            show_exit_warning: false,
            skip_exit_warning: false,
            error_message: None,
        }
    }
}

// HIER ist unsere echte Haupt-App!
impl eframe::App for VisualNovelApp {

    // FIX 1: eframe 0.34 verlangt 'fn ui' anstelle von 'fn update'
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {

        let mut go_back = false;

        // --- GLOBALE NAVIGATION ---
        // Ein "Zurück"-Knopf, der IMMER ganz oben angezeigt wird, außer wir sind im Hauptmenü
        if !matches!(self.state, AppState::MainMenu) {

            // FIX 2: Panels verlangen jetzt .show_inside(ui, ...) statt .show(ctx, ...)
            egui::Panel::top("global_nav").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("⬅ Zurück zum Hauptmenü").clicked() {

                        // Wenn wir im Editor sind UND die Warnung nicht blockiert ist: Popup zeigen!
                        if matches!(self.state, AppState::Editing(_)) && !self.skip_exit_warning {
                            self.show_exit_warning = true;
                        } else {
                            // Wenn wir im Spiel sind oder die Warnung übersprungen wird: Direkt zurück!
                            go_back = true;
                        }
                    }
                });
            });
        }

        // --- DAS WARN-POPUP ---
        if self.show_exit_warning {
            // Ein freischwebendes Fenster, das in der Mitte verankert wird
            // Fenster schweben "über" allem, daher heften wir sie an den Context: ui.ctx()
            egui::Window::new("⚠ Ungespeicherte Änderungen")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Bist du sicher, dass du zum Hauptmenü zurückkehren möchtest?");
                    ui.label(egui::RichText::new("Nicht gespeicherte Änderungen gehen dabei verloren!").color(egui::Color32::RED));

                    ui.add_space(15.0);

                    // Die Checkbox, um das Fenster für diese Sitzung abzustellen
                    ui.checkbox(&mut self.skip_exit_warning, "In dieser Sitzung nicht mehr fragen");

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        if ui.button("❌ Abbrechen").clicked() {
                            self.show_exit_warning = false; // Schließt das Popup
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✅ Ja, verlassen").clicked() {
                                self.show_exit_warning = false;
                                go_back = true; // Löst den Wechsel zum Hauptmenü aus
                            }
                        });
                    });
                });
        }

        // --- DER ZUSTANDS-WECHSEL ---
        if go_back {
            self.state = AppState::MainMenu;
            self.show_exit_warning = false; // Zur Sicherheit das Popup schließen
        }

        // --- DAS FEHLER-POPUP ---
        if let Some(err) = &self.error_message {
            let mut close = false;
            egui::Window::new("❌ Fehler beim Laden")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(egui::RichText::new(err).color(egui::Color32::LIGHT_RED));
                    ui.add_space(15.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("OK").clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.error_message = None;
            }
        }

        // --- HAUPTBEREICH ANZEIGEN ---
        // FIX 3: Auch hier 'show_inside(ui, ...)'!
        egui::CentralPanel::default().show_inside(ui, |ui| {

            match &mut self.state {

                // 1. DAS HAUPTMENÜ
                AppState::MainMenu => {
                    let mut start_pressed = false;
                    let mut editor_pressed = false;
                    let mut settings_pressed = false;
                    let mut help_pressed = false;
                    
                    if self.settings.keybindings.main_start.is_pressed(ui) { start_pressed = true; }
                    if self.settings.keybindings.main_editor.is_pressed(ui) { editor_pressed = true; }
                    if self.settings.keybindings.main_settings.is_pressed(ui) { settings_pressed = true; }
                    if self.settings.keybindings.main_help.is_pressed(ui) { help_pressed = true; }

                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui.heading(egui::RichText::new("📖 Visual Novel Studio").size(48.0).color(egui::Color32::from_rgb(120, 190, 255)));
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Erschaffe und erlebe interaktive Geschichten").size(24.0).italics().color(egui::Color32::GRAY));
                            ui.add_space(60.0);

                            if (ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("▶ Spiel laden & starten").size(20.0))).clicked() || start_pressed)
                                && let Some(path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                                    match std::fs::read_to_string(&path) {
                                        Ok(json_string) => match serde_json::from_str::<Chapter>(&json_string) {
                                            Ok(chapter) => {
                                                self.state = AppState::Playing(Box::new(VnPlayer::new(chapter)));
                                            }
                                            Err(err) => {
                                                self.error_message = Some(format!("Fehler beim Parsen der Datei '{}':\n\n{}", path.display(), err));
                                            }
                                        },
                                        Err(err) => {
                                            self.error_message = Some(format!("Konnte Datei '{}' nicht öffnen:\n\n{}", path.display(), err));
                                        }
                                    }
                                }

                            ui.add_space(20.0);

                            if ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("✏ Neuen Editor öffnen").size(20.0))).clicked() || editor_pressed {
                                self.state = AppState::Editing(Box::new(VnEditor::new()));
                            }

                            ui.add_space(20.0);

                            if ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("⚙ Einstellungen").size(20.0))).clicked() || settings_pressed {
                                self.state = AppState::SettingsMenu;
                            }

                            ui.add_space(20.0);

                            if ui.add_sized([300.0, 60.0], egui::Button::new(egui::RichText::new("❓ Hilfe & Anleitung").size(20.0))).clicked() || help_pressed {
                                self.state = AppState::HelpMenu(HelpState::new());
                            }

                        });
                    });
                }

                // 2. DAS SPIEL (VnPlayer)
                AppState::Playing(player) => {
                    player.ui(ui);
                }

                // 3. DER EDITOR (VnEditor)
                AppState::Editing(editor) => {
                    editor.ui(ui, &self.settings);
                }

                AppState::SettingsMenu => {
                    let mut return_to_main = false;
                    
                    if self.settings.keybindings.global_back.is_pressed(ui) { return_to_main = true; }
                    
                    egui::Window::new("Einstellungen")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .collapsible(false)
                        .resizable(false)
                        .show(ui.ctx(), |ui| {
                            ui.vertical_centered(|ui| {
                                self.settings.ui(ui);
                                ui.add_space(20.0);
                                if ui.button(egui::RichText::new("🔙 Zurück zum Hauptmenü").size(18.0)).clicked() {
                                    return_to_main = true;
                                }
                            });
                        });
                    if return_to_main {
                        self.state = AppState::MainMenu;
                    }
                }

                // 5. HILFE MENU
                AppState::HelpMenu(help) => {
                    let mut return_to_main = false;
                    
                    if self.settings.keybindings.global_back.is_pressed(ui) { return_to_main = true; }
                    
                    ui.horizontal(|ui| {
                        ui.heading("❓ Hilfe & Anleitung");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔙 Zurück zum Hauptmenü").clicked() {
                                return_to_main = true;
                            }
                        });
                    });
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        ui.label("🔍 Suche:");
                        ui.text_edit_singleline(&mut help.search_query);
                    });
                    
                    ui.separator();

                    let query = help.search_query.to_lowercase();
                    
                    // Finde passende Kapitel
                    let filtered_chapters: Vec<(usize, &HelpChapter)> = help.chapters
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.title.to_lowercase().contains(&query) || c.content.to_lowercase().contains(&query))
                        .collect();

                    egui::Panel::left("help_chapters").resizable(false).exact_size(250.0).show_inside(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if filtered_chapters.is_empty() {
                                ui.label(egui::RichText::new("Keine Ergebnisse.").italics().color(egui::Color32::GRAY));
                            }
                            for (original_idx, chapter) in filtered_chapters {
                                let is_selected = help.selected_chapter == original_idx;
                                if ui.selectable_label(is_selected, chapter.title).clicked() {
                                    help.selected_chapter = original_idx;
                                }
                            }
                        });
                    });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        if help.selected_chapter < help.chapters.len() {
                            let active_chapter = &help.chapters[help.selected_chapter];
                            ui.heading(active_chapter.title);
                            ui.add_space(15.0);
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.label(egui::RichText::new(active_chapter.content).size(16.0));
                            });
                        }
                    });
                    
                    if return_to_main {
                        self.state = AppState::MainMenu;
                    }
                }
            }
        });
    }
}

fn setup_custom_theme(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    
    style.spacing.item_spacing = egui::vec2(16.0, 16.0);
    style.spacing.button_padding = egui::vec2(20.0, 12.0);
    style.spacing.window_margin = egui::Margin::same(24);
    
    // Erhöhe alle Schriftgrößen um 15%
    for (_text_style, font_id) in style.text_styles.iter_mut() {
        font_id.size *= 1.15;
    }
    
    let mut visuals = egui::Visuals::dark();
    
    // Edle Dunkel-Palette
    visuals.window_fill = egui::Color32::from_rgb(30, 32, 40);
    visuals.panel_fill = egui::Color32::from_rgb(22, 24, 30);
    
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 48, 60);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(65, 130, 230);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 100, 190);
    
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80));
    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    
    visuals.selection.bg_fill = egui::Color32::from_rgb(65, 130, 230);

    ctx.set_global_style(style);
    ctx.set_visuals(visuals);
}

fn print_cli_help() {
    println!("VN Creator - Visual Novel Studio & Player (v1.1.0)");
    println!("\nVerwendung:");
    println!("  VN_Creator                   Startet die GUI im Hauptmenü");
    println!("  VN_Creator --mcp             Startet den Model Context Protocol (MCP) Server über stdio");
    println!("  VN_Creator --player <pfad>   Startet den Player direkt mit der angegebenen Kapitel-JSON");
    println!("  VN_Creator <pfad.json>       Startet den Player direkt mit der angegebenen Datei");
    println!("  VN_Creator --editor [pfad]   Startet den Editor direkt (optional mit geladenem Kapitel)");
    println!("  VN_Creator --help, -h        Zeigt diese Hilfe an");
}

fn parse_cli_state(args: &[String]) -> AppState {
    if args.len() <= 1 {
        return AppState::MainMenu;
    }

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--player" && i + 1 < args.len() {
            let path = &args[i + 1];
            match std::fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<Chapter>(&content) {
                    Ok(chapter) => return AppState::Playing(Box::new(VnPlayer::new(chapter))),
                    Err(e) => eprintln!("Fehler beim Parsen von '{}': {}", path, e),
                },
                Err(e) => eprintln!("Fehler beim Öffnen von '{}': {}", path, e),
            }
            i += 2;
            continue;
        } else if arg == "--editor" {
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                let path = &args[i + 1];
                match std::fs::read_to_string(path) {
                    Ok(content) => match serde_json::from_str::<Chapter>(&content) {
                        Ok(chapter) => return AppState::Editing(Box::new(VnEditor::with_chapter(chapter))),
                        Err(e) => eprintln!("Fehler beim Parsen von '{}': {}", path, e),
                    },
                    Err(e) => eprintln!("Fehler beim Öffnen von '{}': {}", path, e),
                }
                i += 2;
                continue;
            } else {
                return AppState::Editing(Box::new(VnEditor::new()));
            }
        } else if arg.ends_with(".json") && !arg.starts_with("--") {
            match std::fs::read_to_string(arg) {
                Ok(content) => match serde_json::from_str::<Chapter>(&content) {
                    Ok(chapter) => return AppState::Playing(Box::new(VnPlayer::new(chapter))),
                    Err(e) => eprintln!("Fehler beim Parsen von '{}': {}", arg, e),
                },
                Err(e) => eprintln!("Fehler beim Öffnen von '{}': {}", arg, e),
            }
        }
        i += 1;
    }

    AppState::MainMenu
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--mcp") {
        mcp::run_mcp_server()?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_cli_help();
        return Ok(());
    }

    let initial_state = parse_cli_state(&args);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Visual Novel Studio"),
        ..Default::default()
    };

    eframe::run_native(
        "Visual Novel Studio",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            setup_custom_theme(&cc.egui_ctx);
            Ok(Box::new(VisualNovelApp::new(initial_state)))
        }),
    )?;

    Ok(())
}