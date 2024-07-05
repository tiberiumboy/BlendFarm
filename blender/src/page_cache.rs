use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};
use thiserror::Error;
use url::Url;

// Hide this for now,
#[doc(hidden)]
// rely the cache creation date on file metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PageCache {
    cache: HashMap<Url, PathBuf>,
    was_modified: bool,
}

#[derive(Debug, Error)]
pub enum PageCacheError {
    #[error("Cache directory does not exist!")]
    CacheDirNotFound,
    #[error("Unable to create cache directory at `{path}`!")]
    CannotCreate { path: PathBuf },
    #[error("IO Error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("Reqwest Error: {source}")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
}

// consider using directories::UserDirs::document_dir()
const CACHE_DIR: &str = "cache";
const CACHE_CONFIG: &str = "cache.json";

impl PageCache {
    // fetch cache directory
    fn get_dir() -> Result<PathBuf, PageCacheError> {
        // TODO: What should happen if I can't fetch cache_dir()?
        let mut tmp = dirs::cache_dir().ok_or(PageCacheError::CacheDirNotFound)?;
        tmp.push(CACHE_DIR);
        if fs::create_dir_all(&tmp).is_err() {
            Err(PageCacheError::CannotCreate { path: tmp })
        } else {
            Ok(tmp)
        }
    }

    // fetch path to cache file
    fn get_cache_path() -> Result<PathBuf, PageCacheError> {
        match Self::get_dir() {
            Ok(path) => Ok(path.join(CACHE_CONFIG)),
            Err(e) => Err(e),
        }
    }

    // private method, only used to save when cache has changed.
    fn save(&mut self) -> Result<(), PageCacheError> {
        self.was_modified = false;
        let data = serde_json::to_string(&self).expect("Unable to deserialize data!");
        match Self::get_cache_path() {
            Ok(path) => match fs::write(path, data) {
                Ok(_) => {
                    println!("Successfully saved cache file!");
                    Ok(())
                }
                Err(e) => Err(e.into()),
            },
            Err(e) => Err(e),
        }
    }

    // TODO: Impl a way to verify cache is not old or out of date. What's a good refresh cache time? 2 weeks? server_settings config?
    pub fn load(expiration: SystemTime) -> Result<Self, PageCacheError> {
        // use define path to cache file
        let path = Self::get_cache_path()?;
        let created_date = match fs::metadata(&path) {
            Ok(metadata) => {
                if metadata.is_file() {
                    println!("Cache file found! Fetching metadata creation date property!");
                    metadata.created().unwrap_or(SystemTime::now())
                } else {
                    println!("Unable to find cache file, creating new one!");
                    SystemTime::now()
                }
            }
            Err(_) => SystemTime::now(),
        };

        let data = match expiration.duration_since(created_date) {
            Ok(_) => match fs::read_to_string(path) {
                Ok(data) => serde_json::from_str(&data).unwrap_or(Self::default()),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        };

        Ok(data)
    }

    // This function can be relocated somewhere else?
    fn generate_file_name(url: &Url) -> String {
        let mut file_name = url.to_string();

        // Rule: find any invalid file name characters
        let re = Regex::new(r#"[/\\?%*:|."<>]"#).unwrap();

        // remove trailing slash
        if file_name.ends_with('/') {
            file_name.pop();
        }

        // Replace any invalid characters with hyphens
        re.replace_all(&file_name, "-").to_string()
    }

    /// Fetch url response from argument and save response body to cache directory using url as file name
    /// This will append a new entry to the cache hashmap.
    fn save_content_to_cache(url: &Url) -> Result<PathBuf, PageCacheError> {
        // create an absolute file path
        let mut tmp = Self::get_dir()?;
        tmp.push(Self::generate_file_name(url));

        // fetch the content from the url
        let response = reqwest::blocking::get(url.to_string())?;
        let content = response.text()?;

        // write the content to the file
        fs::write(&tmp, content)?;
        Ok(tmp)
    }

    /// check and see if the url matches the cache,
    /// otherwise, fetch the page from the internet, and save it to storage cache,
    /// then return the page result.
    pub fn fetch(&mut self, url: &Url) -> Result<String, PageCacheError> {
        let path = self.cache.entry(url.to_owned()).or_insert({
            self.was_modified = true;
            Self::save_content_to_cache(url)?.to_owned()
        });

        match fs::read_to_string(path) {
            Ok(data) => Ok(data),
            Err(e) => Err(e.into()),
        }
    }

    // TODO: Maybe this isn't needed, but would like to know if there's a better way to do this? Look into IntoUrl?
    pub fn fetch_str(&mut self, url: &str) -> Result<String, PageCacheError> {
        let url = Url::parse(url).unwrap();
        self.fetch(&url)
    }
}

impl Drop for PageCache {
    fn drop(&mut self) {
        if self.was_modified {
            self.save().unwrap();
        }
    }
}
