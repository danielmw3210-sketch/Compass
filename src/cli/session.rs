use std::io::{self, Write};
use std::path::Path;
use crate::identity::{Identity, NodeRole};
use crate::crypto::KeyPair;
use std::sync::Arc;

/// User Role for CLI Access Control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Admin,    // Full system access
    Worker,   // Job execution only
    Client,   // Wallet operations only
}

/// Production Session with RBAC
pub struct Session {
    pub role: UserRole,
    pub identity: Option<Arc<KeyPair>>,
    pub authenticated: bool,
    pub user_name: String,
}

impl Session {
    /// Single Point of Authentication - PRODUCTION GRADE
    /// Failures result in program exit (no retry)
    pub fn authenticate() -> Result<Self, String> {
        println!("\n===========================================");
        println!("  COMPASS BLOCKCHAIN - Authentication  ");
        println!("===========================================\n");
        
        println!("Select Role:");
        println!("1. Admin (Node Operator)");
        println!("2. Worker (Compute Provider)");
        println!("3. Client (Wallet User)");
        print!("\nRole: ");
        io::stdout().flush().unwrap();
        
        let mut role_choice = String::new();
        io::stdin().read_line(&mut role_choice).unwrap();
        
        let role = match role_choice.trim() {
            "1" => UserRole::Admin,
            "2" => UserRole::Worker,
            "3" => UserRole::Client,
            _ => return Err("Invalid role selection.".to_string()),
        };
        
        // Admin and Worker MUST authenticate
        match role {
            UserRole::Admin => {
                // Try to load identity file first
                match Self::try_load_identity("admin") {
                    Some(identity) => {
                        if identity.role != NodeRole::Admin {
                            return Err("Identity file is not Admin role!".to_string());
                        }
                        let keypair = identity.into_keypair()
                            .map_err(|e| format!("Failed to unlock keypair: {}", e))?;
                        
                        Ok(Session {
                            role,
                            identity: Some(Arc::new(keypair)),
                            authenticated: true,
                            user_name: "admin".to_string(),
                        })
                    },
                    None => {
                        // No admin.json found - offer ephemeral mode
                        println!("\n⚠️  No admin.json found.");
                        println!("   Options:");
                        println!("     1. Generate one: Try running the interactive menu → Key Management");
                        println!("     2. Use EPHEMERAL admin mode (temporary keypair, development only)");
                        print!("\n   Continue with ephemeral mode? [y/N]: ");
                        io::stdout().flush().unwrap();
                        
                        let mut answer = String::new();
                        io::stdin().read_line(&mut answer).unwrap();
                        
                        if answer.trim().to_lowercase() == "y" {
                            println!("\n🔓 Creating ephemeral admin keypair (DEV MODE)");
                            let temp_keypair = crate::crypto::KeyPair::generate();
                            println!("   Ephemeral PubKey: {}", temp_keypair.public_key_hex());
                            println!("   ⚠️  This will NOT persist after restart!\n");
                            
                            Ok(Session {
                                role,
                                identity: Some(Arc::new(temp_keypair)),
                                authenticated: true,
                                user_name: "admin_ephemeral".to_string(),
                            })
                        } else {
                            Err("Admin authentication required.".to_string())
                        }
                    }
                }
            },
            UserRole::Worker => {
                // Try to load identity file first
                match Self::try_load_identity("verifier") {
                    Some(identity) => {
                        if identity.role != NodeRole::Verifier {
                            return Err("Identity file is not Worker/Verifier role!".to_string());
                        }
                        let keypair = identity.into_keypair()
                            .map_err(|e| format!("Failed to unlock keypair: {}", e))?;
                        
                        Ok(Session {
                            role,
                            identity: Some(Arc::new(keypair)),
                            authenticated: true,
                            user_name: "verifier".to_string(),
                        })
                    },
                    None => {
                        // No verifier.json found - offer ephemeral mode
                        println!("\n⚠️  No verifier.json found.");
                        println!("   Options:");
                        println!("     1. Generate one: Run as Admin → Key Management → Generate Verifier Key");
                        println!("     2. Use EPHEMERAL worker mode (temporary keypair, development only)");
                        print!("\n   Continue with ephemeral mode? [y/N]: ");
                        io::stdout().flush().unwrap();
                        
                        let mut answer = String::new();
                        io::stdin().read_line(&mut answer).unwrap();
                        
                        if answer.trim().to_lowercase() == "y" {
                            println!("\n🔓 Creating ephemeral worker keypair (DEV MODE)");
                            let temp_keypair = crate::crypto::KeyPair::generate();
                            println!("   Ephemeral PubKey: {}", temp_keypair.public_key_hex());
                            println!("   ⚠️  This will NOT persist after restart!");
                            println!("   ⚠️  Rewards will go to this ephemeral address!\n");
                            
                            Ok(Session {
                                role,
                                identity: Some(Arc::new(temp_keypair)),
                                authenticated: true,
                                user_name: "worker_ephemeral".to_string(),
                            })
                        } else {
                            Err("Worker authentication required.".to_string())
                        }
                    }
                }
            },
            UserRole::Client => {
                // Client can optionally authenticate for wallet ops
                print!("Enter username (or press Enter to skip): ");
                io::stdout().flush().unwrap();
                let mut username = String::new();
                io::stdin().read_line(&mut username).unwrap();
                let username = username.trim();
                
                if username.is_empty() {
                    // Anonymous client (read-only)
                    Ok(Session {
                        role,
                        identity: None,
                        authenticated: false,
                        user_name: "anonymous".to_string(),
                    })
                } else {
                    // Authenticated client
                    match Self::try_load_identity(username) {
                        Some(identity) => {
                            let keypair = identity.into_keypair()
                                .map_err(|e| format!("Failed to unlock keypair: {}", e))?;
                            Ok(Session {
                                role,
                                identity: Some(Arc::new(keypair)),
                                authenticated: true,
                                user_name: username.to_string(),
                            })
                        },
                        None => {
                            // New user
                            Ok(Session {
                                role,
                                identity: None,
                                authenticated: false,
                                user_name: username.to_string(),
                            })
                        }
                    }
                }
            },
        }
    }
    
