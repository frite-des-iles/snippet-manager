use eframe::{Frame, egui};
use rusqlite::{Connection, params, Result};

struct SnippetApp {
    conn: Connection,
    snippets: Vec<Snippet>,
    new_title: String,
    new_language: String,
    new_code: String,
    new_description: String,
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
        ui.label("Titre :");
        ui.text_edit_singleline(&mut self.new_title);

        ui.label("Langage :");
        ui.text_edit_singleline(&mut self.new_language);

        ui.label("Code :");
        ui.text_edit_multiline(&mut self.new_code);

        ui.label("Description :");
        ui.text_edit_singleline(&mut self.new_description);

        if ui.button("Ajouter").clicked() {
            insert_snippet(
                &self.conn,
                &self.new_title,
                &self.new_language,
                &self.new_code,
                &self.new_description,
            ).expect("Échec de l'insertion");

            self.snippets = load_all_snippets(&self.conn)
                .expect("Échec du rechargement");

            self.new_title.clear();
            self.new_language.clear();
            self.new_code.clear();
            self.new_description.clear();
        }

        ui.separator();

        for snippet in &self.snippets {
            ui.label(format!("{} [{}]", snippet.title, snippet.language));
            ui.separator();
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
        Box::new(|_cc| Ok(Box::new(SnippetApp { conn, snippets, new_title: String::new(), new_language: String::new(), new_code: String::new(), new_description: String::new() }))),
    )
}
