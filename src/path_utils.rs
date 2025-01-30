use std::path::Path;

pub fn get_filestem(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_str()?.to_string())
}

pub fn get_extension(path: &Path) -> Option<String> {
    Some(path.extension()?.to_str()?.to_string())
}

pub fn get_filename(path: &Path) -> Option<String> {
    Some(path.file_name()?.to_str()?.to_string())
}