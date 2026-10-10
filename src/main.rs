use arboard;
use eframe::{Frame, egui};
use rusqlite::{Connection, Result, params};

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
    languages: Vec<String>,
    show_setting_window: bool,
    new_language_input: String,
    language_filter: String,
    language_dropdown_open: bool,
    active_language: Option<String>,
    lang_scroll: f32,
    lang_scroll_max: f32,
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

fn load_languages(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM languages ORDER BY name")?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    let mut lang = Vec::new();
    for row in rows {
        lang.push(row?);
    }
    Ok(lang)
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS languages (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL UNIQUE
        )",
        [],
    )?;

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM languages", [], |r| r.get(0))?;
    if count == 0 {
        for lang in ["rust", "python", "java", "javascript", "typescript", "bash"] {
            conn.execute("INSERT INTO languages (name) VALUES (?1)", params![lang])?;
        }
    }

    Ok(())
}

fn count_snippets(conn: &Connection) -> Result<i64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM snippets", [], |row| row.get(0))?;
    Ok(count)
}

fn insert_snippet(
    conn: &Connection,
    title: &str,
    language: &str,
    code: &str,
    description: &str,
) -> Result<()> {
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

fn insert_language(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("INSERT INTO languages (name) VALUES (?1)", params![name])?;
    Ok(())
}

fn delete_language(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM languages WHERE name = ?1", params![name])?;
    Ok(())
}

impl eframe::App for SnippetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let mut to_copy: Option<i64> = None;
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let arrow_width = 22.0;
                let right_width = 320.0;
                let left_width = (ui.available_width() - right_width - arrow_width * 2.0 - 8.0).max(80.0);

                // ── Flèche gauche ──
                let can_scroll_left = self.lang_scroll > 0.0;
                ui.add_enabled_ui(can_scroll_left, |ui| {
                    if ui
                        .add_sized([arrow_width, 24.0], egui::Button::new("◀"))
                        .clicked()
                    {
                        self.lang_scroll = (self.lang_scroll - 150.0).max(0.0);
                    }
                });

                // ── Zone des langages avec scroll horizontal, scrollbar cachée ──
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, 24.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let output = egui::ScrollArea::horizontal()
                            .id_salt("lang_tabs_scroll")
                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                            .horizontal_scroll_offset(self.lang_scroll)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);

                                    if ui
                                        .selectable_label(self.active_language.is_none(), "Tous")
                                        .clicked()
                                    {
                                        self.active_language = None;
                                    }

                                    let mut new_filter: Option<Option<String>> = None;
                                    for lang in &self.languages {
                                        let is_active = self.active_language.as_ref() == Some(lang);
                                        if ui.selectable_label(is_active, lang).clicked() {
                                            new_filter = Some(Some(lang.clone()));
                                        }
                                    }
                                    if let Some(f) = new_filter {
                                        self.active_language = f;
                                    }

                                    ui.add_space(4.0);
                                });
                            });

                        // Calcul du scroll max, à appliquer pour la flèche droite
                        self.lang_scroll_max =
                            (output.content_size.x - output.inner_rect.width()).max(0.0);
                    },
                );

                // ── Flèche droite ──
                let can_scroll_right = self.lang_scroll < self.lang_scroll_max;
                ui.add_enabled_ui(can_scroll_right, |ui| {
                    if ui
                        .add_sized([arrow_width, 24.0], egui::Button::new("▶"))
                        .clicked()
                    {
                        self.lang_scroll = (self.lang_scroll + 150.0).min(self.lang_scroll_max);
                    }
                });

                // ── Droite : recherche + ⚙ ──
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui.button("⚙").clicked() {
                        self.show_setting_window = true;
                    }
                    ui.add_space(8.0);
                    ui.add_sized(
                        [240.0, 22.0],
                        egui::TextEdit::singleline(&mut String::new()),
                    );
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
                        self.add_error = None;
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
                        if let Some(filter_lang) = &self.active_language {
                            if &snippet.language != filter_lang {
                                continue;
                            }
                        }

                        let selected = self.selected == Some(snippet.id);
                        let display_title = truncate(&snippet.title, 28);
                        let label = if selected {
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
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(8.0);

                                    if ui.button("Supprimer").clicked() {}
                                    if ui.button("Éditer").clicked() {}
                                    if ui.button("Copier").clicked() {
                                        to_copy = Some(snippet.id);
                                    }
                                },
                            );
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

                    // ── Bouton : toujours affiché ──
                    let button_text = if self.new_language.is_empty() {
                        "Choisir un langage...".to_string()
                    } else {
                        self.new_language.clone()
                    };
                    let button_response =
                        ui.add_sized([220.0, 24.0], egui::Button::new(button_text));

                    if button_response.clicked() {
                        self.language_dropdown_open = !self.language_dropdown_open;
                        self.language_filter.clear();
                    }

                    // ── Animation : 0 = fermé, 1 = ouvert ──
                    let anim_t = ui.ctx().animate_bool_with_time(
                        egui::Id::new("lang_dropdown_anim"),
                        self.language_dropdown_open,
                        0.15,
                    );

                    // ── Popup flottant ──
                    // On dessine tant que l'opacité n'est pas nulle → permet le fade-out.
                    if anim_t > 0.0 {
                        let popup_pos = button_response.rect.left_bottom() + egui::vec2(0.0, 2.0);

                        let area_response = egui::Area::new(egui::Id::new("lang_dropdown"))
                            .fixed_pos(popup_pos)
                            .order(egui::Order::Foreground)
                            .interactable(self.language_dropdown_open)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_opacity(anim_t);
                                    ui.set_min_width(220.0);

                                    // Champ de recherche
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.language_filter)
                                            .hint_text("Rechercher...")
                                            .desired_width(220.0),
                                    );
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

                                    // Liste filtrée
                                    let filter = self.language_filter.to_lowercase();
                                    let mut picked: Option<String> = None;

                                    egui::ScrollArea::vertical()
                                        .max_height(150.0)
                                        .show(ui, |ui| {
                                            let mut found = false;
                                            for lang in &self.languages {
                                                if filter.is_empty()
                                                    || lang.to_lowercase().contains(&filter)
                                                {
                                                    found = true;
                                                    if ui.selectable_label(false, lang).clicked() {
                                                        picked = Some(lang.clone());
                                                    }
                                                }
                                            }
                                            if !found {
                                                ui.weak("Aucun résultat");
                                            }
                                        });

                                    if let Some(l) = picked {
                                        self.new_language = l;
                                        self.language_filter.clear();
                                        self.language_dropdown_open = false;
                                    }
                                });
                            });

                        // Fermer au clic ailleurs
                        if self.language_dropdown_open
                            && ui.input(|i| i.pointer.any_click())
                            && !area_response.response.contains_pointer()
                            && !button_response.hovered()
                        {
                            self.language_dropdown_open = false;
                        }
                    }

                    ui.add_space(6.0);

                    ui.label("Code");
                    ui.text_edit_multiline(&mut self.new_code);
                    ui.add_space(6.0);

                    ui.label("Description");
                    ui.text_edit_singleline(&mut self.new_description);
                    ui.add_space(10.0);

                    if let Some(err) = &self.add_error {
                        ui.colored_label(egui::Color32::RED, err);
                        ui.add_space(4.0);
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Ajouter").clicked() {
                            let title = self.new_title.trim();
                            let language = self.new_language.trim();
                            let code = self.new_code.trim();
                            let description = self.new_description.trim();

                            if title.is_empty()
                                || language.is_empty()
                                || code.is_empty()
                                || description.is_empty()
                            {
                                self.add_error =
                                    Some("Tous les champs sont obligatoires.".to_string());
                            } else {
                                insert_snippet(&self.conn, title, language, code, description)
                                    .expect("Échec de l'insertion");

                                self.snippets =
                                    load_all_snippets(&self.conn).expect("Échec du rechargement");

                                self.new_title.clear();
                                self.new_language.clear();
                                self.new_code.clear();
                                self.new_description.clear();

                                self.add_error = None;
                                close_requested = true;
                            }
                        }
                        if ui.button("Annuler").clicked() {
                            self.add_error = None;
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

        if self.show_setting_window {
            let mut open = true;
            egui::Window::new("Paramètres")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([450.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ui.heading("Langages");
                    ui.add_space(6.0);

                    let mut to_delete: Option<String> = None;
                    for lang in &self.languages {
                        ui.horizontal(|ui| {
                            ui.label(lang);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🗑").clicked() {
                                        to_delete = Some(lang.clone());
                                    }
                                },
                            );
                        });
                    }
                    if let Some(name) = to_delete {
                        delete_language(&self.conn, &name).expect("Échec suppression");
                        self.languages = load_languages(&self.conn).expect("Échec reload");

                        if self.active_language.as_ref() == Some(&name) {
                            self.active_language = None;
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.label("Ajouter un langage");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_language_input);
                        if ui.button("Ajouter").clicked() {
                            let trimmed = self.new_language_input.trim().to_lowercase();
                            if !trimmed.is_empty() && !self.languages.contains(&trimmed) {
                                insert_language(&self.conn, &trimmed).expect("Échec insertion");
                                self.languages = load_languages(&self.conn).expect("Échec reload");
                                self.new_language_input.clear();
                            }
                        }
                    });

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.heading("Import / Export");
                    ui.label("(A venir)");
                });
            self.show_setting_window = open;
        }
    }
}

fn main() -> eframe::Result {
    let conn = Connection::open("snippets.db").expect("Impossible d'ouvrir la base");

    init_db(&conn).expect("Impossible de créer la table");
    let count = count_snippets(&conn).expect("Impossible de compter");
    let snippets = load_all_snippets(&conn).expect("Impossible de charger les snippets");
    let languages = load_languages(&conn).expect("Échec chargement langages");

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Snippet Manager",
        options,
        Box::new(|_cc| {
            Ok(Box::new(SnippetApp {
                conn,
                snippets,
                new_title: String::new(),
                new_language: String::new(),
                new_code: String::new(),
                new_description: String::new(),
                selected: None,
                show_add_window: false,
                add_error: None,
                show_setting_window: false,
                new_language_input: String::new(),
                language_filter: String::new(),
                language_dropdown_open: false,
                languages,
                active_language: None,
                lang_scroll: 0.0,
                lang_scroll_max: 0.0,
            }))
        }),
    )
}
