use crate::constant::MAX_VALID_DAYS;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{Error, Read, Result};
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};
use url::Url;

// Hide this for now,
#[doc(hidden)]
// rely the cache creation date on file metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PageCache {
    cache: HashMap<Url, PathBuf>,
    was_modified: bool,
}

impl PageCache {
    // fetch cache directory
    fn get_dir() -> Result<PathBuf> {
        // TODO: What should happen if I can't fetch cache_dir()?
        let mut tmp = dirs::cache_dir().ok_or(Error::new(
            std::io::ErrorKind::NotFound,
            "Unable to fetch cache directory!",
        ))?;
        tmp.push("cache");
        fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    // fetch path to cache file
    fn get_cache_path() -> Result<PathBuf> {
        Ok(Self::get_dir()?.join("cache.json"))
    }

    // private method, only used to save when cache has changed.
    fn save(&mut self) -> Result<()> {
        self.was_modified = false;
        let data = serde_json::to_string(&self).expect("Unable to deserialize data!");
        fs::write(Self::get_cache_path()?, data)?;
        Ok(())
    }

    // TODO: Impl a way to verify cache is not old or out of date. What's a good refresh cache time? 2 weeks? server_settings config?
    pub fn load() -> Result<Self> {
        let current = SystemTime::now();
        // use define path to cache file
        let path = Self::get_cache_path()?;
        let fallback = SystemTime::now();
        let data = fs::metadata(&path);
        let created_date = match data {
            Ok(m) => m
                .is_file()
                .then(|| m.created().unwrap_or(fallback))
                .unwrap_or_else(|| fallback),
            _ => fallback,
        };

        let data = match current.duration_since(created_date) {
            Ok(duration) if duration.as_secs() < MAX_VALID_DAYS * 3600 * 24 => {
                println!(
                    "Time still valid: Remaining {}hrs",
                    duration.as_secs() / 3600 - (MAX_VALID_DAYS * 24)
                );
                match fs::read_to_string(path) {
                    Ok(data) => serde_json::from_str(&data).unwrap_or(Self::default()),
                    _ => Self::default(),
                }
            }
            _ => Self::default(),
        };

        Ok(data)
    }

    // This function can be relocated somewhere else?
    fn generate_file_name(url: &Url) -> String {
        let mut file_name = url.to_string();

        // Rule: find any invalid file name characters
        // TODO: Is there a way to make this shared statically? Doesn't seems like it's being used anywhere?
        let re = Regex::new(r#"[/\\?%*:|."<>]"#).unwrap();

        // remove trailing slash
        file_name.ends_with('/').then(|| file_name.pop());

        // Replace any invalid characters with hyphens
        re.replace_all(&file_name, "-").to_string()
    }

    /// Fetch url response from argument and save response body to cache directory using url as file name
    /// This will append a new entry to the cache hashmap.
    fn save_content_to_cache(url: &Url) -> Result<PathBuf> {
        // create an absolute file path
        let mut tmp = Self::get_dir()?;
        tmp.push(Self::generate_file_name(url));

        // fetch the content from the url
        // expensive implict type cast?
        let mut response = ureq::get(url.as_ref()).call().map_err(Error::other)?;
        let mut body = Vec::new();
        if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
            eprintln!("Fail to read data for cache: {e:?}");
        }

        // write the content to the file
        fs::write(&tmp, body)?;
        Ok(tmp)
    }

    /// check and see if the url matches the cache,
    /// otherwise, fetch the page from the internet, and save it to storage cache,
    /// then return the page result.
    pub fn fetch(&mut self, url: &Url) -> Result<String> {
        let path = self.cache.entry(url.to_owned()).or_insert({
            self.was_modified = true;
            Self::save_content_to_cache(url)?.to_owned()
        });

        fs::read_to_string(path)
    }

    // TODO: Maybe this isn't needed, but would like to know if there's a better way to do this? Look into IntoUrl?
    pub fn fetch_str(&mut self, url: &str) -> Result<String> {
        let url = Url::parse(url).unwrap();
        self.fetch(&url)
    }
}

impl Drop for PageCache {
    fn drop(&mut self) {
        if self.was_modified {
            if let Err(e) = self.save() {
                println!("Error saving cache file: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_pass() {
        let cache = PageCache::load();
        assert_eq!(cache.is_ok(), true);
        let mut cache = cache.unwrap();
        let url = Url::parse("http://www.google.com").unwrap();
        let content = cache.fetch(&url);
        assert_eq!(content.is_ok(), true);
    }

    #[test]
    fn should_fail() {
        todo!();
    }
}
