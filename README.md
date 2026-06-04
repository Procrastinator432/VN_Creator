# VN Creator

Ein Visual Novel Studio, das es erlaubt, interaktive Geschichten (Visual Novels) zu erstellen und zu spielen. Entwickelt in Rust mit `egui` / `eframe`.

## Features
- **Editor:** Erstelle Kapitel, Dialoge, Charaktere und Entscheidungen direkt in der App.
- **Player:** Spiele deine erstellen Kapitel ab, inkl. Text-Animationen, Audio (Musik, Voice, SFX) und Branching (Choices).
- **Settings:** Anpassbare Tastenkürzel und globale Lautstärke.
- **Hilfe:** Integriertes Hilfesystem.

## Bauen & Starten

Stelle sicher, dass [Rust und Cargo](https://rustup.rs/) installiert sind.

Klonen und starten:
```bash
git clone <dein-repo-url>
cd VN_Creator
cargo run --release
```

## Ordnerstruktur
Das Projekt erwartet einen `assets/` Ordner, in den Bilder und Audio-Dateien kopiert werden. Wenn du im Editor Medien auswählst, werden diese automatisch in den `assets` Ordner kopiert.

## Lizenz
Dieses Projekt ist unter der MIT-Lizenz lizenziert - siehe die [LICENSE](LICENSE) Datei für Details.
