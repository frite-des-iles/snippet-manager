// ═══════════════════════════════════════════════════════════════════════
// IMPORTS
// ═══════════════════════════════════════════════════════════════════════
use arboard;
use eframe::{Frame, egui};
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use rfd::FileDialog;

// ═══════════════════════════════════════════════════════════════════════
// ÉTAT DE L'APPLICATION
// ═══════════════════════════════════════════════════════════════════════
struct SnippetApp {
    conn: Connection,
    snippets: Vec<Snippet>,
    languages: Vec<String>,
    new_title: String,
    new_language: String,
    new_code: String,
    new_description: String,
    add_error: Option<String>,
    show_add_window: bool,
    language_filter: String,
    language_dropdown_open: bool,
    selected: Option<i64>,
    active_language: Option<String>,
    lang_scroll: f32,
    lang_scroll_max: f32,
    search_query: String,
    show_setting_window: bool,
    new_language_input: String,
    show_delete_confirm: bool,
    delete_target_id: Option<i64>,
    delete_target_title: String,
    editing_id: Option<i64>,
    sort_mod: SortMod,
    sort_dropdown_open: bool,
    always_on_top: bool,
    import_export_message: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════
// STRUCT MIROIR DE LA TABLE `snippets`
// ═══════════════════════════════════════════════════════════════════════
#[derive(Serialize, Deserialize)]
struct Snippet {
    id: i64,
    signature: String,
    title: String,
    language: String,
    code: String,
    description: Option<String>,
    favorite: i64,
    use_count: i64,
    created_at: i64,
    updated_at: i64,
}

#[derive(Serialize, Deserialize)]
struct ExportData {
    version: u32,
    snippets: Vec<Snippet>,
    languages: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum SortMod {
    Recent,
    Alphabetical,
    MostUsed,
}

impl SortMod {
    fn label(&self) -> &'static str {
        match self {
            SortMod::Recent => "Plus récents",
            SortMod::Alphabetical => "Alphabétique",
            SortMod::MostUsed => "Plus utilisé",
        }
    }

    fn all() -> [SortMod; 3] {
        [SortMod::Recent, SortMod::Alphabetical, SortMod::MostUsed]
    }
}

// ═══════════════════════════════════════════════════════════════════════
// UTILITAIRES
// ═══════════════════════════════════════════════════════════════════════

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

fn compute_signature(
    title: &str,
    language: &str,
    code: &str,
    description: &str,
    created_at: i64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update(b"\x1F");
    hasher.update(language.as_bytes());
    hasher.update(b"\x1F");
    hasher.update(code.as_bytes());
    hasher.update(b"\x1F");
    hasher.update(description.as_bytes());
    hasher.update(b"\x1F");
    hasher.update(created_at.to_string().as_bytes());
    let hash = hasher.finalize();
    hash.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

fn export_data(conn: &Connection, path: impl AsRef<std::path::Path>,) -> Result<(), Box<dyn std::error::Error>> {
    let snippets = load_all_snippets(conn)?;
    let languages = load_languages(conn)?;

    let data = ExportData {
        version: 1,
        snippets,
        languages,
    };

    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(path, json)?;

    Ok(())
}

fn upsert_snippet_from_import(conn: &Connection, s: &Snippet) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO snippets (id, signature, title, language, code, description, favorite, use_count, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![s.id, s.signature, s.title, s.language, s.code, s.description, s.favorite, s.use_count, s.created_at, s.updated_at],
    )?;
    Ok(())
}

fn import_data(conn: &Connection, path: impl AsRef<std::path::Path>,) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let data: ExportData = serde_json::from_str(&json)?;

    let mut added = 0u32;
    let mut updated = 0u32;

    let local_languages = load_languages(conn)?;
    for lang in &data.languages {
        if !local_languages.contains(lang) {
            insert_language(conn, lang)?;
        }
    }

    let local_snippets = load_all_snippets(conn)?;
    let local_by_sig: std::collections::HashMap<&str, &Snippet> = local_snippets
        .iter()
        .map(|s| (s.signature.as_str(), s))
        .collect();

