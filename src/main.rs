use std::{
    env,
    fs::{self, DirEntry, OpenOptions},
    io, os,
    path::{Path, PathBuf},
    vec,
};

use clap::{builder::PossibleValue, Parser, ValueEnum};
use dir_walker::DirWalker;
use log::{debug, error, info, warn};
use name_parser::parse_filepath;
use path_utils::get_extension;
use serde::{Deserialize, Serialize};
use tvdb::TvdbClient;

mod dir_walker;
mod media;
mod name_parser;
mod path_utils;
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

    /// The output directory for the files
    #[arg(short, long)]
    output: String,

    /// The path of the configuration file
    #[arg(long)]
    config: Option<String>,

    /// Should print verbose output (useful for debugging config for example)
    #[arg(long, default_value_t = false)]
    verbose: bool,
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

    /// Directories with these names are ignored
    ignored_dirs: Vec<String>,
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
            replacements: vec![(".".to_string(), " ".to_string())],
            ignored_dirs: vec![
                "Sample".to_string(),
                "sample".to_string(),
                "Samples".to_string(),
                "samples".to_string(),
            ],
        }
    }
}

fn get_conf_dir() -> Option<PathBuf> {
    let Some(mut home_dir) = env::home_dir() else {
        error!("Home dir not found for config, consider specifying the config file path using --config");
        return None;
    };

    home_dir.push(".media-renamer");
    Some(home_dir)
}

fn get_filepath_in_conf_dir(filename: &str) -> Option<PathBuf> {
    let mut path = get_conf_dir()?;
    path.push(filename);
    Some(path)
}

fn extension_matches(path: &Path, extensions: &[String]) -> bool {
    let Some(ext) = get_extension(path) else { return false; };
    extensions.contains(&ext)
}

fn symlink(original: &Path, link: &Path) -> Result<(), io::Error> {
    let original_absolute = original.canonicalize()?;
    #[cfg(target_os = "windows")]
    {
        os::windows::fs::symlink_file(original_absolute, link)?;
    }
    #[cfg(target_os = "linux")]
    {
        os::unix::fs::symlink(original_absolute, link)?;
    }
    Ok(())
}

fn ensure_conf_dir_exists() {
    let conf_dir = get_conf_dir().expect("Could not get home directory");
    if !conf_dir.exists() {
        match fs::create_dir_all(&conf_dir) {
            Ok(()) => {}
            Err(error) => {
                println!(
                    "Could not create conf dir {}: {}",
                    conf_dir.display(),
                    error
                );
                return;
            }
        }
    }
}

fn init_logger(args: &Args) -> bool {
    let Some(log_filepath) = get_filepath_in_conf_dir("log.txt") else {
        return false;
    };

    let file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_filepath)
    {
        Ok(file) => file,
        Err(error) => {
            println!(
                "Could not open log file {}: {}",
                log_filepath.display(),
                error
            );
            return false;
        }
    };

    let level = if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    if let Err(error) = simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            level,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        simplelog::WriteLogger::new(level, simplelog::Config::default(), file),
    ]) {
        println!("Could not initialize logger: {}", error);
        return false;
    }

    true
}

fn read_config(args: &Args) -> Option<Config> {
    let config_path = match &args.config {
        Some(path) => Some(PathBuf::from(path)),
        None => get_filepath_in_conf_dir("config.toml"),
    }?;

    if !config_path.exists() {
        if let Some(parent) = config_path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                error!(
                    "Could not create directories to {}: {}",
                    parent.display(),
                    error
                );
            }
        }
        let default_config = Config::default();

        if let Err(error) = fs::write(
            &config_path,
            toml::to_string(&default_config).expect("Could not serialize the default config"),
        ) {
            error!(
                "Could not write default configuration to {}: {}",
                config_path.display(),
                error
            );
            warn!("Continuing with defaults");
        }
    }

    info!("Reading configuration from {}", config_path.display());
    let config = match fs::read_to_string(&config_path) {
        Ok(config_string) => match toml::from_str(&config_string) {
            Ok(conf) => conf,
            Err(error) => {
                error!(
                    "Could not parse config {}: {}",
                    config_path.display(),
                    error
                );
                warn!("Continuing with defaults");
                Config::default()
            }
        },
        Err(error) => {
            error!("Could not read config {}: {}", config_path.display(), error);
            warn!("Continuing with defaults");
            Config::default()
        }
    };

    Some(config)
}

fn process_file(path: &Path, args: &Args, config: &Config, tvdb: &TvdbClient) {
    info!("Processing file {}", path.display());

    let Some(mut media_file) = parse_filepath(path, &config) else {
        warn!("Could not parse filename {}", path.display());
        return;
    };

    match media_file.request_name(&tvdb) {
        Ok(true) => {}
        Ok(false) => {
            warn!("Could not find {} on TVDB. Ignoring", media_file.name());
            return;
        }
        Err(error) => {
            error!(
                "TVDB error while searching for {}: {}",
                media_file.name(),
                error
            );
        }
    }

    debug!("{:#?}", media_file);

    let mut final_path = PathBuf::from(&args.output);
    final_path.push(media_file.get_path());

    info!("Final path: {}", final_path.display());

    if final_path.exists() {
        warn!("File {} already exists: ignoring", final_path.display());
        return;
    }

    match args.action {
        Action::Test => {}
        _ => match final_path.parent() {
            Some(parent_final_path) => {
                if let Err(error) = fs::create_dir_all(parent_final_path) {
                    error!(
                        "Could not create directory {}: {}",
                        parent_final_path.display(),
                        error
                    );
                    return;
                }
            }
            None => {}
        },
    }

    match args.action {
        Action::Test => {
            info!(
                "TEST: would move from {} to {}",
                path.display(),
                final_path.display()
            );
        }
        Action::Move => {
            if let Err(error) = fs::rename(path, &final_path) {
                error!(
                    "Could not move {} to {}: {}",
                    path.display(),
                    final_path.display(),
                    error
                );
            }
        }
        Action::Copy => {
            if let Err(error) = fs::copy(path, &final_path) {
                error!(
                    "Could not copy {} to {}: {}",
                    path.display(),
                    final_path.display(),
                    error
                );
            }
        }
        Action::Symlink => {
            if let Err(error) = symlink(path, &final_path) {
                error!(
                    "Could not copy {} to {}: {}",
                    path.display(),
                    final_path.display(),
                    error
                );
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    ensure_conf_dir_exists();

    if !init_logger(&args) {
        return;
    }

    debug!("{:#?}", args);

    let Some(config) = read_config(&args) else {
        return;
    };

    debug!("{:#?}", config);

    info!("Connecting TVDB client");
    let mut tvdb = TvdbClient::new(&config.tvdb_api_key);
    if let Err(error) = tvdb.login() {
        error!("Error in logging in to API: ({})", error);
        return;
    }
    info!("Client connected");

    let input_path = PathBuf::from(&args.input);

    if input_path.is_file() {
        if extension_matches(&input_path, &config.extensions) {
            process_file(&input_path, &args, &config, &tvdb);
        } else {
            warn!("Input filename extension is not filtered in config, ignoring");
        }
    } else {
        for entry in DirWalker::new(&input_path, args.max_depth, config.ignored_dirs.clone())
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| extension_matches(&e.path(), &config.extensions))
        {
            process_file(&entry.path(), &args, &config, &tvdb);
        }
    }

}
