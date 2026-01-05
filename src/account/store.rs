//! Account storage and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use super::types::{Account, AccountId, AccountType};
use super::auth::{hash_password, verify_password, encrypt_data, decrypt_data, AuthError};
use super::recovery::{RecoveryKey, RecoveryError};
use crate::storage::Storage;

#[derive(Debug, Clone)]
pub enum AccountStoreError {
    AccountNotFound,
    AccountAlreadyExists,
    AuthError(AuthError),
    RecoveryError(RecoveryError),
    InvalidCredentials,
    StorageError(String),
}

impl From<AuthError> for AccountStoreError {
    fn from(err: AuthError) -> Self {
        AccountStoreError::AuthError(err)
    }
}

impl From<RecoveryError> for AccountStoreError {
    fn from(err: RecoveryError) -> Self {
        AccountStoreError::RecoveryError(err)
    }
}

/// Account store for managing all accounts
#[derive(Serialize, Deserialize, Clone)]
pub struct AccountStore {
    accounts: HashMap<AccountId, Account>,
    
    #[serde(skip)]
    storage: Option<Arc<Storage>>,
}

impl AccountStore {
    /// Create a new empty account store
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            storage: None,
        }
    }
    
    /// Create with storage backend
    pub fn with_storage(storage: Arc<Storage>) -> Self {
        Self {
            accounts: HashMap::new(),
            storage: Some(storage),
        }
    }
    
    /// Create a new account
    pub fn create_account(
        &mut self,
        name: String,
        password: String,
        account_type: AccountType,
    ) -> Result<Account, AccountStoreError> {
        // Check if account already exists
        if self.accounts.contains_key(&name) {
            return Err(AccountStoreError::AccountAlreadyExists);
        }
        
        // Validate account name (lowercase, alphanumeric + underscore)
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(AccountStoreError::InvalidCredentials);
        }
        
        // Validate password strength (min 12 chars)
        if password.len() < 12 {
            return Err(AccountStoreError::InvalidCredentials);
        }
        
        // Hash password
        let (password_hash, salt) = hash_password(&password)?;
        
        // Generate recovery key
        let recovery = RecoveryKey::generate()?;
        let recovery_phrase = recovery.to_phrase();
        let recovery_pubkey = hex::encode(recovery.recovery_pubkey()?);
        
        // Generate signing keypair from recovery
        let (signing_privkey, signing_pubkey_bytes) = recovery.derive_keypair(0)?;
        let signing_pubkey = hex::encode(&signing_pubkey_bytes);
        
        // Encrypt private key with password
        let signing_privkey_encrypted = encrypt_data(&signing_privkey, &password, &salt)?;
        
        // Encrypt recovery seed with password
        let backup_seed_encrypted = encrypt_data(recovery_phrase.as_bytes(), &password, &salt)?;
        
        // Create account
        let account = Account {
            name: name.clone(),
            account_type,
            password_hash,
            salt,
            backup_seed_encrypted,
            recovery_pubkey,
            signing_pubkey,
            signing_privkey_encrypted,
            nonce: 0,
            created_at: current_timestamp(),
            metadata: HashMap::new(),
        };
        
        // Store account
        self.accounts.insert(name.clone(), account.clone());
        
        // Persist to storage if available
        if let Some(storage) = &self.storage {
            storage.save_account(&account)
                .map_err(|e| AccountStoreError::StorageError(e.to_string()))?;
        }
        
        Ok(account)
    }
    
    /// Authenticate an account with password
    pub fn authenticate(
        &self,
        name: &str,
        password: &str,
    ) -> Result<Vec<u8>, AccountStoreError> {
        let account = self.accounts.get(name)
            .ok_or(AccountStoreError::AccountNotFound)?;
        
        // Verify password
        verify_password(password, &account.password_hash)?;
        
        // Decrypt private key
        let signing_privkey = decrypt_data(
            &account.signing_privkey_encrypted,
            password,
            &account.salt,
        )?;
        
        // Return the decrypted private key bytes
        // The caller can use this with the existing crypto module
        Ok(signing_privkey)
    }
    
    /// Recover account using backup seed
    pub fn recover_account(
        &mut self,
        name: &str,
        recovery_phrase: &str,
        new_password: &str,
    ) -> Result<(), AccountStoreError> {
        let account = self.accounts.get_mut(name)
            .ok_or(AccountStoreError::AccountNotFound)?;
        
        // Verify recovery phrase
        let recovery = RecoveryKey::from_phrase(recovery_phrase)?;
        let recovery_pubkey = hex::encode(recovery.recovery_pubkey()?);
        
        if recovery_pubkey != account.recovery_pubkey {
            return Err(AccountStoreError::InvalidCredentials);
        }
        
        // Re-hash password with new password
        let (password_hash, salt) = hash_password(new_password)?;
        
        // Re-encrypt private key
        let (signing_privkey, _) = recovery.derive_keypair(0)?;
        let signing_privkey_encrypted = encrypt_data(&signing_privkey, new_password, &salt)?;
        
        // Re-encrypt backup seed
        let backup_seed_encrypted = encrypt_data(recovery_phrase.as_bytes(), new_password, &salt)?;
        
        // Update account
        account.password_hash = password_hash;
        account.salt = salt;
        account.signing_privkey_encrypted = signing_privkey_encrypted;
        account.backup_seed_encrypted = backup_seed_encrypted;
        
        // Persist to storage
        if let Some(storage) = &self.storage {
            storage.save_account(account)
                .map_err(|e| AccountStoreError::StorageError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    /// Get account by name
    pub fn get(&self, name: &str) -> Option<&Account> {
        self.accounts.get(name)
    }
    
    /// Get mutable account by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Account> {
        self.accounts.get_mut(name)
    }
    
    /// Increment account nonce
    pub fn increment_nonce(&mut self, name: &str) -> Result<u64, AccountStoreError> {
        let account = self.accounts.get_mut(name)
            .ok_or(AccountStoreError::AccountNotFound)?;
        
        account.nonce += 1;
        let new_nonce = account.nonce;
        
        // Persist to storage
        if let Some(storage) = &self.storage {
            storage.save_account(account)
                .map_err(|e| AccountStoreError::StorageError(e.to_string()))?;
        }
        
        Ok(new_nonce)
    }
    
    /// Get all accounts
    pub fn all_accounts(&self) -> Vec<&Account> {
        self.accounts.values().collect()
    }
    
    /// Get all account names
    pub fn account_names(&self) -> Vec<String> {
        self.accounts.keys().cloned().collect()
    }
}

impl Default for AccountStore {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::types::UserAccountData;
    
    #[test]
    fn test_create_account() {
        let mut store = AccountStore::new();
        
        let account = store.create_account(
            "alice".to_string(),
            "secure_password_123".to_string(),
            AccountType::User(UserAccountData::default()),
        ).unwrap();
        
        assert_eq!(account.name, "alice");
        assert!(account.can_sign());
    }
    
    #[test]
    fn test_authenticate() {
        let mut store = AccountStore::new();
        
        store.create_account(
            "alice".to_string(),
            "secure_password_123".to_string(),
            AccountType::User(UserAccountData::default()),
        ).unwrap();
        
        // Correct password
        let privkey_bytes = store.authenticate("alice", "secure_password_123").unwrap();
        assert!(privkey_bytes.len() > 0);
        
        // Wrong password
        assert!(store.authenticate("alice", "wrong_password").is_err());
    }
}
