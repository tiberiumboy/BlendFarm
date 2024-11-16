use super::category::BlenderCategory;
use crate::page_cache::PageCache;
use regex::Regex;
use std::io::Error;
use url::Url;

#[derive(Debug)]
pub struct BlenderHome {
    pub list: Vec<BlenderCategory>,
}

impl BlenderHome {
    // this might be a bit dangerous? we'll see?
    pub fn new() -> Result<Self, Error> {
        let parent = Url::parse("https://download.blender.org/release/").unwrap();

        //  TODO: Verify this-: In original source code - there's a comment implying we should use cache as much as possible to avoid possible IP lacklisted.
        let mut cache = PageCache::load()?;

        // fetch the content of the subtree information
        let content = cache.fetch(&parent)?;

        // Omit any blender version 2.8 and below
        let pattern = r#"<a href=\"(?<url>.*)\">(?<name>Blender(?<major>[3-9]|\d{2,}).(?<minor>\d*).*)\/<\/a>"#;
        let regex = Regex::new(pattern).unwrap();
        let collection = regex
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

        Ok(Self { list: collection })
    }
}
