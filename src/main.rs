#![allow(unused)]
use anyhow::Result;
use axum::Router;
use axum::response::Html;
use axum::routing::get;
use notify::RecursiveMode;
use notify::Watcher;
use notify_debouncer_full::DebounceEventResult;
use notify_debouncer_full::DebouncedEvent;
use notify_debouncer_full::new_debouncer;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use tower_http::services::ServeDir;
use tower_livereload::LiveReloadLayer;
use tower_livereload::Reloader;

#[tokio::main]
async fn main() -> Result<()> {
    run_server().await?;
    Ok(())
}

async fn run_server() -> Result<()> {
    let dir_to_serve = Path::new(".");
    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    let service = ServeDir::new(dir_to_serve)
        .append_index_html_on_directories(true)
        .not_found_service(get(|| missing_page()));
    let app = Router::new().fallback_service(service).layer(livereload);
    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        None,
        move |result: DebounceEventResult| {
            if let Ok(debounced) = result {
                debounced.iter().find(|event| {
                    // dbg!(&event.event);
                    match event.event.kind {
                        notify::EventKind::Create(..) => has_trigger_file(&event.paths),
                        notify::EventKind::Modify(payload) => match payload {
                            notify::event::ModifyKind::Data(change_type) => match change_type {
                                _ => has_trigger_file(&event.paths),
                            },
                            _ => false,
                        },
                        _ => false,
                    }
                });

                // match debounced {
                //     DebouncedEvent { event, .. } => {
                //         dbg!(event);
                //         ()
                //     }
                // }

                //                dbg!(debounced);
            }
        }, // Ok(events) => {
           //     events
           //         .iter()
           //         .find(|event| {
           //             // dbg!(&event.event.kind);
           //             match event.event.kind {
           //                 notify::EventKind::Create(..) => true,
           //                 _ => false,
           //             }
           //             // println!("{event:?}");
           //             // reloader.reload();
           //         })
           // }
           // Err(errors) => false,
           //
    )?;
    debouncer.watch(".", RecursiveMode::Recursive)?;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5444").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

fn has_trigger_file(paths: &Vec<PathBuf>) -> bool {
    paths
        .iter()
        .filter(|p| p.is_file())
        .filter(|p| !p.ends_with("~"))
        .filter_map(|p| p.file_name())
        .map(|file_name| file_name.to_string_lossy())
        .filter(|file_name| !file_name.starts_with("."))
        .find(|file_name| {
            dbg!(&file_name);
            true
        })
        .is_some()
}

async fn missing_page() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head><style>body { background: black; color: white;}</style></head>
<body>Page Not Found</body>
</html>"#,
    )
}
