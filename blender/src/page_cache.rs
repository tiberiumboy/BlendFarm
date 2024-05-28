use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct PageCache {
    cache: HashMap<Url, PathBuf>,
}

const CACHE_DIR: &str = "cache";
const CACHE_CONFIG: &str = "cache.json";

impl PageCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    // fetch the directory
    fn get_dir() -> PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.push(CACHE_DIR);
        if !tmp.exists() {
            fs::create_dir(&tmp).expect("Unable to create directory! Permission issue?");
        }
        tmp
    }

    fn create() -> Self {
        let obj = Self::new();
        obj.save();
        obj
    }

    // private method, only used to save when cache has changed.
    fn save(&self) {
        // it may seems like this is a bad idea but I would expect this function to work either way?
        // Wonder if this is the best practice?
        let data = serde_json::to_string(&self).expect("Unable to deserialize data!");
        let path = Self::get_dir().join(CACHE_CONFIG);
        fs::write(path, &data);
    }

    pub fn load() -> Self {
        let path = Self::get_dir().join(CACHE_CONFIG);
        match fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).expect("Unable to parse content!"),
            Err(_) => Self::create(),
        }
    }

    /// check and see if the url matches the cache,
    /// otherwise, fetch the page from the internet, and save it to storage cache,
    /// then return the page result.
    /// Otherwise if page is inaccessible - None will be returned instead.
    pub fn fetch(&mut self, url: &Url) -> Option<String> {
        let path = match self.cache.get(&url) {
            // if we are unable to find the url that we have previous cached, then we need to create a new entry.
            // after we append that entry, we need to save it to the file somewhere.
            Some(path) => path.to_owned(),
            None => {
                let mut tmp = Self::get_dir();
                let re = Regex::new(r#"[/\\?%*:|."<>]"#).unwrap();
                let mut url_name = format!("{}{}", url.host().unwrap(), url.path());
                if url_name.ends_with('/') {
                    url_name.pop();
                }
                let file_name = re.replace_all(&url_name, "-").to_string();
                tmp.push(file_name);

                let content = match reqwest::blocking::get(url.clone()) {
                    Ok(data) => data.text().unwrap(),
                    Err(_) => return None,
                };
                fs::write(&tmp, content).unwrap();
                self.cache.insert(url.clone(), tmp.clone());
                self.save();
                tmp
            }
        };

        match fs::read_to_string(path) {
            Ok(data) => Some(data),
            Err(_) => None, // usually it means that there were no data to load?
        }
    }

    pub fn fetch_str(&mut self, url: &str) -> Option<String> {
        let url = Url::parse(url).unwrap();
        self.fetch(&url)
    }
}
