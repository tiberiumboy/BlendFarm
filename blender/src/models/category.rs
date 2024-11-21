use super::download_link::DownloadLink;
use crate::page_cache::PageCache;
use regex::Regex;
use semver::Version;
use std::env::consts;
use thiserror::Error;
use url::Url;

// I'd like to relocate this to a different file. Possibly home?
#[derive(Debug)]
pub struct BlenderCategory {
    pub name: String,
    pub url: Url,
    pub major: u64,
    pub minor: u64,
}

#[derive(Debug, Error)]
pub enum BlenderCategoryError {
    #[error("Architecture type \"{0}\" is not supported!")]
    InvalidArch(String),
    #[error("Unsupported operating system: {0}")]
    UnsupportedOS(String),
    #[error("Not found")]
    NotFound,
    #[error("Io Error")]
    Io(#[from] std::io::Error),
}

impl BlenderCategory {
    /// fetch current architecture (Currently support x86_64 or aarch64 (apple silicon))
    fn get_valid_arch() -> Result<String, BlenderCategoryError> {
        match consts::ARCH {
            "x86_64" => Ok("x64".to_owned()),
            "aarch64" => Ok("arm64".to_owned()),
            arch => Err(BlenderCategoryError::InvalidArch(arch.to_string())),
        }
    }

    /// Return extension matching to the current operating system (Only display Windows(.zip), Linux(.tar.xz), or macos(.dmg)).
    pub fn get_extension() -> Result<String, String> {
        match consts::OS {
            "windows" => Ok(".zip".to_owned()),
            "macos" => Ok(".dmg".to_owned()),
            "linux" => Ok(".tar.xz".to_owned()),
            os => Err(os.to_string()),
        }
    }

    pub fn new(name: String, url: Url, major: u64, minor: u64) -> Self {
        Self {
            name,
            url,
            major,
            minor,
        }
    }

    // TODO - implement thiserror?
    // for some reason I was fetching this multiple of times already. This seems expensive to call for some reason?
    // also, strange enough, the pattern didn't pick up anything?
    pub fn fetch(&self) -> Result<Vec<DownloadLink>, BlenderCategoryError> {
        // TODO: Find a way to recycle PageCache from BlenderHome
        let mut cache = PageCache::load()?; // I really hate the fact that I have to create a new instance for this.
        let content = cache.fetch(&self.url).map_err(BlenderCategoryError::Io)?;
        let arch = Self::get_valid_arch()?;
        let ext = Self::get_extension().map_err(BlenderCategoryError::UnsupportedOS)?;

        // Regex rules - Find the url that matches version, computer os and arch, and the extension.
        // - There should only be one entry matching for this. Otherwise return error stating unable to find download path
        let pattern = format!(
            r#"<a href=\"(?<url>.*)\">(?<name>.*-{}\.{}\.(?<patch>\d*.)-{}.*{}*.{})<\/a>"#,
            self.major,
            self.minor,
            consts::OS,
            arch,
            ext,
        );

        let regex = Regex::new(&pattern).unwrap();
        // for (_, [url, name, patch]) in
        let vec = regex
            .captures_iter(&content)
            .filter_map(|c| {
                let (_, [url, name, patch]) = c.extract();
                let url = self.url.join(url).ok()?;
                let patch = patch.parse().ok()?;
                let version = Version::new(self.major, self.minor, patch);
                Some(DownloadLink::new(name.to_owned(), url, version))
            })
            .collect();

        Ok(vec)
    }

    pub fn fetch_latest(&self) -> Result<DownloadLink, BlenderCategoryError> {
        let mut list = self.fetch()?;
        list.sort_by(|a, b| b.cmp(a));
        let entry = list.first().ok_or(BlenderCategoryError::NotFound)?;
        Ok(entry.clone())
    }

    pub fn retrieve(&self, version: &Version) -> Result<DownloadLink, BlenderCategoryError> {
        let list = self.fetch()?;
        let entry = list
            .iter()
            .find(|dl| dl.as_ref().eq(version))
            .ok_or(BlenderCategoryError::NotFound)?;
        Ok(entry.to_owned())
    }
}

impl PartialEq for BlenderCategory {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.major.eq(&other.major) && self.minor.eq(&other.minor)
    }
}

impl Eq for BlenderCategory {}

impl PartialOrd for BlenderCategory {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => return self.minor.partial_cmp(&other.minor),
            ord => return ord,
        }
    }
}

impl Ord for BlenderCategory {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => self.minor.cmp(&other.minor),
            all => return all,
        }
    }
}
