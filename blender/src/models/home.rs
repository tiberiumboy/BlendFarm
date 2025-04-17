use super::category::{BlenderCategory, BlenderCategoryError};
use crate::page_cache::PageCache;
use regex::Regex;
use std::io::{Error, ErrorKind};
use url::Url;

#[derive(Debug)]
pub struct BlenderHome {
    // might use this as a ref?
    list: Vec<BlenderCategory>,
    // I'd like to reuse this component throughout blender program. If I need to access a web page, this should be used.
    cache: PageCache,
}

impl BlenderHome {
    fn get_content(cache: &mut PageCache) -> Result<Vec<BlenderCategory>, Error> {
        let parent = Url::parse("https://download.blender.org/release/").unwrap();
        let content = cache.fetch(&parent)?;

        // Omit any blender version 2.8 and below
        let pattern = r#"<a href=\"(?<url>.*)\">(?<name>Blender(?<major>[3-9]|\d{2,}).(?<minor>\d*).*)\/<\/a>"#;
        let regex = Regex::new(pattern).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Unable to create new Regex pattern! {e:?}"),
            )
        })?;

        let mut list: Vec<BlenderCategory> = regex
            .captures_iter(&content)
            .map(|c| {
                let (_, [url, name, major, minor]) = c.extract();
                let url = parent.join(url).ok()?;
                let major = major.parse().ok()?;
                let minor = minor.parse().ok()?;
                Some(BlenderCategory::new(name.to_owned(), url, major, minor))
            })
            .flatten()
            .collect();

        list.sort_by(|a, b| b.cmp(a));
        Ok(list)
    }

    // I need to have this reference regardless. Offline or online mode.
    pub fn new() -> Result<Self, Error> {
        //  TODO: Verify this-: In original source code - there's a comment implying we should use cache as much as possible to avoid possible IP Blacklisted.
        let mut cache = PageCache::load()?;
        let list = Self::get_content(&mut cache).unwrap_or_else(|_| Vec::new());
        Ok(Self { list, cache })
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        let content = Self::get_content(&mut self.cache)?;
        self.list = content;
        Ok(())
    }

    pub fn get_latest(&self) -> Result<&BlenderCategory, BlenderCategoryError> {
        self.list.first().ok_or_else( || { BlenderCategoryError::NotFound })
    }

    // I may want to change this to see if I'm picking the one from locally installed or from remote
    pub fn get_version(&self, major: u64, minor: u64) -> Option<&BlenderCategory> {
        // Get the latest patch from blender home
        self.list
            .iter()
            .find(|v| v.major.eq(&major) && v.minor.eq(&minor))
    }
}

impl AsRef<Vec<BlenderCategory>> for BlenderHome {
    fn as_ref(&self) -> &Vec<BlenderCategory> {
        &self.list
    }
}
