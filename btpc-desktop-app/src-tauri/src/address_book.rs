//! Address Book Management
//!
//! Manages saved recipient addresses for quick selection when sending funds.
//! Provides CRUD operations for address book entries with persistent storage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::error::{BtpcError, BtpcResult};
use btpc_core::crypto::Address;

/// Address book entry with label and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBookEntry {
    /// Unique identifier for this entry
    pub id: String,
    /// User-friendly label/nickname for this address
    pub label: String,
    /// BTPC address (Base58-encoded, ~34 characters)
    pub address: String,
    /// Optional notes about this recipient
    pub notes: Option<String>,
    /// When this entry was created
    pub created_at: DateTime<Utc>,
    /// When this address was last used for sending
    pub last_used: Option<DateTime<Utc>>,
    /// Number of times this address has been used
    pub usage_count: u32,
    /// Optional category/tag for organization
    pub category: Option<String>,
}

/// Request to add a new address book entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAddressBookRequest {
    pub label: String,
    pub address: String,
    pub notes: Option<String>,
    pub category: Option<String>,
}

/// Request to update an existing address book entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAddressBookRequest {
    pub id: String,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub category: Option<String>,
}

/// Address book manager
pub struct AddressBookManager {
    /// Storage file path
    storage_file: PathBuf,
    /// In-memory address book entries
    entries: HashMap<String, AddressBookEntry>,
}