    for incoming in &data.snippets {
        match local_by_sig.get(incoming.signature.as_str()) {
            None => {
                conn.execute("INSERT INTO snippets (signature, title, language, code, description, favorite, use_count, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![incoming.signature, incoming.title, incoming.language, incoming.code, incoming.description, incoming.favorite, incoming.use_count,incoming.created_at, incoming.updated_at])?;
                added += 1;
            }
            Some(local) => {
                if incoming.updated_at > local.updated_at {
                    conn.execute(
                        "UPDATE snippets
                         SET title = ?1, language = ?2, code = ?3, description = ?4,
                             favorite = ?5, use_count = ?6, updated_at = ?7
                         WHERE id = ?8",
                        params![
                            incoming.title,
                            incoming.language,
                            incoming.code,
                            incoming.description,
                            incoming.favorite,
                            incoming.use_count,
                            incoming.updated_at,
                            local.id,
                        ],
                    )?;
                    updated += 1
                }
            }
        }
    }
    Ok((added, updated))
}

// ═══════════════════════════════════════════════════════════════════════
// ACCÈS BASE DE DONNÉES
// ═══════════════════════════════════════════════════════════════════════

fn load_all_snippets(conn: &Connection) -> Result<Vec<Snippet>> {
    let mut stmt = conn.prepare(
        "SELECT id, signature, title, language, code, description, favorite, use_count, created_at, updated_at
        FROM snippets"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Snippet {
            id: row.get(0)?,
            signature: row.get(1)?,
            title: row.get(2)?,
            language: row.get(3)?,
            code: row.get(4)?,
            description: row.get(5)?,
            favorite: row.get(6)?,
            use_count: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
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

fn toggle_favorite(conn: &Connection, id: i64) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Horloge invalide")
        .as_secs() as i64;

    conn.execute(
        "
    UPDATE snippets
    SET favorite = 1 - favorite, updated_at = ?1
    WHERE id = ?2
    ",
        params![now, id],
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

    let has_signature: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('snippets') WHERE name = 'signature'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)?;

    if !has_signature {
        conn.execute(
            "ALTER TABLE snippets ADD COLUMN signature TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }

    let mut stmt = conn.prepare(
        "SELECT id, title, language, code, description, created_at
            FROM snippets WHERE signature = ''",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;

    let mut to_update: Vec<(i64, String)> = Vec::new();
    for row in rows {
        let (id, title, language, code, description, created_at) = row?;
        let desc = description.unwrap_or_default();
        let sig = compute_signature(&title, &language, &code, &desc, created_at);
        to_update.push((id, sig));
    }

    for (id, sig) in to_update {
        conn.execute(
            "UPDATE snippets SET signature = ?1 WHERE id = ?2",
            [sig, id.to_string()],
        )?;
    }

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

    let signature = compute_signature(&title, &language, &code, &description, now);

    conn.execute(
        "INSERT INTO snippets (signature, title, language, code, description, favorite, use_count, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            signature,
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

fn update_snippet(
    conn: &Connection,
    id: i64,
    title: &str,
    language: &str,
    code: &str,
    description: &str,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Horloge invalide.")
        .as_secs() as i64;

    conn.execute(
        "UPDATE snippets
        SET title = ?1, language = ?2, code = ?3, description = ?4, updated_at = ?5
        WHERE id = ?6",
        params![title, language, code, description, now, id],
    )?;
    Ok(())
}

fn delete_snippet(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
    Ok(())
}

fn delete_language(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM languages WHERE name = ?1", params![name])?;
    Ok(())
}

fn snippet_matches(snippet: &Snippet, lang_filter: &Option<String>, query: &str) -> bool {
    if let Some(l) = lang_filter {
        if &snippet.language != l {
            return false;
        }
    }

    if query.is_empty() {
        return true;
    }

    let q = query.to_lowercase();
    snippet.title.to_lowercase().contains(&q)
        || snippet.code.to_lowercase().contains(&q)
        || snippet
            .description
            .as_ref()
            .map_or(false, |d| d.to_lowercase().contains(&q))
}

fn toggle_always_on_top(ui: &egui::Ui, state: &mut bool) {
    *state = !*state;
    let level = if *state {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    };
    ui.ctx()
        .send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
}

// ═══════════════════════════════════════════════════════════════════════
// INTERFACE
// ═══════════════════════════════════════════════════════════════════════
impl eframe::App for SnippetApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let mut to_toggle_favorite: Option<i64> = None;
        let mut to_delete_target: Option<i64> = None;
        let mut to_edit_target: Option<i64> = None;
        let mut to_copy: Option<i64> = None;

        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::T)) {
            toggle_always_on_top(ui, &mut self.always_on_top);
        }

        let mut visible: Vec<&Snippet> = self
            .snippets
            .iter()
            .filter(|s| snippet_matches(s, &self.active_language, &self.search_query))
            .collect();

        match self.sort_mod {
            SortMod::Recent => {
                visible.sort_by(|a, b| {
                    b.favorite
                        .cmp(&a.favorite)
                        .then(b.created_at.cmp(&a.created_at))
                });
            }
            SortMod::Alphabetical => {
                visible.sort_by(|a, b| {
                    b.favorite
                        .cmp(&a.favorite)
                        .then(a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                });
            }
            SortMod::MostUsed => visible.sort_by(|a, b| {
                b.favorite
                    .cmp(&a.favorite)
                    .then(b.use_count.cmp(&a.use_count))
            }),
        }

        let filtered_count = visible.len();
        let is_filtered = !self.search_query.is_empty() || self.active_language.is_some();

        // ═══════════════════════════════════════════════════════════════
        // BARRE DU HAUT
        // ═══════════════════════════════════════════════════════════════
        egui::Panel::top("top_bar").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let arrow_width = 22.0;
                let search_width = 180.0;
                let sort_width = 140.0;
                let icon_btn_width = 32.0;
                let magnifier_width = 20.0;
                let gap = 8.0;

                let right_width = gap
                    + magnifier_width
                    + icon_btn_width
                    + ui.spacing().item_spacing.x
                    + icon_btn_width
                    + ui.spacing().item_spacing.x
                    + sort_width
                    + ui.spacing().item_spacing.x
                    + search_width
                    + ui.spacing().item_spacing.x
                    + gap;

                let left_width = (ui.available_width()
                    - right_width
                    - arrow_width * 2.0
                    - ui.spacing().item_spacing.x * 2.0)
                    .max(80.0);

                let can_scroll_left = self.lang_scroll > 0.0;
                ui.add_enabled_ui(can_scroll_left, |ui| {
                    if ui
                        .add_sized([arrow_width, 20.0], egui::Button::new("◀"))
                        .clicked()
                    {
                        self.lang_scroll = (self.lang_scroll - 150.0).max(0.0);
                    }
                });

                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, 20.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        let output = egui::ScrollArea::horizontal()
                            .id_salt("lang_tabs_scroll")
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .horizontal_scroll_offset(self.lang_scroll)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.spacing_mut().button_padding.x = 3.0;

                                    let all_text = egui::RichText::new("Tous").size(12.0);
                                    if ui
                                        .selectable_label(self.active_language.is_none(), all_text)
                                        .clicked()
                                    {
                                        self.active_language = None;
                                    }

                                    let mut new_filter: Option<Option<String>> = None;
                                    for lang in &self.languages {
                                        let is_active = self.active_language.as_ref() == Some(lang);
                                        let label_text = egui::RichText::new(lang).size(12.0);
                                        if ui.selectable_label(is_active, label_text).clicked() {
                                            new_filter = Some(Some(lang.clone()));
                                        }
                                    }
                                    if let Some(f) = new_filter {
                                        self.active_language = f;
                                    }

                                    ui.add_space(4.0);
                                });
                            });

                        self.lang_scroll_max =
                            (output.content_size.x - output.inner_rect.width()).max(0.0);
                    },
                );

                let can_scroll_right = self.lang_scroll < self.lang_scroll_max;
                ui.add_enabled_ui(can_scroll_right, |ui| {
                    if ui
                        .add_sized([arrow_width, 20.0], egui::Button::new("▶"))
                        .clicked()
                    {
                        self.lang_scroll = (self.lang_scroll + 150.0).min(self.lang_scroll_max);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    let pin_icon = if self.always_on_top { "📌" } else { "📍" };
                    if ui.button(pin_icon).clicked() {
                        toggle_always_on_top(ui, &mut self.always_on_top);
                    }

                    ui.add_space(8.0);
                    if ui.button("⚙").clicked() {
                        self.show_setting_window = true;
                    }

                    let sort_button_text = self.sort_mod.label();
                    let sort_response =
                        ui.add_sized([sort_width, 22.0], egui::Button::new(sort_button_text));

                    if sort_response.clicked() {
                        self.sort_dropdown_open = !self.sort_dropdown_open;
                    }

                    let sort_anim = ui.ctx().animate_bool_with_time(
                        egui::Id::new("sort_dropdown_anim"),
                        self.sort_dropdown_open,
                        0.12,
                    );

                    if sort_anim > 0.0 {
                        let popup_pos = sort_response.rect.left_bottom() + egui::vec2(0.0, 2.0);
                        let area_response = egui::Area::new(egui::Id::new("sort_dropdown"))
                            .fixed_pos(popup_pos)
                            .order(egui::Order::Foreground)
                            .interactable(self.sort_dropdown_open)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_opacity(sort_anim);
                                    ui.set_min_width(140.0);
                                    for mode in SortMod::all() {
                                        let selected = self.sort_mod == mode;
                                        if ui.selectable_label(selected, mode.label()).clicked() {
                                            self.sort_mod = mode;
                                            self.sort_dropdown_open = false;
                                        }
                                    }
                                });
                            });
                        if self.sort_dropdown_open
                            && ui.input(|i| i.pointer.any_click())
                            && !area_response.response.contains_pointer()
                            && !sort_response.hovered()
                        {
                            self.sort_dropdown_open = false;
                        }
                    }

                    ui.add_space(8.0);
                    ui.add_sized(
                        [search_width, 22.0],
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text("Rechercher..."),
                    );
                    ui.label("🔍");
                });
            });
            ui.add_space(6.0);
        });

        // ═══════════════════════════════════════════════════════════════
        // BARRE DU BAS
        // ═══════════════════════════════════════════════════════════════
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                let label = if is_filtered {
                    format!("{} résultats sur {}", filtered_count, self.snippets.len())
                } else {
                    format!("{} snippets", self.snippets.len())
                };
                ui.label(label);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    if ui.button("+ Nouveau snippet").clicked() {
                        self.add_error = None;
                        self.editing_id = None;
                        self.new_title.clear();
                        self.new_language.clear();
                        self.new_code.clear();
                        self.new_description.clear();
                        self.show_add_window = true;
                    }
                });
            });
            ui.add_space(4.0);
        });

        // ═══════════════════════════════════════════════════════════════
        // LISTE À GAUCHE
        // ═══════════════════════════════════════════════════════════════
        egui::Panel::left("snippet_list")
            .resizable(true)
            .default_size(320.0)
            .min_size(220.0)
            .max_size(420.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut to_select: Option<i64> = None;
                    for &snippet in &visible {
                        let selected = self.selected == Some(snippet.id);
                        let truncated = truncate(&snippet.title, 26);
                        let star = if snippet.favorite == 1 { "⭐" } else { "  " };
                        let display_title = format!("{}{}", star, truncated);
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

        // ═══════════════════════════════════════════════════════════════
        // ZONE CENTRALE
        // ═══════════════════════════════════════════════════════════════
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

                                    let star = if snippet.favorite == 1 { "⭐" } else { "☆" };
                                    if ui.button(star).clicked() {
                                        to_toggle_favorite = Some(snippet.id);
                                    }
                                    if ui.button("Supprimer").clicked() {
                                        to_delete_target = Some(snippet.id);
                                    }
                                    if ui.button("Éditer").clicked() {
                                        to_edit_target = Some(snippet.id)
                                    }
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

        // ═══════════════════════════════════════════════════════════════
        // APPLICATION DES ACTIONS DIFFÉRÉES
        // ═══════════════════════════════════════════════════════════════

        if let Some(id) = to_delete_target {
            self.delete_target_id = Some(id);
            if let Some(s) = self.snippets.iter().find(|s| s.id == id) {
                self.delete_target_title = s.title.clone();
            }
            self.show_delete_confirm = true;
        }

        if let Some(id) = to_edit_target {
            if let Some(s) = self.snippets.iter().find(|s| s.id == id) {
                self.new_title = s.title.clone();
                self.new_language = s.language.clone();
                self.new_code = s.code.clone();
                self.new_description = s.description.clone().unwrap_or_default();
            }
            self.editing_id = Some(id);
            self.add_error = None;
            self.show_add_window = true;
        }

        if let Some(id) = to_copy {
            if let Some(snippet) = self.snippets.iter().find(|s| s.id == id) {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(snippet.code.clone());
                }
            }
            increment_use_count(&self.conn, id).expect("Échec incrément");
            self.snippets = load_all_snippets(&self.conn).expect("Échec reload");
        }

        // ═══════════════════════════════════════════════════════════════
        // MODALE : AJOUT DE SNIPPET
        // ═══════════════════════════════════════════════════════════════
        let add_anim = ui.ctx().animate_bool_with_time(
            egui::Id::new("add_window_anim"),
            self.show_add_window,
            0.12,
        );

        if add_anim > 0.0 {
            let mut open = true;
            let mut close_requested = false;
            let window_title = if self.editing_id.is_some() {
                "Modifier le snippet"
            } else {
                "Nouveau snippet"
            };

            egui::Window::new(window_title)
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 450.0])
                .interactable(self.show_add_window)
                .show(ui.ctx(), |ui| {
                    ui.set_opacity(add_anim);

                    ui.label("Titre");
                    ui.text_edit_singleline(&mut self.new_title);

                    ui.label("Langage");

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

                    let anim_t = ui.ctx().animate_bool_with_time(
                        egui::Id::new("lang_dropdown_anim"),
                        self.language_dropdown_open,
                        0.15,
                    );

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

                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.language_filter)
                                            .hint_text("Rechercher...")
                                            .desired_width(220.0),
                                    );
                                    ui.add_space(4.0);
                                    ui.separator();
                                    ui.add_space(4.0);

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
                        let action_label = if self.editing_id.is_some() {
                            "Enregistrer"
                        } else {
                            "Ajouter"
                        };

                        if ui.button(action_label).clicked() {
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
                                if let Some(id) = self.editing_id {
                                    update_snippet(
                                        &self.conn,
                                        id,
                                        title,
                                        language,
                                        code,
                                        description,
                                    )
                                    .expect("Échec de l'édition");
                                } else {
                                    insert_snippet(&self.conn, title, language, code, description)
                                        .expect("Échec de l'insertion");
                                }

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
                            self.editing_id = None;
                            self.new_title.clear();
                            self.new_language.clear();
                            self.new_code.clear();
                            self.new_description.clear();
                            close_requested = true;
                        }
                    });
                });

            // ⚠️ PATCH : on ne ferme QUE si l'utilisateur l'a demandé
            // (bouton Ajouter/Annuler, ou croix ✕ de la fenêtre).
            // Sinon, on ne touche à rien → l'animation peut descendre.
            if close_requested || !open {
                self.show_add_window = false;
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // MODALE : PARAMÈTRES
        // ═══════════════════════════════════════════════════════════════
        let settings_anim = ui.ctx().animate_bool_with_time(
            egui::Id::new("settings_window_anim"),
            self.show_setting_window,
            0.12,
        );

        if settings_anim > 0.0 {
            let mut open = true;

            egui::Window::new("Paramètres")
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([450.0, 400.0])
                .interactable(self.show_setting_window)
                .show(ui.ctx(), |ui| {
                    ui.set_opacity(settings_anim);

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
                    ui.add_space(6.0);

                    let mut do_export = false;
                    let mut do_import = false;

                    ui.horizontal(|ui| {
                        if ui.button("Export...").clicked() {
                            do_export = true;
                        }
                        if ui.button("Import...").clicked() {
                            do_import = true;
                        }
                    });

                    if let Some(msg) = &self.import_export_message {
                        ui.add_space(6.0);
                        ui.label(msg);
                    }

                    if do_export {
                        if let Some(path) = FileDialog::new()
                            .set_title("Exporter les snippets")
                            .set_file_name("snippets_export.json")
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
                            match export_data(&self.conn, &path) {
                                Ok(()) => {
                                    self.import_export_message =
                                        Some(format!("✅ Export réussi : {}", path.display()));
                                }
                                Err(e) => {
                                    self.import_export_message = Some(format!("❌ Erreur export : {}", e));
                                }
                            }
                        }
                    }

                    if do_import {
                        if let Some(path) = FileDialog::new()
                            .set_title("Importer des snippets")
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            match import_data(&self.conn, &path) {
                                Ok((added, updated)) => {
                                    self.import_export_message = Some(format!(
                                        "✅ {} ajoutés, {} mis à jour",
                                        added, updated
                                    ));
                                    self.snippets = load_all_snippets(&self.conn).expect("Échec reload");
                                    self.languages = load_languages(&self.conn).expect("Échec reload");
                                }
                                Err(e) => {
                                    self.import_export_message = Some(format!("❌ Erreur import : {}", e));
                                }
                            }
                        }
                    }
                });


            if !open {
                self.show_setting_window = false;
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // MODALE : CONFIRMATION DE SUPPRESSION
        // ═══════════════════════════════════════════════════════════════
        let delete_anim = ui.ctx().animate_bool_with_time(
            egui::Id::new("delete_window_anim"),
            self.show_delete_confirm,
            0.12,
        );

        if delete_anim > 0.0 {
            let mut open = true;
            let mut do_delete = false;
            let mut cancel = false;

            egui::Window::new("Confirmer la suppression")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_size([360.0, 130.0])
                .interactable(self.show_delete_confirm)
                .show(ui.ctx(), |ui| {
                    ui.set_opacity(delete_anim);

                    ui.label(format!("Supprimer « {} » ?", self.delete_target_title));
                    ui.add_space(8.0);
                    ui.label("Cette action est irréversible.");
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        if ui.button("Supprimer").clicked() {
                            do_delete = true;
                        }
                        if ui.button("Annuler").clicked() {
                            cancel = true;
                        }
                    });
                });

            if cancel || !open {
                self.show_delete_confirm = false;
            }
            if do_delete {
                if let Some(id) = self.delete_target_id {
                    delete_snippet(&self.conn, id).expect("Échec suppression");
                    self.snippets = load_all_snippets(&self.conn).expect("Échec reload");

                    if self.selected == Some(id) {
                        self.selected = None;
                    }
                }
                self.show_delete_confirm = false;
            }
        }

        if let Some(id) = to_toggle_favorite {
            toggle_favorite(&self.conn, id).expect("Échec toggle favori");
            self.snippets = load_all_snippets(&self.conn).expect("Échec reload");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// POINT D'ENTRÉE
// ═══════════════════════════════════════════════════════════════════════
fn main() -> eframe::Result {
    let conn = Connection::open("snippets.db").expect("Impossible d'ouvrir la base");

    init_db(&conn).expect("Impossible de créer la table");
    let _count = count_snippets(&conn).expect("Impossible de compter");
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
                languages,
                new_title: String::new(),
                new_language: String::new(),
                new_code: String::new(),
                new_description: String::new(),
                add_error: None,
                show_add_window: false,
                language_filter: String::new(),
                language_dropdown_open: false,
                selected: None,
                active_language: None,
                lang_scroll: 0.0,
                lang_scroll_max: 0.0,
                search_query: String::new(),
                show_setting_window: false,
                new_language_input: String::new(),
                show_delete_confirm: false,
                delete_target_id: None,
                delete_target_title: String::new(),
                editing_id: None,
                sort_mod: SortMod::Recent,
                sort_dropdown_open: false,
                always_on_top: false,
                import_export_message: None,
            }))
        }),
    )
}
