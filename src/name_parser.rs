use std::path::Path;

use log::{debug, warn};
use regex::Regex;

use crate::{media::{MediaData, MediaFile}, path_utils::{get_extension, get_filestem}, Config};

pub fn parse_filepath(path: &Path, config: &Config) -> Option<MediaFile> {
    let mut stem = get_filestem(path)?;
    for replacement in &config.replacements {
        debug!("Applying replacement {} -> {}", &replacement.0, &replacement.1);
        stem = stem.replace(&replacement.0, &replacement.1);
    }
    debug!("Applying regex to stem: {}", &stem);

    let (name, media_data) = parse_stem(&stem, &config)?;

    Some(MediaFile::new(name, media_data, get_extension(path)?))
}

fn parse_stem(stem: &str, config: &Config) -> Option<(String, MediaData)> {
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

        return Some((name, MediaData::TvSeries { season, episode, }));
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

        return Some((name, MediaData::Movie { year }));
    }

    None
}