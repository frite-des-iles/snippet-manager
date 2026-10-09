use eframe::{Frame, egui};
use rusqlite::{Connection, params, Result};
use arboard;

struct SnippetApp {
    conn: Connection,
    snippets: Vec<Snippet>,
    new_title: String,
    new_language: String,
    new_code: String,
    new_description: String,
    selected: Option<i64>,
    show_add_window: bool,
    add_error: Option<String>,
}


struct Snippet {
    id: i64,
    title: String,
    language: String,
    code: String,
    description: Option<String>,
    favorite: i64,
    use_count: i64,
    created_at: i64,
    updated_at: i64,
}


fn truncate(s: &str, max_char: usize) -> String {
    let count = s.chars().count();
    if count <= max_char {
        s.to_string()
    } else {
        let cutoff = max_char.saturating_sub(3);
        let truncated: String = s.chars().take(cutoff).collect();
        format!("{}...", truncated)
    }
}


fn load_all_snippets(conn: &Connection) -> Result<Vec<Snippet>> {
    let mut stmt = conn.prepare("SELECT id, title, language, code, description, favorite, use_count, created_at, updated_at FROM snippets")?;
    let rows = stmt.query_map([], |row| {
        Ok(Snippet {
            id: row.get(0)?,
            title: row.get(1)?,
            language: row.get(2)?,
            code: row.get(3)?,
            description: row.get(4)?,
            favorite: row.get(5)?,
            use_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })?;

    let mut snippets = Vec::new();
    for row in rows {
        snippets.push(row?);
    }
    Ok(snippets)
}

fn increment_use_count(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE snippets SET use_count = use_count+1, updated_at = ?1 WHERE id = ?2",
        params![
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Horloge invalide")
                .as_secs() as i64,
            id,
        ],
    )?;
    Ok(())
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snippets (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        title           TEXT NOT NULL,
        language        TEXT NOT NULL,
        code            TEXT NOT NULL,
        description     TEXT,
        favorite        INTEGER DEFAULT 0,
        use_count       INTEGER DEFAULT 0,
        created_at      INTEGER NOT NULL,
        updated_at      INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}


fn count_snippets(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM snippets",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}


fn insert_snippet(conn: &Connection, title: &str, language: &str, code: &str, description: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Horloge système invalide")
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO snippets (title, language, code, description, favorite, use_count, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            title,
            language,
            code,
            description,
            0,
            0,
            now,
            now
        ],
    )?;
    Ok(())
}

impl eframe::App for SnippetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let mut to_copy: Option<i64> = None;
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                if ui.selectable_label(true, "Tous").clicked() {}
                if ui.selectable_label(true, "Rust").clicked() {}
                if ui.selectable_label(true, "Python").clicked() {}

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    ui.add_sized([240.0, 22.0], egui::TextEdit::singleline(&mut String::new()));
                    ui.label("🔍");
                });
            });
            ui.add_space(6.0);
        });

        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.label(format!("{} snippets", self.snippets.len()));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui.button("+ Nouveau snippet").clicked() {
                        self.show_add_window = true;
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::left("snippet_list")
            .resizable(true)
            .default_size(320.0)
            .min_size(220.0)
            .max_size(420.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut to_select: Option<i64> = None;
                    for snippet in &self.snippets {
                        let selected = self.selected == Some(snippet.id);
                        let display_title = truncate(&snippet.title, 28);
                        let label = if self.selected.is_some() {
                            format!("▶ {} [{}]", display_title, snippet.language)
                        } else {
                            format!("  {} [{}]", display_title, snippet.language)
                        };
                        if ui.selectable_label(selected, label).clicked() {
                            to_select = Some(snippet.id);
                        }
                    }
                    if let Some(id) = to_select {
                        self.selected = Some(id);
                    }
                });
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);

            if let Some(id) = self.selected {
                if let Some(snippet) = self.snippets.iter().find(|s| s.id == id) {
                    ui.vertical(|ui| {
                       ui.horizontal(|ui| {
                           ui.heading(&snippet.title);
                           ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                              ui.add_space(8.0);

                               if ui.button("Supprimer").clicked() {

                               }
                               if ui.button("Éditer").clicked() {

                               }
                               if ui.button("Copier").clicked() {
                                   to_copy = Some(snippet.id);
                               }
                           });
                       });

                        ui.label(format!("Langage : {}", snippet.language));
                        if let Some(desc) = &snippet.description {
                            ui.label(format!("Description: {}", desc));
                        }
                        ui.label(format!("Copié : {} fois", snippet.use_count));

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.monospace(&snippet.code);
                        });
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                       ui.label("Snippet introuvable");
                    });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Sélectionne un snippet à gauche.");
                });
            }
        });

        if let Some(id) = to_copy {
            if let Some(snippet) = self.snippets.iter().find(|s| s.id == id) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(snippet.code.clone());
                }
            }
            increment_use_count(&self.conn, id).expect("Échec incrément");
            self.snippets = load_all_snippets(&self.conn).expect("Échec reload");
        }

        if self.show_add_window {
            let mut open = true;
            let mut close_requested = false;
            egui::Window::new("Nouveau snippet")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 450.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Titre");
                    ui.text_edit_singleline(&mut self.new_title);

                    ui.label("Langage");
                    ui.text_edit_singleline(&mut self.new_language);
                    ui.add_space(6.0);

                    ui.label("Code");
                    ui.text_edit_multiline(&mut self.new_code);
                    ui.add_space(6.0);

                    ui.label("Description");
                    ui.text_edit_singleline(&mut self.new_description);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Ajouter").clicked() {
                            insert_snippet(
                                &self.conn,
                                &self.new_title,
                                &self.new_language,
                                &self.new_code,
                                &self.new_description,
                            ).expect("Échec de l'insertion");

                            self.snippets = load_all_snippets(&self.conn).expect("Échec du rechargement");

                            self.new_title.clear();
                            self.new_language.clear();
                            self.new_code.clear();
                            self.new_description.clear();

                            close_requested = true;
                        }
                        if ui.button("Annuler").clicked() {
                            close_requested = true;
                        }
                    });
                });
            if close_requested {
                self.show_add_window = false;
            } else {
                self.show_add_window = open;
            }
        }
    }
}

fn main() -> eframe::Result {
    let conn = Connection::open("snippets.db").expect("Impossible d'ouvrir la base");

    init_db(&conn).expect("Impossible de créer la table");
    let count = count_snippets(&conn).expect("Impossible de compter");
    let snippets = load_all_snippets(&conn).expect("Impossible de charger les snippets");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Snippet Manager",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp { conn, snippets, new_title: String::new(), new_language: String::new(), new_code: String::new(), new_description: String::new(), selected: None, show_add_window: false, add_error: None }))),
    )
}
