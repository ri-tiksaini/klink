#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use klink_common::ScanConfig;
use klink_core::{FileSearchDto, KlinkEngine, NodeDetailDto, ScanDiffDto, ScanSummaryDto, SearchHitDto};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::ipc::Response;
use tauri::State;

struct AppState {
    engine: Arc<KlinkEngine>,
}

#[tauri::command]
async fn pick_folder() -> Option<String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Open Repository Folder in Klink")
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
async fn scan_folder(path: String, state: State<'_, AppState>) -> Result<ScanSummaryDto, String> {
    let eng = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        eng.scan_folder(&PathBuf::from(path), &ScanConfig::default())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn get_packed_nodes(state: State<'_, AppState>) -> Response {
    Response::new(state.engine.packed_nodes_bytes())
}

#[tauri::command]
fn get_packed_edges(state: State<'_, AppState>) -> Response {
    Response::new(state.engine.packed_edges_bytes())
}

#[tauri::command]
fn query_impact_mask(node_id: u32, state: State<'_, AppState>) -> Result<Response, String> {
    let (bytes, _dto) = state
        .engine
        .query_impact_mask(node_id, 256 * 8)
        .ok_or_else(|| "node not found".to_string())?;
    Ok(Response::new(bytes))
}

#[tauri::command]
fn get_node_detail(node_id: u32, state: State<'_, AppState>) -> Option<NodeDetailDto> {
    state.engine.node_detail(node_id)
}

#[tauri::command]
fn search_files(query: String, state: State<'_, AppState>) -> Vec<FileSearchDto> {
    state.engine.search_files(&query, 15)
}

#[tauri::command]
fn search_symbols(query: String, state: State<'_, AppState>) -> Vec<SearchHitDto> {
    state.engine.search_symbols(&query, 15)
}

#[tauri::command]
fn get_scan_diff(state: State<'_, AppState>) -> Option<ScanDiffDto> {
    state.engine.scan_diff()
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>) {
    state.engine.cancel_scan();
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            engine: Arc::new(KlinkEngine::new()),
        })
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            scan_folder,
            get_packed_nodes,
            get_packed_edges,
            query_impact_mask,
            get_node_detail,
            search_files,
            search_symbols,
            get_scan_diff,
            cancel_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running Klink desktop");
}
