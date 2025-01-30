# media-renamer

> [!WARNING]  
> This program is still work-in-progress, some features may not work as intended. Contributions are welcome!

media-renamer is an utility program to rename media files into the format supported by the Plex Media Server.

It works by first walking through an input directory (recursively or not) and collecting the files with the recognized media extensions (configurable).
The filenames are then parsed first applying a set of string replacements (configurable) to say, to example, replace dots (.) with spaces. Then a configurable
set of regex is tried to extract the media name, episode and season.

Then, using the TVDB API the correct media name is searched and finally all is placed into the output directory.

# Parameters
Help text
```
Rename downloaded media and create the Plex directory structure

Usage: media-renamer.exe [OPTIONS] --input <INPUT> --output <OUTPUT>

Options:
  -i, --input <INPUT>          The input file or folder
  -m, --max-depth <MAX_DEPTH>  The max depth to traverse directories, if none recurse indefinitely
  -a, --action <ACTION>        What action should be done on the files [default: test] [possible values: test, move, copy, symlink]
  -o, --output <OUTPUT>        The output directory for the files
      --config <CONFIG>        The path of the configuration file
  -h, --help                   Print help
  -V, --version                Print version
```
Explanation:
- `--input`: the input directory
- `--max-depth`: the max depth to traverse the directory, or nothing to recurse indefinitely
- `--action`: the action to be done on the files:
  * `test`: just print what would happen
  * `move`: move the files to the new location
  * `copy`: copy the files to the new location (useful to keep seeding files when torrenting)
  * `symlink`: create a symlink to the original file in the new location (useful to keep seeding when disk space is an issue)
- `--output`: the output directory
- `--config`: the path to the configuration file, if not set it is at `~/.media-renamer/config.toml` and will be created after the first run
- `--help`: prints the help text
- `--version`: prints the program version

# Configuration
Default configuration
```toml
tvdb_api_key = "9dfa4bc9-a0ff-4d9a-a99b-41a36531350f"
extensions = ["mkv", "srr"]
tv_regex = ["(?<name>.*) [Ss](?<season>[0-9]+)[Ee](?<episode>[0-9]+)"]
movie_regex = ["(?<name>.*) (?<year>[0-9]+) "]
replacements = [[".", " "]]
ignored_dirs = ["Sample", "sample", "Samples", "samples"]
```
Explanation:
- `tvdb_api_key`: self-explanatory
- `extensions`: only the files with these extensions are processed
- `tv_regex`: if the filename matches any of these regexes, the file is considered a TV Show. The default regex matches `Show Name S01E01`
- `movie_regex`: if the filename matches any of these regexes and does not match any TV Show regex the file is considered a movie. The default regex matches `Move Name 2025`
- `replacements`: replacements to be applied before the regexes are matched. The default replacement allows matching  `Show.Name.S01E01` and  `Show Name S01E01` with the same regex.
- `ignored_dirs`: directories names that should be ignored while traversing the directory tree.

# Installation
You need to have `cargo` installed, then
```bash
git clone https://github.com/lucabtz/media-renamer
cd media-renamer
cargo install
```
