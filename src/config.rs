use std::cell::Cell;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tokio::{fs, io, select};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Configuration
{
    #[serde(rename = "berry-name", alias = "berryName", alias = "Berry Name", alias = "name", alias = "Name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub berry_name: Option<String>,
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

#[derive(Debug)]
pub enum ReadError
{
    Io(io::Error),
    JsonLenient(serde_json_lenient::Error),
    Yaml(serde_yaml::Error),
}

impl fmt::Display for ReadError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            ReadError::Io(x) => x.fmt(f),
            ReadError::JsonLenient(x) => x.fmt(f),
            ReadError::Yaml(x) => x.fmt(f),
        }
    }
}

impl From<io::Error> for ReadError
{
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

impl From<serde_json_lenient::Error> for ReadError
{
    fn from(value: serde_json_lenient::Error) -> Self { Self::JsonLenient(value) }
}

impl From<serde_yaml::Error> for ReadError
{
    fn from(value: serde_yaml::Error) -> Self { Self::Yaml(value) }
}

pub async fn read_config(dir_path: &Path) -> Result<Configuration, heapless::Vec<(&'static str, ReadError), 3>>
{
    let json_error = Cell::new(None);
    let jsonc_error = Cell::new(None);
    let yaml_error = Cell::new(None);

    select!
    {
        Ok(config) = async
        {
            match fs::read(dir_path.join("config.json")).await
            {
                Ok(file) =>
                {
                    match serde_json_lenient::from_slice(&file)
                    {
                        Ok(x) => Ok(x),
                        Err(x) => Err(json_error.set(Some(x.into()))),
                    }
                }
                Err(error) =>
                {
                    match error.kind()
                    {
                        ErrorKind::NotFound => Err(()),
                        _ => Err(json_error.set(Some(error.into()))),
                    }
                },
            }
        }
        => Ok(config),
        Ok(config) = async
        {
            match fs::read(dir_path.join("config.jsonc")).await
            {
                Ok(file) =>
                {
                    match serde_json_lenient::from_slice(&file)
                    {
                        Ok(x) => Ok(x),
                        Err(x) => Err(json_error.set(Some(x.into()))),
                    }
                }
                Err(error) =>
                {
                    match error.kind()
                    {
                        ErrorKind::NotFound => Err(()),
                        _ => Err(json_error.set(Some(error.into()))),
                    }
                },
            }
        }
        => Ok(config),
        Ok(config) = async
        {
            match fs::read(dir_path.join("config.yaml")).await
            {
                Ok(file) =>
                {
                    match serde_yaml::from_slice(&file)
                    {
                        Ok(x) => Ok(x),
                        Err(x) => Err(json_error.set(Some(x.into()))),
                    }
                }
                Err(error) =>
                {
                    match error.kind()
                    {
                        ErrorKind::NotFound => Err(()),
                        _ => Err(json_error.set(Some(error.into()))),
                    }
                },
            }
        }
        => Ok(config),
        else =>
        {
            let mut errors = heapless::Vec::new();

            if let Some(x) = json_error.into_inner()
            {
                errors.push(("config.json", x)).ok().unwrap();
            }

            if let Some(x) = jsonc_error.into_inner()
            {
                errors.push(("config.jsonc", x)).ok().unwrap();
            }

            if let Some(x) = yaml_error.into_inner()
            {
                errors.push(("config.yaml", x)).ok().unwrap();
            }

            Err(errors)
        },
    }
}