use std::path::PathBuf;

use crate::tvdb::{TvdbClient, TvdbError};

#[derive(Debug)]
pub struct MediaFile {
    name: String,
    extension: String,
    media_data: MediaData,
}

impl MediaFile {
    pub fn new(name: String, media_data: MediaData, extension: String) -> Self {
        Self {
            name,
            extension,
            media_data,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn media(&self) -> &MediaData {
        &self.media_data
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn media_type(&self) -> MediaType {
        match self.media_data {
            MediaData::TvSeries { .. } => MediaType::Series,
            MediaData::Movie { .. } => MediaType::Movie,
        }
    }

    pub fn request_name(&mut self, tvdb: &TvdbClient) -> Result<bool, TvdbError> {
        let media_type = match self.media_data {
            MediaData::TvSeries { .. } => MediaType::Series,
            MediaData::Movie { .. } => MediaType::Movie,
        };

        let results = tvdb.search(&self.name, media_type)?;

        if let Some(result) = results.first() {
            self.name = result.name.clone();
        } else {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn get_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        match &self.media_data {
            MediaData::TvSeries { season, episode } => {
                path.push("TV");
                path.push(&self.name);
                path.push(format!("Season {}", season));
                path.push(format!(
                    "{} - s{:0>2}e{:0>2}.{}",
                    &self.name, season, episode, &self.extension
                ));
            }
            MediaData::Movie { year } => {
                path.push("Movies");
                path.push(format!("{} ({})", &self.name, year));
                path.push(format!("{} ({}).{}", &self.name, year, &self.extension));
            }
        }

        path
    }
}

#[derive(Debug)]
pub enum MediaData {
    TvSeries { season: u32, episode: u32 },
    Movie { year: u32 },
}

#[derive(PartialEq, Eq, Debug)]
pub enum MediaType {
    Movie,
    Series,
}

impl From<MediaType> for &str {
    fn from(value: MediaType) -> Self {
        match value {
            MediaType::Movie => "movie",
            MediaType::Series => "series",
        }
    }
}
