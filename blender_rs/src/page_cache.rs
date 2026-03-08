use crate::constant::MAX_VALID_DAYS;
use lazy_regex::regex_replace;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::io::{self, Error, ErrorKind, Read, Result};
use std::os::fd::AsFd;
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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PageCacheConfiguration {
    expiration_duration: ExpirationUnits,
    cache_dir: PathBuf,
    config_path: PathBuf,
}

// Hide this for now,
#[doc(hidden)]
// rely the cache creation date on file metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PageCache {
    cache: HashMap<Url, PathBuf>,
    was_modified: bool,
    config: PageCacheConfiguration,
}

// the whole idea behind this was to store information from blender with minimal connectivity
// interface as possible. Rely on cache if we need to lookup again. This separate us from ChatGPT and other LLM agents.
impl PageCache {
    const CACHE_DIR: &str = "cache";
    const CONFIG_NAME: &str = "cache.json";

    // fetch cache directory
    // TODO: rename me to "get_default_dir()"
    fn get_dir() -> Result<PathBuf> {
        // FIXME: Consider using some kind of system settings to load where to save the cache to.
        let mut tmp = dirs::cache_dir().ok_or(Error::new(
            std::io::ErrorKind::NotFound,
            "Unable to fetch cache directory! Must have permission to create cache directory!",
        ))?;
        // append our program folder name.
        tmp.push(Self::CACHE_DIR);
        // ensure directory exist and created.
        fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    // fetch path to cache file
    fn get_cache_path() -> Result<PathBuf> {
        Ok(Self::get_dir()?.join(Self::CONFIG_NAME))
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

    fn read_skipping_ws(mut reader: impl Read) -> Result<u8> {
        loop {
            let mut byte = 0u8;
            reader.read_exact(std::slice::from_mut(&mut byte))?;
            if !byte.is_ascii_whitespace() {
                return Ok(byte);
            }
        }
    }

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

        let data = match current.duration_since(created_date) {
            Ok(duration) if duration.as_secs() < MAX_VALID_DAYS * 3600 * 24 => {
                println!(
                    "Time still valid: Remaining {}hrs",
                    duration.as_secs() / 3600 - (MAX_VALID_DAYS * 24)
                );
                // is there a way to stream it instead?

                let reader = fs::File::open(path)?;
                reader.read(Self::iter_json_array)?;
                fs::read(path)



                if let Ok(data) = fs::read_to_string(path) {
                    return serde_json::from_str(&data).map_or(Self::default(), |f| {

                    });
                }
                Self::default()
            }
            _ => Self::default(),
        };

        Ok(data)
    }

    fn generate_file_name(url: &Url) -> String {
        let mut file_name = url.to_string();
        // Rule: find any invalid file name characters
        // remove trailing slash
        file_name.ends_with('/').then(|| file_name.pop());
        // Replace any invalid characters with hyphens
        regex_replace!(r#"[/\\?%*:|."<>]"#, &file_name, "-").to_string()
    }

    // I often wonder if there was any need to return Unit. I think it'd be a lot better if it return something in principle.
    // pub fn update<T: Into<str>>(&mut self, url: &Url, content: T) -> Result<()> {

    // }

    /// check and see if the url matches the cache,
    /// otherwise, fetch the page from the internet, and save it to storage cache,
    /// then return the page result.
    pub fn fetch_or_update(&mut self, url: &Url) -> Result<String> {
        

        let path = match self.cache.get(url) {
            Some(path) => path.to_owned(),
            None => {
                let file_name = Self::generate_file_name( url ); //.to_file_path().map_err(|_| Error::new(ErrorKind::InvalidFilename, "Must have valid file name in url path!"))?;
                // let file_name = file_name.file_name().ok_or_else( || std::io::Error::new(std::io::ErrorKind::InvalidFilename, "Must have valid file name in url path!"))?;                
                let destination_path = self.config.cache_dir.join(file_name);
                
                let mut response = ureq::get(url.as_ref()).call().map_err(Error::other)?;
                let mut body = Vec::new();
                if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                    eprintln!("Fail to read data for cache: {e:?}");
                }
                
                // write the content to the file
                fs::write(&destination_path, body)?;
                destination_path    
            },
        };

        /* 
        // TODO can we avoid using to_owned()?
        let path = &self.cache.entry(url.to_owned()).or_insert({
            // code smells
            let mut tmp = &Self::get_dir()?;
            tmp.push(self.generate_file_name(url));
            
            // fetch the content from the url
            // expensive implict type cast?
            let mut response = ureq::get(url.as_ref()).call().map_err(Error::other)?;
            let mut body = Vec::new();
            if let Err(e) = response.body_mut().as_reader().read_to_end(&mut body) {
                eprintln!("Fail to read data for cache: {e:?}");
            }
            
            // write the content to the file
            fs::write(&tmp, body)?;
            tmp.to_path_buf()
        });
        */

        // let path = match self.cache.contains_key(url) {
        //     true => self.cache.get(url).unwrap(),
        //     false => {
        //         let path = self.save_content_to_cache(url)?.to_owned();
        //         self.cache.insert(url.to_owned(), path.clone());
        //         &path.clone()
        //     }
        // };

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
        let cache = PageCache::get_dir();
        assert!(cache.is_ok());
    }
}
