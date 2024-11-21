use super::category::BlenderCategory;
use crate::page_cache::PageCache;
use regex::Regex;
use std::io::Result;
use url::Url;

#[derive(Debug)]
pub struct BlenderHome {
    pub list: Vec<BlenderCategory>,
    // I'd like to reuse this component throughout blender program. If I need to access a web page, this should be used.
    cache: PageCache,
}

impl BlenderHome {
    fn get_content(cache: &mut PageCache) -> Result<Vec<BlenderCategory>> {
        let parent = Url::parse("https://download.blender.org/release/").unwrap();
        let content = cache.fetch(&parent)?;

        // Omit any blender version 2.8 and below
        let pattern = r#"<a href=\"(?<url>.*)\">(?<name>Blender(?<major>[3-9]|\d{2,}).(?<minor>\d*).*)\/<\/a>"#;
        let regex = Regex::new(pattern).unwrap();

        let list = regex
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

        Ok(list)
    }

    // I need to hvae this reference regardless. Offline or online mode.
    pub fn new() -> Result<Self> {
        //  TODO: Verify this-: In original source code - there's a comment implying we should use cache as much as possible to avoid possible IP lacklisted.
        let mut cache = PageCache::load()?;
        let list = match Self::get_content(&mut cache) {
            Ok(col) => col,
            // maybe the user is offline, we don't know! This shouldn't stop the program from running
            Err(_) => Vec::new(),
        };
        Ok(Self { list, cache })
    }

    pub fn refresh(&mut self) -> Result<()> {
        let content = Self::get_content(&mut self.cache)?;
        self.list = content;
        Ok(())
    }
}
