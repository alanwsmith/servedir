#![allow(unused)]
use anyhow::Result;
use axum::Router;
use axum::response::Html;
use axum::routing::get;
use notify::RecursiveMode;
use notify::event::DataChange;
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
            .filter_map(|path| std::path::absolute(path).ok())
            .filter(|path| self.check_path(&path))
            .for_each(|path| {
                let _ = self.detect_change(&path);
            });
        Ok(())
    }

    pub fn detect_change(&self, path: &PathBuf) -> Option<PathBuf> {
        if !self.check_path(path) {
            None
        } else {
            dbg!(&path);

            //let hash = self.hash_file(path).ok();
            //let mut stmt = self
            //    .conn
            //    .prepare("SELECT hash FROM files WHERE path = ?")
            //    .ok();
            //if stmt.query_row([format!("x{}", path.display().to_string())], |r| {
            //    // let check_hash = r.get_unwrap::<usize, String>(0);
            //    Ok(())
            //}) {
            //    dbg!("inserting data");
            //    // let insert_data = "INSERT INTO files (path, hash) VALUES (?1, ?2)";
            //    // self.conn
            //    //     .execute(insert_data, (path.display().to_string(), hash))
            //    //     .is_ok();
            //    //
            //};
            None
        }
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
        Ok(DirServer { conn, dir })
    }

    pub fn process_event(&self, debounced: DebounceEventResult) -> Option<PathBuf> {
        if let Ok(events) = debounced {
            events.iter().find_map(|event| match event.event.kind {
                notify::EventKind::Create(..) => {
                    event.paths.iter().find_map(|p| self.detect_change(&p))
                }
                notify::EventKind::Modify(change_kind) => match change_kind {
                    notify::event::ModifyKind::Data(..) => {
                        event.paths.iter().find_map(|p| self.detect_change(&p))
                    }
                    _ => None,
                },
                _ => None,
            })
        } else {
            None
        }
    }
}

//    // match event.event.kind {
//    //     notify::EventKind::Create(..) => {
//    //         false
//    //         // if has_trigger_file(&event.paths) {
//    //         //     true
//    //         // } else {
//    //         //     false
//    //         // }
//    //     }
//    //     notify::EventKind::Modify(payload) => match payload {
//    //         notify::event::ModifyKind::Data(change_type) => match change_type {
//    //             _ => {
//    //                 false
//    //                 // if has_trigger_file(&event.paths) {
//    //                 //     dbg!(&event);
//    //                 //     true
//    //                 // } else {
//    //                 //     false
//    //                 // }
//    //             }
//    //         },
//    //         _ => false,
//    //     },
//    //     _ => false,
//    // }

//}) {
//    //reloader.reload();
//}
//}

//debounced.iter().filter_map(|event| {
//    dbg!(&event);
//    // event.is
//    //find(|event| {
//    //dbg!(event?);
//    //false
//    Some("asdf".to_string())
//});

//    // // dbg!(&event.event);
//    // match event.event.kind {
//    //     notify::EventKind::Create(..) => {
//    //         false
//    //         // if has_trigger_file(&event.paths) {
//    //         //     true
//    //         // } else {
//    //         //     false
//    //         // }
//    //     }
//    //     notify::EventKind::Modify(payload) => match payload {
//    //         notify::event::ModifyKind::Data(change_type) => match change_type {
//    //             _ => {
//    //                 false
//    //                 // if has_trigger_file(&event.paths) {
//    //                 //     dbg!(&event);
//    //                 //     true
//    //                 // } else {
//    //                 //     false
//    //                 // }
//    //             }
//    //         },
//    //         _ => false,
//    //     },
//    //     _ => false,
//    // }
//}) {
//    //reloader.reload();
//}
//}

// false

#[tokio::main]
async fn main() -> Result<()> {
    run_server().await?;
    Ok(())
}

async fn run_server() -> Result<()> {
    let ds = DirServer::new()?;
    ds.load_files()?;
    let dir = Path::new(".");
    let conn = Connection::open_in_memory()?;
    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    let service = ServeDir::new(&ds.dir)
        .append_index_html_on_directories(true)
        .not_found_service(get(|| missing_page()));
    let app = Router::new().fallback_service(service).layer(livereload);
    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        None,
        move |result: DebounceEventResult| {
            //if let Ok(debounced) = result {
            if ds.process_event(result).is_some() {
                reloader.reload();
            }

            //if let Some(_) = debounced.iter().find(|event| {
            //    // // dbg!(&event.event);
            //    // match event.event.kind {
            //    //     notify::EventKind::Create(..) => {
            //    //         false
            //    //         // if has_trigger_file(&event.paths) {
            //    //         //     true
            //    //         // } else {
            //    //         //     false
            //    //         // }
            //    //     }
            //    //     notify::EventKind::Modify(payload) => match payload {
            //    //         notify::event::ModifyKind::Data(change_type) => match change_type {
            //    //             _ => {
            //    //                 false
            //    //                 // if has_trigger_file(&event.paths) {
            //    //                 //     dbg!(&event);
            //    //                 //     true
            //    //                 // } else {
            //    //                 //     false
            //    //                 // }
            //    //             }
            //    //         },
            //    //         _ => false,
            //    //     },
            //    //     _ => false,
            //    // }
            //}) {
            //    //reloader.reload();
            //}
            //}
        },
    )?;
    debouncer.watch(".", RecursiveMode::Recursive)?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5444").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
