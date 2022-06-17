#![warn(clippy::all)]
use colored::*;
use handle_errors::handle_errors;
use tracing_subscriber::fmt::format::FmtSpan;
use uuid::Uuid;
use warp::{http::Method, Filter};

use crate::{
    db::Database,
    routes::{
        kb_list::{add_kb, delete_kb, get_kb, get_kb_by_id, update_kb},
        reply::add_reply,
    },
};

mod db;
mod routes;
mod types;

#[tokio::main]
async fn main() {
    let log_rec = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "rust-kb-center=info,warp=info,error".to_owned());

    tracing_subscriber::fmt()
        .with_env_filter(log_rec)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let db = Database::new("postgres://postgres:123123@localhost/postgres").await;
    // run migrations after database successfull connection
    sqlx::migrate!()
        .run(&db.clone().connection)
        .await
        .expect("Migrations failed");
    let db_access = warp::any().map(move || db.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(&[Method::GET, Method::POST, Method::PUT, Method::DELETE]);

    let get_kb = warp::get()
        .and(warp::path("kb"))
        .and(warp::path::end())
        .and(warp::query())
        .and(db_access.clone())
        .and_then(get_kb)
        .with(warp::trace(|info| {
            tracing::info_span!(
                "GET kb request",
                method = %info.method(),
                path = %info.path(),
                id = %Uuid::new_v4(),
            )
        }));

    let get_kb_by_id = warp::get()
        .and(warp::path("kb"))
        .and(warp::path::param())
        .and(db_access.clone())
        .and_then(get_kb_by_id)
        .with(warp::trace(|info| {
            tracing::info_span!(
                "GET kb/{id} request",
                method = %info.method(),
                path = %info.path(),
                id = %Uuid::new_v4(),
            )
        }));

    let add_kb = warp::post()
        .and(warp::path("kb"))
        .and(warp::path::end())
        .and(db_access.clone())
        .and(warp::body::json())
        .and_then(add_kb);

    let update_kb = warp::put()
        .and(warp::path("kb"))
        .and(warp::path::param::<i32>())
        .and(warp::path::end())
        .and(db_access.clone())
        .and(warp::body::json())
        .and_then(update_kb);

    let delete_kb = warp::post()
        .and(warp::path("kb"))
        .and(warp::path::param::<i32>())
        .and(warp::path::end())
        .and(db_access.clone())
        .and_then(delete_kb);

    let add_reply = warp::post()
        .and(warp::path("kb"))
        .and(warp::path::end())
        .and(db_access.clone())
        .and(warp::body::form())
        .and_then(add_reply);

    let router = get_kb
        .or(add_kb)
        .or(get_kb_by_id)
        .or(update_kb)
        .or(delete_kb)
        .or(add_reply)
        .with(cors)
        .with(warp::trace::request())
        .recover(handle_errors);
    println!(
        "{}: {}",
        "Running server on port".green().bold(),
        " 8080 ".on_bright_yellow().black().blink()
    );
    warp::serve(router).run(([127, 0, 0, 1], 8080)).await;
}
