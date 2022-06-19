use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    NetworkError(#[from] reqwest::Error),

    #[error("Cannot parse the response")]
    DeserializeFailed,
}

type BundleMap = HashMap<String, Bundle>;

#[derive(Debug, Deserialize)]
pub struct Bundle {
    pub gamekey: String,

    #[serde(rename = "product")]
    pub details: BundleDetails,

    #[serde(rename = "subproducts")]
    pub entries: Vec<Product>,
}

#[derive(Debug, Deserialize)]
pub struct BundleDetails {
    pub machine_name: String,
    pub human_name: String,
}

impl Bundle {
    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.total_size()).sum()
    }
}

#[derive(Debug, Deserialize)]
pub struct Product {
    pub machine_name: String,
    pub human_name: String,

    #[serde(rename = "url")]
    pub product_details_url: String,

    pub downloads: Vec<DownloadEntry>,
}

impl Product {
    pub fn total_size(&self) -> u64 {
        self.downloads.iter().map(|e| e.total_size()).sum()
    }

    pub fn formats_as_vec(&self) -> Vec<String> {
        self.downloads
            .iter()
            .map(|d| d.formats_as_vec())
            .flatten()
            .collect::<Vec<_>>()
    }

    pub fn formats(&self) -> String {
        self.formats_as_vec().join(", ")
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadEntry {
    #[serde(rename = "download_struct")]
    pub sub_items: Vec<DownloadEntryItem>,
}

impl DownloadEntry {
    pub fn total_size(&self) -> u64 {
        self.sub_items.iter().map(|e| e.file_size).sum()
    }

    pub fn formats_as_vec(&self) -> Vec<String> {
        self.sub_items
            .iter()
            .map(|s| s.item_type.clone())
            .collect::<Vec<_>>()
    }

    pub fn formats(&self) -> String {
        self.formats_as_vec().join(", ")
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadEntryItem {
    pub md5: String,

    #[serde(rename = "name")]
    pub item_type: String,

    pub file_size: u64,

    pub url: DownloadUrl,
}

#[derive(Debug, Deserialize)]
pub struct DownloadUrl {
    pub web: String,
    pub bittorrent: String,
}

#[derive(Debug, Deserialize)]
struct GameKey {
    gamekey: String,
}

pub struct HumbleApi {
    auth_key: String,
}

impl HumbleApi {
    pub fn new(auth_key: &str) -> Self {
        Self {
            auth_key: auth_key.to_owned(),
        }
    }

    pub fn list_bundles(&self) -> Result<Vec<Bundle>, ApiError> {
        let client = Client::new();

        // First: get the game keys
        let res = client
            .get("https://www.humblebundle.com/api/v1/user/order")
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                "cookie".to_owned(),
                format!("_simpleauth_sess={}", self.auth_key),
            )
            .send()?
            .error_for_status()?;

        let game_keys = res.json::<Vec<GameKey>>()?;

        // Second: get details for those game keys
        let query_params: Vec<_> = game_keys
            .into_iter()
            .map(|g| ("gamekeys", g.gamekey))
            .collect();

        let res = client
            .get("https://www.humblebundle.com/api/v1/orders")
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                "cookie".to_owned(),
                format!("_simpleauth_sess={}", self.auth_key),
            )
            .query(&query_params)
            .send()?
            .error_for_status()?;

        let product_map = res.json::<BundleMap>()?;
        Ok(product_map.into_values().collect())
    }

    pub fn read_bundle(&self, product_key: &str) -> Result<Bundle, ApiError> {
        let url = format!("https://www.humblebundle.com/api/v1/order/{}", product_key);

        let client = Client::new();
        let res = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                "cookie".to_owned(),
                format!("_simpleauth_sess={}", self.auth_key),
            )
            .send()?
            .error_for_status()?;

        res.json::<Bundle>()
            .map_err(|_| ApiError::DeserializeFailed)
    }
}
