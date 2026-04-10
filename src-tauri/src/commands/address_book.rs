//! Address Book Tauri Commands
//!
//! Commands for managing the address book (adding, listing, updating, deleting, searching entries).

use tauri::State;

use crate::address_book::{AddAddressBookRequest, AddressBookEntry, UpdateAddressBookRequest};
use crate::AppState;

// ============================================================================
// Address Book Commands
// ============================================================================

#[tauri::command]
pub async fn add_address_book_entry(
    state: State<'_, AppState>,
    request: AddAddressBookRequest,
) -> Result<AddressBookEntry, String> {
    let mut address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book
        .add_entry(request)
        .map_err(|e| format!("Failed to add address book entry: {}", e))
}

#[tauri::command]
pub async fn list_address_book_entries(
    state: State<'_, AppState>,
) -> Result<Vec<AddressBookEntry>, String> {
    let address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    Ok(address_book.list_entries())
}

#[tauri::command]
pub async fn get_address_book_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<AddressBookEntry, String> {
    let address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book
        .get_entry(&id)
        .cloned()
        .ok_or_else(|| format!("Address book entry with ID '{}' not found", id))
}

#[tauri::command]
pub async fn update_address_book_entry(
    state: State<'_, AppState>,
    request: UpdateAddressBookRequest,
) -> Result<AddressBookEntry, String> {
    let mut address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book
        .update_entry(request)
        .map_err(|e| format!("Failed to update address book entry: {}", e))
}

#[tauri::command]
pub async fn delete_address_book_entry(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let mut address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    address_book
        .delete_entry(&id)
        .map_err(|e| format!("Failed to delete address book entry: {}", e))?;

    Ok(format!("Address book entry '{}' deleted successfully", id))
}

#[tauri::command]
pub async fn search_address_book_entries(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<AddressBookEntry>, String> {
    let address_book = state
        .address_book_manager
        .lock()
        .map_err(|e| format!("Failed to lock address book manager: {}", e))?;

    Ok(address_book.search_entries(&query))
}