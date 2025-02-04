use std::path::Path;

use log::{debug, warn};
use regex::Regex;

use crate::{
    media::{MediaData, MediaFile},
    path_utils::{get_extension, get_filestem},
    Config,
};

pub fn parse_filepath(path: &Path, config: &Config) -> Option<MediaFile> {
    let mut stem = get_filestem(path)?;
    for replacement in &config.replacements {
        debug!(
            "Applying replacement {} -> {}",
            &replacement.0, &replacement.1
        );
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

        return Some((name, MediaData::TvSeries { season, episode }));
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

#[cfg(test)]
mod tests {
    use core::panic;
    use std::path::PathBuf;

    use crate::media::MediaData;

    use super::*;

    fn test_series(
        config: &Config,
        test_path: &str,
        test_name: &str,
        test_season: u32,
        test_episode: u32,
    ) {
        let path = PathBuf::from(test_path);
        let Some(media_file) = parse_filepath(&path, &config) else {
            panic!("parse_filepath failed for {}", test_path);
        };
        assert_eq!(media_file.name(), test_name);
        match media_file.media() {
            MediaData::TvSeries { season, episode } => {
                assert_eq!(*season, test_season);
                assert_eq!(*episode, test_episode);
            }
            _ => panic!("Should be a series"),
        }
    }

    fn test_movie(config: &Config, test_path: &str, test_name: &str, test_year: u32) {
        let path = PathBuf::from(test_path);
        let Some(media_file) = parse_filepath(&path, &config) else {
            panic!("parse_filepath failed for {}", test_path);
        };
        assert_eq!(media_file.name(), test_name);
        match media_file.media() {
            MediaData::Movie { year } => assert_eq!(*year, test_year),
            _ => panic!("Should be a movie"),
        }
    }

    #[test]
    fn default_config_matchers() {
        let config = Config::default();
        test_series(
            &config,
            "Paradise.2025.S01E04.480p.x264-RUBiK.mkv",
            "Paradise 2025",
            1,
            4,
        );
        test_series(
            &config,
            "Star.Wars.Skeleton.Crew.S01E08.480p.x264-RUBiK.mkv",
            "Star Wars Skeleton Crew",
            1,
            8,
        );
        test_movie(
            &config,
            "Smile 2 2024 BluRay 1080p AC-3 TrueHD7.1 Atmos _+ Multi H264-PiR8.mkv",
            "Smile 2",
            2024,
        );
        test_movie(
            &config,
            "Conclave.2024.2160p.UHD.BluRay.x265-SURCODE.mkv",
            "Conclave",
            2024,
        );
        test_movie(
            &config,
            "Anora.2024.2160p.iT.WEB-DL.DDP5.1.DV.HDR.H.265-DRX.mkv	",
            "Anora",
            2024,
        );
        test_movie(
            &config,
            "Pulse.2001.German.AUS.UHDBD.2160p.HDR10.HEVC.DTSHD.DL.Remux-pmHD.mkv	",
            "Pulse",
            2001,
        );
        test_movie(
            &config,
            "Blade.Runner.2049.2017.2160p.MA.WEB-DL.TrueHD.Atmos.7.1.DV.HDR.H.265-FLUX.mkv	",
            "Blade Runner 2049",
            2017,
        );
    }
}