impl AddressBookManager {
    /// Create a new address book manager
    pub fn new(storage_dir: PathBuf) -> BtpcResult<Self> {
        // Ensure storage directory exists
        std::fs::create_dir_all(&storage_dir)
            .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::DirectoryCreationFailed {
                path: storage_dir.display().to_string(),
            }))?;

        let storage_file = storage_dir.join("address_book.json");

        let mut manager = Self {
            storage_file,
            entries: HashMap::new(),
        };

        // Load existing entries
        manager.load_entries()?;

        Ok(manager)
    }

    /// Add a new address book entry
    pub fn add_entry(&mut self, request: AddAddressBookRequest) -> BtpcResult<AddressBookEntry> {
        // Validate address format (Base58 with checksum)
        if Address::from_string(&request.address).is_err() {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "address_format".to_string(),
                message: "Invalid BTPC address format (must be valid Base58 address with checksum)".to_string(),
            }));
        }

        // Validate label is not empty
        if request.label.trim().is_empty() {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "label_required".to_string(),
                message: "Label cannot be empty".to_string(),
            }));
        }

        // Check if address already exists
        if self.entries.values().any(|e| e.address == request.address) {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "duplicate_address".to_string(),
                message: format!("Address {} already exists in address book", request.address),
            }));
        }

        // Create new entry
        let entry = AddressBookEntry {
            id: Uuid::new_v4().to_string(),
            label: request.label,
            address: request.address,
            notes: request.notes,
            created_at: Utc::now(),
            last_used: None,
            usage_count: 0,
            category: request.category,
        };

        // Store entry
        self.entries.insert(entry.id.clone(), entry.clone());
        self.save_entries()?;

        Ok(entry)
    }

    /// List all address book entries
    pub fn list_entries(&self) -> Vec<AddressBookEntry> {
        let mut entries: Vec<AddressBookEntry> = self.entries.values().cloned().collect();
        // Sort by most recently used, then by label
        entries.sort_by(|a, b| {
            match (a.last_used, b.last_used) {
                (Some(a_used), Some(b_used)) => b_used.cmp(&a_used),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.label.cmp(&b.label),
            }
        });
        entries
    }

    /// Get a specific address book entry by ID
    pub fn get_entry(&self, id: &str) -> Option<&AddressBookEntry> {
        self.entries.get(id)
    }

    /// Update an address book entry
    pub fn update_entry(&mut self, request: UpdateAddressBookRequest) -> BtpcResult<AddressBookEntry> {
        let entry = self.entries.get_mut(&request.id)
            .ok_or_else(|| BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "entry_not_found".to_string(),
                message: format!("Address book entry with ID '{}' not found", request.id),
            }))?;

        // Update fields if provided
        if let Some(label) = request.label {
            if label.trim().is_empty() {
                return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                    rule: "label_required".to_string(),
                    message: "Label cannot be empty".to_string(),
                }));
            }
            entry.label = label;
        }

        if let Some(notes) = request.notes {
            entry.notes = Some(notes);
        }

        if let Some(category) = request.category {
            entry.category = Some(category);
        }

        let result = entry.clone();
        self.save_entries()?;

        Ok(result)
    }

    /// Delete an address book entry
    pub fn delete_entry(&mut self, id: &str) -> BtpcResult<()> {
        if self.entries.remove(id).is_none() {
            return Err(BtpcError::Validation(crate::error::ValidationError::CustomValidation {
                rule: "entry_not_found".to_string(),
                message: format!("Address book entry with ID '{}' not found", id),
            }));
        }

        self.save_entries()?;
        Ok(())
    }

    /// Mark an address as used (called when sending to this address)
    pub fn mark_as_used(&mut self, address: &str) -> BtpcResult<()> {
        // Find entry by address
        if let Some(entry) = self.entries.values_mut().find(|e| e.address == address) {
            entry.last_used = Some(Utc::now());
            entry.usage_count += 1;
            self.save_entries()?;
        }
        Ok(())
    }

    /// Search address book entries
    pub fn search_entries(&self, query: &str) -> Vec<AddressBookEntry> {
        let query_lower = query.to_lowercase();
        self.entries.values()
            .filter(|e| {
                e.label.to_lowercase().contains(&query_lower) ||
                e.address.to_lowercase().contains(&query_lower) ||
                e.notes.as_ref().is_some_and(|n| n.to_lowercase().contains(&query_lower)) ||
                e.category.as_ref().is_some_and(|c| c.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    // Private helper methods

    fn load_entries(&mut self) -> BtpcResult<()> {
        if !self.storage_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.storage_file)
            .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::ReadFailed {
                path: self.storage_file.display().to_string(),
                error: "Failed to read address book file".to_string(),
            }))?;

        self.entries = serde_json::from_str(&content)
            .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::DeserializationError {
                data_type: "address book".to_string(),
                error: "Invalid JSON format".to_string(),
            }))?;

        Ok(())
    }

    fn save_entries(&self) -> BtpcResult<()> {
        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|_| BtpcError::Utxo(crate::error::UtxoError::SerializationError {
                data_type: "address book".to_string(),
            }))?;

        std::fs::write(&self.storage_file, content)
            .map_err(|_| BtpcError::FileSystem(crate::error::FileSystemError::WriteFailed {
                path: self.storage_file.display().to_string(),
                error: "Failed to save address book".to_string(),
            }))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_add_and_list_entries() {
        let temp_dir = env::temp_dir().join("btpc_test_address_book");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut manager = AddressBookManager::new(temp_dir.clone()).unwrap();

        // Add entry
        let request = AddAddressBookRequest {
            label: "Alice".to_string(),
            address: "a".repeat(128),
            notes: Some("Test recipient".to_string()),
            category: Some("Friends".to_string()),
        };

        let entry = manager.add_entry(request).unwrap();
        assert_eq!(entry.label, "Alice");
        assert_eq!(entry.usage_count, 0);

        // List entries
        let entries = manager.list_entries();
        assert_eq!(entries.len(), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_duplicate_address_rejection() {
        let temp_dir = env::temp_dir().join("btpc_test_address_book_dup");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut manager = AddressBookManager::new(temp_dir.clone()).unwrap();

        let address = "b".repeat(128);

        // Add first entry
        let request1 = AddAddressBookRequest {
            label: "Bob".to_string(),
            address: address.clone(),
            notes: None,
            category: None,
        };
        manager.add_entry(request1).unwrap();

        // Try to add duplicate
        let request2 = AddAddressBookRequest {
            label: "Bob Copy".to_string(),
            address: address.clone(),
            notes: None,
            category: None,
        };
        let result = manager.add_entry(request2);
        assert!(result.is_err());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}