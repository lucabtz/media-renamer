use std::{
    collections::VecDeque,
    fs::{self, DirEntry, ReadDir},
    io::Error,
    path::PathBuf,
};

use log::debug;

use crate::path_utils::get_filename;

/// An iterator that iterates over a directory
pub struct DirWalker {
    iterator_queue: VecDeque<Result<ReadDir, Error>>,
    max_depth: Option<usize>,
    ignored_dirs: Vec<String>,
}

impl DirWalker {
    pub fn new(directory: &str, max_depth: Option<usize>, ignored_dirs: Vec<String>) -> Self {
        let path = PathBuf::from(directory);
        let mut iterator_queue = VecDeque::new();
        if !path.is_dir() {
            let error = Err(Error::new(
                std::io::ErrorKind::NotADirectory,
                format!("The file at {} is not a directory", directory),
            ));
            iterator_queue.push_back(error);
        } else {
            let iter = fs::read_dir(path);
            iterator_queue.push_back(iter);
        }

        Self {
            iterator_queue,
            max_depth,
            ignored_dirs,
        }
    }
}

impl Iterator for DirWalker {
    type Item = Result<DirEntry, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(iter_res) = self.iterator_queue.pop_front() {
            if self.max_depth.is_some_and(|val| val <= 0) {
                break
            }

            match iter_res {
                Ok(mut iter) => {
                    if let Some(item) = iter.next() {
                        if let Ok(entry) = &item {
                            if entry.path().is_dir() {
                                if get_filename(&entry.path()).is_some_and(|name| self.ignored_dirs.contains(&name)) {
                                    debug!("Ignoring directory {} because excluded", entry.path().display());
                                } else {
                                    debug!("Adding directory to iteration queue {}", entry.path().display());
                                    self.iterator_queue.push_back(fs::read_dir(entry.path()));
                                }
                            }
                        }
                        // put back the iterator in front of the queue, it may be not exhausted yet
                        self.iterator_queue.push_front(Ok(iter));
                        return Some(item);
                    } 
                    // else {
                    // the iterator is exausted, try with the next in the queue
                    self.max_depth = self.max_depth.map(|val| val - 1);
                    // continue;
                    // }
                }
                Err(e) => return Some(Err(e)),
            }
        }
        // all iterators are exhausted, done
        None
    }
}
