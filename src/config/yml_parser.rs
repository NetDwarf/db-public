use serde::{Deserialize, Serialize};
use serde_yml;
use std::fs;

const CONFIG_FILE_PATH: &str = "./config/config.yml";

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigYml {
    pub db: DbCred,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub exportignore: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DbCred {
    Mysql(MySqlCredentials),
    Sqlite(SqliteCredentials),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MySqlCredentials {
    pub host: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SqliteCredentials {
    pub file_path: String,
}

impl ConfigYml {
    pub fn parse() -> ConfigYml {
        let config_yml_text = fs::read_to_string(CONFIG_FILE_PATH).unwrap();
        let config_yml: ConfigYml = serde_yml::from_str(config_yml_text.as_str()).unwrap();
        return config_yml;
    }
}