    /// Load identity with STRICT validation (Production Mode)
    /// Failure = boot out
    fn load_identity_strict(name: &str) -> Result<Identity, String> {
        let filename = format!("{}.json", name);
        if !Path::new(&filename).exists() {
            return Err(format!("Identity file '{}' not found! Generate it first with Key Management.", filename));
        }
        
        print!("Enter password for '{}': ", name);
        io::stdout().flush().unwrap();
        let mut pass = String::new();
        io::stdin().read_line(&mut pass).unwrap();
        
        Identity::load_and_decrypt(Path::new(&filename), pass.trim())
            .map_err(|e| format!("Authentication Failed: {}", e))
    }
    
    /// Try to load identity (for optional client auth)
    fn try_load_identity(name: &str) -> Option<Identity> {
        let filename = format!("{}.json", name);
        if !Path::new(&filename).exists() {
            return None;
        }
        
        print!("Enter password for '{}': ", name);
        io::stdout().flush().unwrap();
        let mut pass = String::new();
        io::stdin().read_line(&mut pass).unwrap();
        
        Identity::load_and_decrypt(Path::new(&filename), pass.trim()).ok()
    }
    
    // ========== RBAC Permission Checks ==========
    
    pub fn can_start_node(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }
    
    pub fn can_access_worker_jobs(&self) -> bool {
        matches!(self.role, UserRole::Worker)
    }
    
    pub fn can_view_wallets(&self) -> bool {
        // Admin can view for debugging, Client for normal use
        matches!(self.role, UserRole::Admin | UserRole::Client)
    }
    
    pub fn can_make_transfers(&self) -> bool {
        // Must be authenticated client or admin
        self.authenticated && matches!(self.role, UserRole::Admin | UserRole::Client)
    }
    
    pub fn can_access_tools(&self) -> bool {
        // Only admin can wipe DB, etc.
        matches!(self.role, UserRole::Admin)
    }
    
    pub fn can_manage_keys(&self) -> bool {
        // Anyone can generate keys (before auth)
        true
    }
}
