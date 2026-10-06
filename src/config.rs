use std::io::ErrorKind;
use std::path::PathBuf;

use serde::Deserialize;
use tokio::fs;
use tokio::sync::OnceCell;

use crate::{CURRENT_DIR, log::*};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Configuration
{
    #[serde(rename = "name", alias = "Name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "logo-path", alias = "logoPath", alias = "Logo Path")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_path: Option<PathBuf>,
    #[serde(rename = "simple-logo-path", alias = "simpleLogoPath", alias = "Simple Logo Path")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple_logo_path: Option<PathBuf>,
    #[serde(rename = "listen-port", alias = "listenPort", alias = "Listen Port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,
    #[serde(alias = "broadcast-port", alias = "broadcastPort", alias = "Broadcast Port")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcast_port: Option<u16>,
}

pub async fn poll_config() -> Option<Configuration>
{
    static PATH_JSON: OnceCell<PathBuf> = OnceCell::const_new();

    let path_json = PATH_JSON.get_or_init(async ||
    {
        let mut path = CURRENT_DIR.get().unwrap().clone();
        path.extend(["config.json"]);
        path
    })
        .await;

    match fs::read(path_json).await
    {
        Ok(file) =>
        {
            match serde_json_lenient::from_slice(&file)
            {
                Ok(x) => return Some(x),
                Err(error) => eprintln!("{Warn}: Failed to read configuration file: {}.", error),
            }
        }
        Err(error) =>
        {
            match error.kind()
            {
                ErrorKind::NotFound => (),
                _ => eprintln!("{Warn}: Failed to find configuration file: {}.", error),
            }
        },
    }

    static PATH_JSONC: OnceCell<PathBuf> = OnceCell::const_new();

    let path_jsonc = PATH_JSONC.get_or_init(async ||
    {
        let mut path = CURRENT_DIR.get().unwrap().clone();
        path.extend(["config.jsonc"]);
        path
    })
        .await;

    match fs::read(path_jsonc).await
    {
        Ok(file) =>
        {
            match serde_json_lenient::from_slice(&file)
            {
                Ok(x) => return Some(x),
                Err(error) => eprintln!("{Warn}: Failed to read configuration file: {}.", error),
            }
        }
        Err(error) =>
        {
            match error.kind()
            {
                ErrorKind::NotFound => (),
                _ => eprintln!("{Warn}: Failed to find configuration file: {}.", error),
            }
        },
    }

    static PATH_YAML: OnceCell<PathBuf> = OnceCell::const_new();

    let path_yaml = PATH_YAML.get_or_init(async ||
    {
        let mut path = CURRENT_DIR.get().unwrap().clone();
        path.extend(["config.yaml"]);
        path
    })
        .await;

    match fs::read(path_yaml).await
    {
        Ok(file) =>
        {
            match serde_yaml::from_slice(&file)
            {
                Ok(x) => return Some(x),
                Err(error) => eprintln!("{Warn}: Failed to read configuration file: {}.", error),
            }
        }
        Err(error) =>
        {
            match error.kind()
            {
                ErrorKind::NotFound => (),
                _ => eprintln!("{Warn}: Failed to find configuration file: {}.", error),
            }
        },
    }

    eprintln!("{Warn}: No configuration file found, using default configuration.");

    None
}