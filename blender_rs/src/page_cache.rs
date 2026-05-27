use crate::constant::MAX_VALID_DAYS;
use lazy_regex::regex_replace_all;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, ErrorKind, Error, Read, Result};
use std::path::Path;
use std::time::Duration;
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

impl ExpirationUnits {

    const DAYS_TO_WEEK: u64 = 7;
    const DAYS_TO_MONTH: u64 = 30;
    const DAY_INTO_HOURS: u64 = 24;
    const WEEK_INTO_HOURS: u64 = Self::DAY_INTO_HOURS * Self::DAYS_TO_WEEK;
    const MONTH_INTO_HOURS: u64 = Self::DAY_INTO_HOURS * Self::DAYS_TO_MONTH;

    fn cast_to_duration(&self) -> Option<Duration> {
        match self {
            ExpirationUnits::Day(d) => {
                Some(Duration::from_hours((*d as u64) * Self::DAY_INTO_HOURS))
            },
            ExpirationUnits::Week(w) => {
                Some(Duration::from_hours((*w as u64) * Self::WEEK_INTO_HOURS))
            }
            ExpirationUnits::Month(m) => {
                Some(Duration::from_hours((*m as u64) * Self::MONTH_INTO_HOURS))
            },
            ExpirationUnits::Disable => None
        }
    } 

    // None is return when ExpirationUnits is disabled
    pub fn get_expiration_date(&self) -> Option<SystemTime> {
        let current_date = SystemTime::now();
        let duration = self.cast_to_duration()?;
        current_date.checked_sub(duration)
    }
}

impl Default for ExpirationUnits {
    fn default() -> Self {
        ExpirationUnits::Month(6)
    }
}
// Hide this for now,
#[doc(hidden)]
// rely the cache creation date on file metadata.
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PageCache {
    #[serde(skip)]
    inner: PathBuf,
    cache: HashMap<Url, PathBuf>,
    expiration_duration: ExpirationUnits,
    cache_dir: PathBuf,
}

// the whole idea behind this was to store information from blender with minimal connectivity
// interface as possible. Rely on cache if we need to lookup again. This separate us from ChatGPT and other LLM agents.
impl PageCache {
    const CACHE_DIR: &str = "cache";
    const CONFIG_NAME: &str = "cache.json";
    const SECONDS_TO_HOUR: u64 = 3600;
    const HOURS_TO_DAY: u64 = 24;

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
    pub(crate) fn save(&mut self) -> Result<()> {
        let data = serde_json::to_string(&self)?;
        fs::write(&self.inner, data)
    }

    // TODO: See where and how we can utilize this validation process?
    #[allow(dead_code)]
    fn validate_cache(&mut self) {
        // Here we run a check of all of the cache we have stored, and then check the last modified date. If it exceed page cache's
        // TODO: Present a "Delete cache after X Y" Where X is a number and Y is enum such as Day, Weeks, or Month 
        // - We should be realistic, protective, and caution about security and delete cache older than 6 months as default value, 
        //      unless someone objects this idea and creates a PR request removing this comment and prove me wrong why we should store cache older than a year? 
        //      At this point, you might as well just turn off this feature?
        
        // gather a list of files currently in the cache directory (excluding cache.json)
        // this will help us clean the cache folder of files ready to be deleted from the system.
        // let files_found = fs::read_dir(&self.cache_dir).map_or(Vec::new(), f);
        
        self.cache.retain(|_, v| {
            if !&v.exists() {
                return false;
            }

            if let Some(expiration_date) = self.expiration_duration.get_expiration_date() {
                match fs::metadata(&v) {
                    Ok(m) => {
                        // the error would raise if field doesn't exist on specific platform. I believe we're safeguarded to use latest major OS platform (Linux/Mac/Win)
                        return match m.created() {
                            Ok(date) => expiration_date.ge(&date),
                            Err(e) => {
                                eprintln!("Shouldn't be possible to error unless the feature doesn't exist on target platform: {e:?}");
                                return false;
                            }
                        }
                    }, 
                    Err(e) => {
                        eprintln!("[PageCache] Unable to read metadata!{e:?}");
                        return false;
                    }
                }
            }

            // how do we handle with files existing in the cache?

            // because of "disable" enum, disable retains all records.
            true
        });
    }

    // suppressing this for now, I'm testing the program out without having to worry about invalidating cache files for now.
    // Currently used in commented code in PageCache::load() implementation.
    #[allow(dead_code)]
    fn check_expiration(cache_path: impl AsRef<Path>) -> bool {
        let current = SystemTime::now();
        let fallback = current.clone();
        // read the metadata of the cache.json file.
        // if the creation date is beyond the configuration expiration rule, we should delete the file and refresh from the source of truth.
        let created_date = match fs::metadata(&cache_path) {
            Ok(m) => m
                .is_file()
                .then(|| m.created().unwrap_or(fallback))
                .unwrap_or_else(|| fallback),
            _ => fallback,
        };

        // TODO: For now I'm trying to test this out without having to redownload everything again from the internet source.
        // if file exist and provides duration date.
        if let Ok(duration) = current.duration_since(created_date) {
            // must be within valid window timeframe.
            if duration.as_secs() < MAX_VALID_DAYS * Self::SECONDS_TO_HOUR * Self::HOURS_TO_DAY { 
                // TODO: Enable via verbosity option     
                println!(
                    "Time still valid: Remaining {}hrs",
                    duration.as_secs() / Self::SECONDS_TO_HOUR - (MAX_VALID_DAYS * Self::HOURS_TO_DAY)   
                );
                return true;
            }
        }
        false
    }

    // TODO: name is too ambiguous. What is load? What are we loading? What does it do? Does it load the program? File? Something?
    pub fn load() -> Result<Self> {
        // use define path to cache file
        let path = Self::get_cache_path()?;
        
        // TODO: For now I'm trying to test this out without having to redownload everything again from the internet source.
        // use define path to cache file
        // if Self::check_expiration(&path) == false {
        //     return Ok(Self::default());
        // }

        let reader = BufReader::new(fs::File::open(&path)?);
        let mut data: PageCache = serde_json::from_reader(reader)?;
        data.inner = path;
        Ok(data)
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
        
        // TODO can we avoid using to_owned()/clone()?
        let path = self.cache.entry(url.clone()).or_insert( {
                let file_name = Self::generate_file_name( url );
                let destination_path = self.cache_dir.join(file_name);

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
