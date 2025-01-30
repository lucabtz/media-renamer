use std::{
    env, fs::{self, DirEntry}, io, os, path::{Path, PathBuf}
};

use clap::{builder::PossibleValue, Parser, ValueEnum};
use dir_walker::DirWalker;
use log::{debug, error, info, warn, };
use media::Media;
use name_parser::parse_filepath;
use serde::{Deserialize, Serialize};
use tvdb::{MediaType, TvdbClient};

mod dir_walker;
mod name_parser;
mod media;
mod tvdb;

#[derive(Debug, Clone, Copy)]
enum Action {
    Test,
    Move,
    Copy,
    Symlink,
}

impl ValueEnum for Action {
    fn value_variants<'a>() -> &'a [Self] {
        &[Action::Test, Action::Move, Action::Copy, Action::Symlink]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(Into::<&str>::into(*self)))
    }
}

impl From<Action> for &str {
    fn from(value: Action) -> Self {
        match value {
            Action::Test => "test",
            Action::Move => "move",
            Action::Copy => "copy",
            Action::Symlink => "symlink",
        }
    }
}

impl ToString for Action {
    fn to_string(&self) -> String {
        Into::<&str>::into(*self).into()
    }
}

#[derive(Parser, Debug)]
#[command(version, about = "Rename downloaded media and create the Plex directory structure", long_about = None)]
struct Args {
    /// The input file or folder
    #[arg(short, long)]
    input: String,

    /// The max depth to traverse directories, if none recurse indefinitely
    #[arg(short, long)]
    max_depth: Option<usize>,

    /// What action should be done on the files
    #[arg(short, long, default_value_t = Action::Test)]
    action: Action,

    // The output directory for the files
    #[arg(short, long)]
    output: String,

    // The path of the configuration file
    #[arg(long)]
    config: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    /// The API key for TVDB
    tvdb_api_key: String,

    /// The extensions of the files that should be processed
    extensions: Vec<String>,

    /// The regular expressions to parse tv series filenames
    tv_regex: Vec<String>,

    /// The regular expressions to parse movie filenames
    movie_regex: Vec<String>,

    /// Replacements that will be applied before matching with regex
    replacements: Vec<(String, String)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tvdb_api_key: "<ENTER HERE THE TVDB API KEY>".to_string(),
            extensions: vec!["mkv".to_string(), "srr".to_string()],
            tv_regex: vec![
                "(?<name>.*) [Ss](?<season>[0-9]+)[Ee](?<episode>[0-9]+)".to_string(), // Series Name S01E01
            ],
            movie_regex: vec![
                "(?<name>.*) (?<year>[0-9]+) ".to_string(), // Movie Name 2025
            ],
            replacements: vec![
                (".".to_string(), " ".to_string()),
            ],
        }
    }
}

fn extension_matches(entry: &DirEntry, extensions: &[String]) -> bool {
    let path = entry.path();
    let Some(extension_os) = path.extension() else {
        return false;
    };
    let Some(extension_str) = extension_os.to_str() else {
        return false;
    };
    let ext = extension_str.to_string();

    extensions.contains(&ext)
}

fn symlink(original: &Path, link: &Path) -> Result<(), io::Error> {
    #[cfg(target_os="windows")]
    {
        os::windows::fs::symlink_file(original, link)?;
    }
    #[cfg(target_os="linux")]
    {
        os::unix::fs::symlink(original, link);
    }
    Ok(())
}

fn main() {
    simplelog::TermLogger::init(
        log::LevelFilter::Debug,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .expect("Could not initialize logger");

    let args = Args::parse();

    debug!("{:#?}", args);

    let Some(config_path) = args.config.map_or_else(|| {
        let Some(mut home_dir) = env::home_dir() else { 
            error!("Home dir not found for config, consider specifying the config file path using --config");
            return None;
        };
            
        home_dir.push(".media-renamer");
        home_dir.push("config.toml");
        Some(home_dir)
    }, |c| Some(PathBuf::from(c))) else { return; };

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                error!("Could not create directories to {}: {}", parent.display(), error);
            }
        }
        let default_config = Config::default();

        if let Err(error) = fs::write(
            &config_path,
            toml::to_string(&default_config).expect("Could not serialize the default config"),
        ) {
            error!("Could not write default configuration to {}: {}", config_path.display(), error);
            warn!("Continuing with defaults");
        }
    }

    info!("Reading configuration from {}", config_path.display());
    let config = match fs::read_to_string(&config_path) {
        Ok(config_string) => {
            match toml::from_str(&config_string) {
                Ok(conf) => conf,
                Err(error) => {
                    error!("Could not parse config {}: {}", config_path.display(), error);
                    warn!("Continuing with defaults");
                    Config::default()
                },
            }
        },
        Err(error) => {
            error!("Could not read config {}: {}", config_path.display(), error);
            warn!("Continuing with defaults");
            Config::default()
        },
    };

    debug!("{:#?}", config);

    info!("Connecting TVDB client");
    let mut tvdb = TvdbClient::new(&config.tvdb_api_key);
    if let Err(error) = tvdb.login() {
        error!("Error in logging in to API: ({})", error);
        return;
    }
    info!("Client connected");    

    for entry in DirWalker::new(&args.input, args.max_depth)
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| extension_matches(e, &config.extensions))
    {
        info!("Processing file {}", entry.path().display());

        let Some(parsed_file) = parse_filepath(&entry.path(), &config) else {
            warn!("Could not parse filename {}", entry.path().display());
            continue;
        };

        let media_type = match parsed_file.media() {
            Media::TvSeries { .. } => MediaType::Series,
            Media::Movie { .. } => MediaType::Movie,
        };
        
        let media = match parsed_file.media() {
            Media::TvSeries { name, .. } | Media::Movie { name, ..} => {
                match tvdb.search(&name, media_type) {
                    Ok(results) => {
                        if let Some(result) = results.first() {
                            parsed_file.media().change_name(result.name.clone())    
                        } else {
                            warn!("No result for {} on TVDB: ignoring", name);
                            continue;
                        }
                    },
                    Err(error) => {
                        error!("{}", error);
                        continue;
                    },
                }
            },
        };

        debug!("{:#?}", media);

        let mut final_path = PathBuf::from(&args.output);
        final_path.push(media.get_path(parsed_file.extension()));

        info!("Final path: {}", final_path.display());

        match args.action {
            Action::Test => {},
            _ => match final_path.parent() {
                Some(parent_final_path) => {
                    if let Err(error) = fs::create_dir_all(parent_final_path) {
                        error!("Could not create directory {}: {}", parent_final_path.display(), error);
                        continue;
                    }
                },
                None => {},
            }
            
        }

        match args.action {
            Action::Test => {
                info!("TEST: would move from {} to {}", entry.path().display(), final_path.display());
            },
            Action::Move => {
                if let Err(error) = fs::rename(entry.path(), &final_path) {
                    error!("Could not move {} to {}: {}", entry.path().display(), final_path.display(), error);
                }
            },
            Action::Copy => {
                if let Err(error) = fs::copy(entry.path(), &final_path) {
                    error!("Could not copy {} to {}: {}", entry.path().display(), final_path.display(), error);
                }
            },
            Action::Symlink => {
                if let Err(error) = symlink(&entry.path(), &final_path) {
                    error!("Could not copy {} to {}: {}", entry.path().display(), final_path.display(), error);
                }
            },
        }
    }
}
