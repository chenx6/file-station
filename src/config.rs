use std::collections::HashMap;
use std::env::vars;
use std::fs::{canonicalize, create_dir};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// FS_FOLDER
    pub folder_path: PathBuf,
    /// FS_DATABASE
    pub database_path: String,
    /// FS_LISTEN
    pub listen_addr: String,
    /// FS_REGISTER
    pub can_register: bool,
    /// FS_SALT
    pub salt: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            folder_path: "./files".into(),
            database_path: "./database.db".into(),
            listen_addr: "127.0.0.1:5000".into(),
            can_register: true,
            salt: "AAAABBBBCCCCDDDD".into(),
        }
    }

    /// Get config from environment.
    /// This function will panic if environment variable is wrong.
    pub fn from_env() -> Config {
        let e: HashMap<String, String> = HashMap::from_iter(vars());
        let mut folder_path = PathBuf::from(e.get("FS_FOLDER").unwrap_or(&"./files".into()));
        // If folder doesn't exist, create it
        if !folder_path.exists() {
            create_dir(&folder_path).unwrap();
        }
        folder_path = canonicalize(folder_path).unwrap();
        let database_path = e
            .get("FS_DATABASE")
            .unwrap_or(&"./database.db".into())
            .clone();
        let listen_addr = e
            .get("FS_LISTEN")
            .unwrap_or(&"127.0.0.1:5000".into())
            .clone();
        let can_register = e.get("FS_REGISTER").unwrap_or(&"TRUE".into()) == "TRUE";
        let salt = e
            .get("FS_SALT")
            .unwrap_or(&"AAAABBBBCCCCDDDD".into())
            .clone();

        Config {
            folder_path,
            database_path,
            listen_addr,
            can_register,
            salt,
        }
    }
}
