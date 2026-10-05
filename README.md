# VN Creator

Ein Visual Novel Studio & Player, das es erlaubt, interaktive Geschichten (Visual Novels) zu erstellen und zu spielen. Entwickelt in Rust mit `egui` / `eframe`.

Zusätzlich verfügt VN Creator über einen integrierten **Model Context Protocol (MCP) Server**, womit KI-Assistenten (wie Claude, Cursor, Antigravity etc.) Visual Novel Kapitel erstellen, bearbeiten, validieren, Medien verwalten und direkt im Player oder Editor starten können.

---

## Features

- **Editor:** Erstelle Kapitel, Dialoge, Charaktere und Entscheidungen direkt in der App.
- **Player:** Spiele deine erstellten Kapitel ab, inkl. Text-Animationen, Audio (Musik, Voice, SFX) und Branching (Choices).
- **Settings:** Anpassbare Tastenkürzel und globale Lautstärke.
- **Hilfe:** Integriertes Hilfesystem.
- **MCP Server (KI-Integration):** Standardisierter MCP-Server (JSON-RPC über stdio) zur vollständigen Steuerung des Studios durch KI-Agenten.
- **CLI-Startoptionen:** Starte direkt in den Player oder Editor mit bestimmten Kapiteln.

---

## Bauen & Starten

Stelle sicher, dass [Rust und Cargo](https://rustup.rs/) installiert sind.

### GUI-Modus (Standard)
```bash
cargo run --release
```

### CLI-Optionen
```bash
# Player direkt mit einem Kapitel starten
cargo run -- --player test_chapter.json
# oder kurz:
cargo run -- test_chapter.json

# Editor direkt starten (optional mit vorausgewähltem Kapitel)
cargo run -- --editor test_chapter.json
cargo run -- --editor

# MCP Server starten (stdio)
cargo run -- --mcp

# Hilfe anzeigen
cargo run -- --help
```

---

## Model Context Protocol (MCP) Integration

VN Creator kann direkt als MCP-Server in MCP-kompatiblen Clients (Claude Desktop, Cursor, Antigravity, etc.) eingebunden werden.

### Konfigurationsbeispiel (`claude_desktop_config.json` / `mcpServers`)

```json
{
  "mcpServers": {
    "vn-creator": {
      "command": "cargo",
      "args": ["run", "--release", "--", "--mcp"],
      "cwd": "D:\\Rust Projects\\VN_Creator"
    }
  }
}
```

Oder direkt mit dem kompilierten Binary:

```json
{
  "mcpServers": {
    "vn-creator": {
      "command": "D:\\Rust Projects\\VN_Creator\\target\\release\\VN_Creator.exe",
      "args": ["--mcp"],
      "cwd": "D:\\Rust Projects\\VN_Creator"
    }
  }
}
```

### Verfügbare MCP Tools

| Tool | Beschreibung | Parameter |
|---|---|---|
| `list_chapters` | Durchsucht das Projektverzeichnis nach `.json`-Kapiteln und liefert Metadaten (Titel, Aktionen, Charaktere, Choices). | `directory` *(optional)* |
| `read_chapter` | Liest ein Kapitel ein und liefert die JSON-Struktur sowie eine lesbare Szenenübersicht (Outline). | `path` *(erforderlich)*, `outline_only` *(optional)* |
| `create_chapter` | Erstellt ein neues Kapitel mit Titel, Theme und initialen Aktionen. | `path`, `title`, `theme`, `actions`, `overwrite` |
| `save_chapter` | Speichert oder überschreibt ein komplettes Chapter-Objekt als formatierte JSON. | `path`, `chapter`, `overwrite` |
| `add_actions` | Fügt Aktionen (Dialoge, Choices, Labels, Sprünge, Musik) am Ende oder an einer Position ein. | `path`, `actions`, `insert_at` |
| `validate_chapter` | Prüft Kapitel auf ungültige Sprungziele (`Jump`/`Choice`), doppelte/unbenutzte Labels, fehlende Assets und liefert Metriken. | `path` oder `chapter` |
| `list_assets` | Listet alle Medien in `assets/` auf (Hintergründe, Charakter-Sprites, Musik, SFX, Stimmen). | `category` *(all, backgrounds, characters, music, voices, sfx)* |
| `launch_player` | Startet den VN-Player in einem neuen Fenster mit dem angegebenen Kapitel zur sofortigen Vorschau. | `path` *(erforderlich)* |
| `launch_editor` | Startet den VN-Editor in einem neuen Fenster (optional mit Kapitel). | `path` *(optional)* |
| `get_settings` | Liefert die aktuellen Einstellungen der App (Lautstärken, Shortcuts). | *(keine)* |

### MCP Resources & Prompts
- **Resources:** `vn://chapter/<dateiname>` – Ermöglicht MCP-Clients das direkte Einsehen gespeicherter Kapitel als Ressource.
- **Prompts:** 
  - `create_vn_scene`: Strukturierter Prompt zur Generierung vollständiger Visual Novel Szenen.
  - `review_and_fix_chapter`: Prompt zur automatisierten Analyse und Korrektur von Kapiteln.

---

## Ordnerstruktur

- `src/models.rs`: Datenmodell (`Chapter`, `Action`, `Theme`, `SaveState`).
- `src/player.rs`: GUI-Player mit Text-Animationen, Audio und Verzweigungen.
- `src/editor.rs`: Visueller Kapitel-Editor mit Datei- und Audio-Browser.
- `src/mcp/`: Integrierter Model Context Protocol Server (`mod.rs`, `protocol.rs`, `tools.rs`, `validation.rs`).
- `assets/`: Medienordner für Bilder (`backgrounds`, `characters`) und Audio (`music`, `voices`, `sfx`).

---

## Lizenz
Dieses Projekt ist unter der MIT-Lizenz lizenziert - siehe die [LICENSE](LICENSE) Datei für Details.
