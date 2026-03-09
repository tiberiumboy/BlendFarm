use crate::constant::MAX_VALID_DAYS;
use lazy_regex::regex_replace_all;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, ErrorKind, Error, Read, Result};
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ExpirationUnits {
    Disable,
    Day(i8),
    Week(i8),
    Month(i8),
    // Year(i8),
}

impl Default for ExpirationUnits {
    fn default() -> Self {
        ExpirationUnits::Month(6)
    }
}

// Unless PageCache manages this internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageCacheConfiguration {
    expiration_duration: ExpirationUnits,
    cache_dir: PathBuf,
    config_path: PathBuf,
}

impl Default for PageCacheConfiguration {
    fn default() -> Self {
        let cache_dir = PageCache::get_default_dir().expect("Must have access to cache directory");
        let config_path = PageCache::get_cache_path().expect("Must have access to cache dir");

        Self { 
            expiration_duration: Default::default(), 
            cache_dir, 
            config_path 
        }
    }
}

// Hide this for now,
#[doc(hidden)]
// rely the cache creation date on file metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PageCache {
    cache: HashMap<Url, PathBuf>,
    // TODO: consider replacing this to something else.
    was_modified: bool,
    config: PageCacheConfiguration,
}

// the whole idea behind this was to store information from blender with minimal connectivity
// interface as possible. Rely on cache if we need to lookup again. This separate us from ChatGPT and other LLM agents.
impl PageCache {
    const CACHE_DIR: &str = "cache";
    const CONFIG_NAME: &str = "cache.json";

    // fetch cache directory
    fn get_default_dir() -> Result<PathBuf> {
        let mut tmp = dirs::cache_dir().ok_or(Error::new(
            ErrorKind::NotFound,
            "Unable to fetch cache directory! Must have permission to create cache directory!",
        ))?;
        // append our program folder name.
        tmp.push(Self::CACHE_DIR);
        // ensure directory exist and created.
        fs::create_dir_all(&tmp).and(Ok(tmp))
    }

    // fetch path to cache file
    #[inline]
    fn get_cache_path() -> Result<PathBuf> {
        Ok(Self::get_default_dir()?.join(Self::CONFIG_NAME))
    }

    // private method, only used to save when cache has changed.
    fn save(&mut self) -> Result<()> {
        if !self.was_modified {
            return Ok(());
        }

        let data = serde_json::to_string(&self)?;
        fs::write(Self::get_cache_path()?, data)?;
        self.was_modified = false;
        Ok(())
    }

    #[allow(dead_code)]
    fn validate_cache(&mut self) {
        // Here we run a check of all of the cache we have stored, and then check the last modified date. If it exceed page cache's
        // TODO: Present a "Delete cache after X Y" Where X is a number and Y is enum such as Day, Weeks, or Month - We should be realistic, protective, and caution about security and delete cache older than 6 months, unless someone objects this idea and creates a PR request removing this comment and prove me wrong why we should store cache older than a year? At this point, you might as well just turn off this feature?
        // PageCacheConfig::get_expiration_duration(self) -> Option<ExpirationUnits>
    }

    /* 
    // for future project, consider stream io input instead of read_to_string();
    
    fn read_skipping_ws(mut reader: impl Read) -> Result<u8> {
        loop {
            let mut byte = 0u8;
            reader.read_exact(std::slice::from_mut(&mut byte))?;
            if !byte.is_ascii_whitespace() {
                return Ok(byte);
            }
        }
    }

    #[inline]
    fn invalid_data(msg: &str) -> Error {
        Error::new(ErrorKind::InvalidData, msg)
    }

    fn deserialize_single<T: DeserializeOwned, R: Read> (reader: R) -> Result<T> {
        let next_obj = Deserializer::from_reader(reader).into_iter::<T>().next();
        match next_obj {
            Some(result) => result.map_err(Into::into),
            None => Err(Self::invalid_data("premature EOF")),
        }
    }

    fn yield_next_obj<T: DeserializeOwned, R: Read> (
        mut reader: R,
        at_start: &mut bool,
    ) -> Result<Option<T>> {
        if !*at_start {
            *at_start = true;
            if Self::read_skipping_ws(&mut reader)? == b'[' {
                let peek = Self::read_skipping_ws(&mut reader)?;
                if peek == b']' {
                    Ok(None)
                } else {
                    // we're creating new cursor each yield objects?
                    let obj = Self::deserialize_single(io::Cursor::new([peek]).chain(reader))?;
                    Ok(Some(obj))
                }
            } else {
                Err(Self::invalid_data("`[` not found"))
            }
        } else {
            match Self::read_skipping_ws(&mut reader)? {
                b',' => Self::deserialize_single(reader).map(Some),
                b']' => Ok(None),
                _ => Err(Self::invalid_data("`,` or `]` not found")),
            }
        }
    }

    fn iter_json_array<T: DeserializeOwned, R: Read>(
        mut reader: R,
    ) -> impl Iterator<Item = Result<T>> {
        let mut at_start = false;
        std::iter::from_fn(move || Self::yield_next_obj(&mut reader, &mut at_start).transpose())
    }

    */

