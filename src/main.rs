#![allow(unused)]
use anyhow::Result;
use axum::Router;
use axum::response::Html;
use axum::routing::get;
use notify::RecursiveMode;
use notify_debouncer_full::DebounceEventResult;
use notify_debouncer_full::new_debouncer;
use rusqlite::Connection;
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tower_http::services::ServeDir;
use tower_livereload::LiveReloadLayer;
use walkdir::WalkDir;

struct DirServer {
    pub dir: PathBuf,
    pub conn: Connection,
}

impl DirServer {
    pub fn check_path(&self, path: &PathBuf) -> bool {
        if !path.is_file() {
            false
        } else if path.ends_with("~") {
            false
        } else if path
            .iter()
            .skip(1)
            .filter_map(|p| p.to_str())
            .any(|part| part.starts_with(".") || part == "target")
        {
            false
        } else {
            true
        }
    }

    pub fn load_files(&self) -> Result<()> {
        WalkDir::new(&self.dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| Some(entry.into_path()))
            .filter(|path| self.check_path(&path))
            .for_each(|path| {
                let _ = self.detect_change(&path);
            });
        Ok(())
    }

    pub fn detect_change(&self, path: &PathBuf) -> Result<()> {
        let hash = self.hash_file(path)?;
        let mut stmt = self.conn.prepare("SELECT hash FROM files WHERE path = ?");
        if !stmt?
            .query_row([format!("x{}", path.display().to_string())], |r| {
                let check_hash = r.get_unwrap::<usize, String>(0);
                Ok(())
            })
            .is_ok()
        {
            dbg!("inserting data");
            let insert_data = "INSERT INTO files (path, hash) VALUES (?1, ?2)";
            self.conn
                .execute(insert_data, (path.display().to_string(), hash))?;
        };
        Ok(())
    }

    pub fn hash_file(&self, path: &PathBuf) -> Result<String> {
        let contents = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(contents);
        let result: String = format!("{:X}", hasher.finalize());
        Ok((result))
    }

    pub fn new() -> Result<DirServer> {
        let dir = PathBuf::from(".");
        let conn = Connection::open_in_memory()?;
        let create_table_sql = "CREATE TABLE IF NOT EXISTS
            files (
                path TEXT PRIMARY KEY,
                hash TEXT NOT NULL 
            )";
        conn.execute(create_table_sql, ())?;

        // seed test data

        Ok(DirServer { conn, dir })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ds = DirServer::new()?;
    ds.load_files()?;
    // run_server().await?;
    Ok(())
}

async fn run_server() -> Result<()> {
    let dir = Path::new(".");
    let conn = Connection::open_in_memory()?;

    // let livereload = LiveReloadLayer::new();
    // let reloader = livereload.reloader();
    // let service = ServeDir::new(dir_to_serve)
    //     .append_index_html_on_directories(true)
    //     .not_found_service(get(|| missing_page()));
    // let app = Router::new().fallback_service(service).layer(livereload);
    // let mut debouncer = new_debouncer(
    //     Duration::from_millis(150),
    //     None,
    //     move |result: DebounceEventResult| {
    //         if let Ok(debounced) = result {
    //             if let Some(_) = debounced.iter().find(|event| {
    //                 // dbg!(&event.event);
    //                 match event.event.kind {
    //                     notify::EventKind::Create(..) => {
    //                         if has_trigger_file(&event.paths) {
    //                             true
    //                         } else {
    //                             false
    //                         }
    //                     }
    //                     notify::EventKind::Modify(payload) => match payload {
    //                         notify::event::ModifyKind::Data(change_type) => match change_type {
    //                             _ => {
    //                                 if has_trigger_file(&event.paths) {
    //                                     dbg!(&event);
    //                                     true
    //                                 } else {
    //                                     false
    //                                 }
    //                             }
    //                         },
    //                         _ => false,
    //                     },
    //                     _ => false,
    //                 }
    //             }) {
    //                 reloader.reload();
    //             }
    //         }
    //     },
    // )?;
    // debouncer.watch(".", RecursiveMode::Recursive)?;
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:5444").await.unwrap();
    // axum::serve(listener, app).await.unwrap();

    Ok(())
}

// fn has_trigger_file(paths: &Vec<PathBuf>) -> bool {
//     if let Some(path) = paths
//         .iter()
//         .filter(|p| p.is_file())
//         .filter(|p| !p.ends_with("~"))
//         .filter(|p| match p.file_name() {
//             Some(name) => !name.to_string_lossy().starts_with("."),
//             None => false,
//         })
//         .find_map(|p| Some(p))
//     {
//         println!("{}", path.display());
//         true
//     } else {
//         false
//     }
// }

async fn missing_page() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head><style>body { background: black; color: white;}</style></head>
<body>Page Not Found</body>
</html>"#,
    )
}
