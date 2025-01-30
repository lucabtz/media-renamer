use std::path::Path;

use log::{debug, warn};
use regex::Regex;

use crate::{media::Media, Config};

pub fn parse_filepath(path: &Path, config: &Config) -> Option<ParsedFile> {
    let mut stem = get_filestem(path)?;
    for replacement in &config.replacements {
        debug!("Applying replacement {} -> {}", &replacement.0, &replacement.1);
        stem = stem.replace(&replacement.0, &replacement.1);
    }
    debug!("Applying regex to stem: {}", &stem);

    let media = parse_stem(&stem, &config)?;

    Some(ParsedFile { media, extension: get_extension(path)? })
}

pub struct ParsedFile {
    media: Media,
    extension: String,
}

impl ParsedFile {
    pub fn media(&self) -> &Media {
        &self.media
    }
    
    pub fn extension(&self) -> &str {
        &self.extension
    }
}

fn get_filestem(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_str()?.to_string())
}

fn get_extension(path: &Path) -> Option<String> {
    Some(path.extension()?.to_str()?.to_string())
}

fn parse_stem(stem: &str, config: &Config) -> Option<Media> {
    for re_string in &config.tv_regex {
        let Ok(re) = Regex::new(re_string) else {
            warn!(
                "Invalid regex {} consider fixing in the config file",
                re_string
            );
            continue;
        };

        debug!("Trying TV regex {}", re_string);

        let Some(captures) = re.captures(&stem) else {
            continue;
        };

        let Some(name) = captures.name("name").map(|n| n.as_str().to_string()) else {
            continue;
        };

        debug!("Found name: {}", name);

        let Some(season) = captures.name("season").map(|s_str| s_str.as_str()) else {
            continue;
        };
        let Ok(season) = season.parse::<u32>() else {
            continue;
        };

        debug!("Found season: {}", season);

        let Some(episode) = captures.name("episode").map(|s_str| s_str.as_str()) else {
            continue;
        };
        let Ok(episode) = episode.parse::<u32>() else {
            continue;
        };

        debug!("Found episode: {}", episode);

        return Some(Media::TvSeries { name, season, episode, });
    }

    for re_string in &config.movie_regex {
        let Ok(re) = Regex::new(re_string) else {
            warn!(
                "Invalid regex {} consider fixing in the config file",
                re_string
            );
            continue;
        };

        debug!("Trying movie regex {}", re_string);

        let Some(captures) = re.captures(&stem) else {
            continue;
        };

        let Some(name) = captures.name("name").map(|n| n.as_str().to_string()) else {
            continue;
        };

        debug!("Found name: {}", name);

        let Some(year) = captures.name("year").map(|s_str| s_str.as_str()) else {
            continue;
        };
        let Ok(year) = year.parse::<u32>() else {
            continue;
        };

        debug!("Found year: {}", year);

        return Some(Media::Movie { name, year, });
    }

    None
}