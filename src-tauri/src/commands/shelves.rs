use crate::error::Result;
use crate::models::{Book, Shelf};
use crate::services::shelf_service::ShelfService;
use crate::utils::validate;
use crate::AppState;
use tauri::State;

// ==================== Shelf CRUD Commands ====================

#[tauri::command]
pub fn get_shelves(state: State<AppState>) -> Result<Vec<Shelf>> {
    let conn = state.db.get_connection()?;
    ShelfService::get_shelves(&conn)
}

#[tauri::command]
pub fn get_shelf(id: i64, state: State<AppState>) -> Result<Shelf> {
    validate::require_positive_id(id, "id")?;
    let conn = state.db.get_connection()?;
    ShelfService::get_shelf(&conn, id)
}

#[tauri::command]
pub fn create_shelf(
    name: String,
    description: Option<String>,
    parent_id: Option<i64>,
    is_smart: bool,
    smart_rules: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    shelf_type: Option<String>,
    state: State<AppState>,
) -> Result<Shelf> {
    validate::require_non_empty(&name, "name")?;
    validate::require_max_length(&name, 500, "name")?;
    if let Some(ref desc) = description {
        validate::require_max_length(desc, 2000, "description")?;
    }
    if let Some(pid) = parent_id {
        validate::require_positive_id(pid, "parent_id")?;
    }
    let conn = state.db.get_connection()?;
    ShelfService::create_shelf(
        &conn,
        &name,
        description.as_deref(),
        parent_id,
        is_smart,
        smart_rules.as_deref(),
        icon.as_deref(),
        color.as_deref(),
        shelf_type.as_deref(),
    )
}

#[tauri::command]
pub fn update_shelf(
    id: i64,
    name: String,
    description: Option<String>,
    parent_id: Option<i64>,
    smart_rules: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    state: State<AppState>,
) -> Result<()> {
    validate::require_positive_id(id, "id")?;
    validate::require_non_empty(&name, "name")?;
    validate::require_max_length(&name, 500, "name")?;
    if let Some(ref desc) = description {
        validate::require_max_length(desc, 2000, "description")?;
    }
    if let Some(pid) = parent_id {
        validate::require_positive_id(pid, "parent_id")?;
    }
    let conn = state.db.get_connection()?;
    ShelfService::update_shelf(
        &conn,
        id,
        &name,
        description.as_deref(),
        parent_id,
        smart_rules.as_deref(),
        icon.as_deref(),
        color.as_deref(),
    )
}

#[tauri::command]
pub fn delete_shelf(id: i64, state: State<AppState>) -> Result<()> {
    validate::require_positive_id(id, "id")?;
    let conn = state.db.get_connection()?;
    ShelfService::delete_shelf(&conn, id)
}

// ==================== Book Management Commands ====================

#[tauri::command]
pub fn add_book_to_shelf(shelf_id: i64, book_id: i64, state: State<AppState>) -> Result<()> {
    validate::require_positive_id(shelf_id, "shelf_id")?;
    validate::require_positive_id(book_id, "book_id")?;
    let conn = state.db.get_connection()?;
    ShelfService::add_book_to_shelf(&conn, shelf_id, book_id)
}

#[tauri::command]
pub fn remove_book_from_shelf(shelf_id: i64, book_id: i64, state: State<AppState>) -> Result<()> {
    validate::require_positive_id(shelf_id, "shelf_id")?;
    validate::require_positive_id(book_id, "book_id")?;
    let conn = state.db.get_connection()?;
    ShelfService::remove_book_from_shelf(&conn, shelf_id, book_id)
}

#[tauri::command]
pub fn add_books_to_shelf(shelf_id: i64, book_ids: Vec<i64>, state: State<AppState>) -> Result<()> {
    validate::require_positive_id(shelf_id, "shelf_id")?;
    validate::require_non_empty_vec(&book_ids, "book_ids")?;
    let conn = state.db.get_connection()?;
    ShelfService::add_books_to_shelf(&conn, shelf_id, book_ids)
}

#[tauri::command]
pub fn get_shelf_books(shelf_id: i64, state: State<AppState>) -> Result<Vec<Book>> {
    validate::require_positive_id(shelf_id, "shelf_id")?;
    let conn = state.db.get_connection()?;
    ShelfService::get_shelf_books(&conn, shelf_id)
}

#[tauri::command]
pub fn get_book_shelf_ids(book_id: i64, state: State<AppState>) -> Result<Vec<i64>> {
    validate::require_positive_id(book_id, "book_id")?;
    let conn = state.db.get_connection()?;
    ShelfService::get_book_shelf_ids(&conn, book_id)
}

#[tauri::command]
pub fn get_nested_shelves(state: State<AppState>) -> Result<Vec<Shelf>> {
    let conn = state.db.get_connection()?;
    ShelfService::get_nested_shelves(&conn)
}

#[tauri::command]
pub fn toggle_book_favorite(book_id: i64, state: State<AppState>) -> Result<bool> {
    validate::require_positive_id(book_id, "book_id")?;
    let conn = state.db.get_connection()?;
    ShelfService::toggle_book_favorite(&conn, book_id)
}

#[tauri::command]
pub fn get_favorite_book_ids(state: State<AppState>) -> Result<Vec<i64>> {
    let conn = state.db.get_connection()?;
    ShelfService::get_favorite_book_ids(&conn)
}

#[tauri::command]
pub fn get_shelves_by_type(shelf_type: String, state: State<AppState>) -> Result<Vec<Shelf>> {
    let conn = state.db.get_connection()?;
    ShelfService::get_shelves_by_type(&conn, &shelf_type)
}

#[tauri::command]
pub fn preview_smart_shelf(smart_rules: String, state: State<AppState>) -> Result<i64> {
    let conn = state.db.get_connection()?;
    ShelfService::preview_smart_shelf(&conn, &smart_rules)
}
