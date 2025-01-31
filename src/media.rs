use std::path::PathBuf;

#[derive(Debug)]
pub enum Media {
    TvSeries {
        name: String,
        season: u32,
        episode: u32,
    },

    Movie {
        name: String,
        year: u32,
    },
}

impl Media {
    pub fn change_name(&self, new_name: String) -> Media {
        match *self {
            Media::TvSeries { season, episode, .. } => Media::TvSeries { name: new_name, season, episode, },
            Media::Movie { year, .. } => Media::Movie { name: new_name, year },
        }
    }

    pub fn get_path(&self, extension: &str) -> PathBuf {
        let mut path = PathBuf::new();

        match self {
            Media::TvSeries { name, season, episode } => {
                path.push("TV");
                path.push(name);
                path.push(format!("Season {}", season));
                path.push(format!("{} - s{:0>2}e{:0>2}.{}", name, season, episode, extension));
            },
            Media::Movie { name, year } => {
                path.push("Movies");
                path.push(format!("{} ({})", name, year));
                path.push(format!("{} ({}).{}", name, year, extension));
            },
        }

        path
    }
}