    // TODO: name is too ambiguous. What is load? What are we loading? What does it do? Does it load the program? File? Something?
    pub fn load() -> Result<Self> {
        let current = SystemTime::now();
        // use define path to cache file
        let path = Self::get_cache_path()?;
        let fallback = SystemTime::now();
        // read the metadata of the cache.json file.
        let data = fs::metadata(&path);
        // if the creation date is beyond the configuration expiration rule, we should delete the file and refresh from the source of truth.
        let created_date = match data {
            Ok(m) => m
                .is_file()
                .then(|| m.created().unwrap_or(fallback))
                .unwrap_or_else(|| fallback),
            _ => fallback,
        };

        // if file exist and provides duration date.
        if let Ok(duration) = current.duration_since(created_date) {
            // must be within valid window timeframe.
            if duration.as_secs() < MAX_VALID_DAYS * 3600 * 24 {
                // logger
                println!(
                    "Time still valid: Remaining {}hrs",
                    duration.as_secs() / 3600 - (MAX_VALID_DAYS * 24)
                );
                let reader = BufReader::new(fs::File::open(path)?);
                return Ok(serde_json::from_reader(reader)?)
            }
        }
        Ok(Self::default())
    }

    fn generate_file_name(url: &Url) -> String {
        let mut file_name = url.to_string();
        // Rule: find any invalid file name characters
        // remove trailing slash
        file_name.ends_with('/').then(|| file_name.pop());
        // Replace any invalid characters with hyphens
        regex_replace_all!(r#"[/\\?%*:|."<>]"#, &file_name, "-").to_string()
    }

    /// check and see if the url matches the cache,
    /// otherwise, fetch the page from the internet, and save it to storage cache,
    /// then return the page result.
    pub fn fetch_or_update(&mut self, url: &Url) -> Result<String> {
        
        // TODO can we avoid using to_owned()?
        let path = self.cache.entry(url.clone()).or_insert( {
                let file_name = Self::generate_file_name( url );
                let destination_path = self.config.cache_dir.join(file_name);

                // Are we making the assumption that if the file is not in the entry then we can just presume it's valid?
                if !destination_path.exists() {
                    let mut response = ureq::get(url.as_ref()).call().map_err(Error::other)?;
                    let mut body = Vec::new();
                    if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                        eprintln!("Fail to read data for cache: {e:?}");
                    }
                    
                    // write the content to the file
                    fs::write(&destination_path, body)?;
                }
                
                destination_path    
            });
            
        fs::read_to_string(path)
    }

    pub fn fetch(self, url: &Url) -> Option<String> {
        let path = self.cache.get(url)?;
        fs::read_to_string(path).ok()
    }

    // TODO: Maybe this isn't needed, but would like to know if there's a better way to do this? Look into IntoUrl?
    // pub fn fetch_str(&mut self, url: &str) -> Result<String> {
    //     let url = Url::parse(url).unwrap();
    //     self.fetch(&url)
    // }
}

impl Drop for PageCache {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            println!("Error saving cache file: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // This automation test does not make a lot of sense at all. It should be per each function callings.
    #[test]
    fn should_pass() {
        let cache = PageCache::load();
        assert!(cache.is_ok());
        let mut cache = cache.unwrap();
        let url = Url::parse("http://www.google.com").unwrap();
        let content = cache.fetch_or_update(&url);
        assert_eq!(content.is_ok(), true);
    }

    #[test]
    fn should_fail() {
        // TODO: How can I fail page_cache?
        // - lack of permission for directory asking to store and save web contents.
        // - logic condition inside Drop method scope. We try to invoke some Io operation on drop. Discouraging? Maybe?
        // - fetch_str rely on url parsing.
        let cache = PageCache::load();
        assert!(cache.is_ok());
    }

    // TODO: write unit test for get_dir()
    #[test]
    fn get_dir_succeed() {
        let cache = PageCache::get_default_dir();
        assert!(cache.is_ok());
    }
}
