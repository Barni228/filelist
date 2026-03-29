use dashmap::DashMap;
use either::Either;
use getset::{CopyGetters, Getters, MutGetters, Setters, WithSetters};
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};
use walkdir::WalkDir;

use crate::helper::{IsHidden, replace_when};

// TODO: since I removed logic that doesn't hash output file, if user gives force flag then remove output file before doing any hashing
#[derive(Clone, Getters, Setters, WithSetters, MutGetters, CopyGetters)]
pub struct Hasher {
    /// If true, don't hash any files
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    no_hash: bool,
    /// If true, hash hidden files
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    all: bool,
    /// If true, hash directories
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    hash_directory: bool,
    /// If true, when passing a directory, this will return hash of directory and hash of everything inside
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    recursive: bool,
    /// If true, follow symlinks, if false, this will treat symlinks as files
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    follow_links: bool,
    /// If true, hash files in parallel
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    use_parallel: bool,
    /// The paths that will be hashed
    #[getset(get = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    paths: Vec<PathBuf>,
    /// An optional progress object (can be used to show progress bar)
    progress: Option<Arc<dyn HasherProgress>>,
    cache: Arc<DashMap<PathBuf, String>>,
}

impl Default for Hasher {
    fn default() -> Self {
        Self {
            no_hash: false,
            all: false,
            hash_directory: false,
            recursive: true,
            follow_links: false,
            use_parallel: true,
            paths: Vec::new(),
            progress: None,
            cache: Arc::new(DashMap::new()),
        }
    }
}

impl std::fmt::Debug for Hasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hasher")
            .field("no_hash", &self.no_hash)
            .field("all", &self.all)
            .field("hash_directory", &self.hash_directory)
            .field("recursive", &self.recursive)
            .field("follow_links", &self.follow_links)
            .field("use_parallel", &self.use_parallel)
            .field("paths", &self.paths)
            .field(
                "progress",
                &self
                    .progress
                    .as_ref()
                    .map(|_| "Some(<dyn HasherProgress>)")
                    .unwrap_or("None"),
            )
            .finish()
    }
}

impl Hasher {
    pub fn set_progress(&mut self, progress: Arc<dyn HasherProgress>) {
        self.progress = Some(progress);
    }
    pub fn clear_progress(&mut self) {
        self.progress = None;
    }
    pub fn with_progress(mut self, progress: Arc<dyn HasherProgress>) -> Self {
        self.set_progress(progress);
        self
    }
}

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start hashing, will call all [`HasherProgress`] methods
    /// returns a BTreeMap of paths to hashes
    pub fn start(&mut self) -> BTreeMap<PathBuf, String> {
        let output_paths = self.get_output_paths();
        let dependencies = self.get_hash_dependencies(&output_paths);

        if let Some(progress) = &self.progress {
            progress.init(dependencies.iter().flatten().map(|p| p.as_path()).collect());
        }

        // cache every single path, in such order that we never hash the same file twice
        for set in &dependencies {
            replace_when! {
                self.use_parallel,
                set.[par_iter | iter]().for_each(|p| {
                    self.hash_no_error(p);
                })
            };
        }

        if let Some(progress) = &self.progress {
            progress.finish();
        }

        // convert every path into a hash, and collect as a BTreeMap
        replace_when! {
            self.use_parallel,
            output_paths
                .[into_par_iter | into_iter]()
                // .map(|path| (path, self.hash_no_error(path)))
                // get everything from cache
                .map(|path| {
                    let hash = self.cache.get(&path).unwrap().value().clone();
                    (path, hash)
                })
                .collect()
        }
    }

    /// Hash a file or directory, and cache the result
    pub fn hash(&self, path: &Path) -> io::Result<String> {
        // if a VALID hash is in the cache, return it
        // if there is an Error, then try to hash it again to get the same error and return that
        // this is because I cannot cache io::Result (does not implement Clone)
        if let Some(hash) = self.cache.get(path)
            && !hash.starts_with("ERROR:")
        {
            return Ok(hash.clone());
        }

        // if we dont follow symlinks and the path is a symlink, hash the target path
        let hash_result = if self.no_hash {
            Ok(String::new())
        } else if path.is_symlink() && !self.follow_links {
            self.hash_link(path)
        } else if path.is_dir() {
            self.hash_dir(path)
        // if this is something else, like a file, /dev/fd/* or non existing path, treat it as file
        } else {
            self.hash_file(path)
        };

        // get the hash as a string, because i still need to cache and progress it
        let hash_str = result_to_hash(&hash_result);

        if let Some(progress) = &self.progress {
            progress.update_file(path, &hash_str);
        }
        // cache it even if its an error, so that hash_no_error can use it
        self.cache.insert(path.to_path_buf(), hash_str);

        hash_result
    }

    pub fn hash_no_error(&self, path: &Path) -> String {
        // if the path is in the cache, even if it is an error, return it
        if let Some(hash) = self.cache.get(path) {
            return hash.clone();
        }
        result_to_hash(&self.hash(path))
    }

    /// Hash something that implements Read
    ///
    /// Could be a file, stdin, or anything else
    pub fn hash_reader(&self, mut reader: impl io::Read) -> io::Result<String> {
        let mut hasher = Sha256::new();

        let mut buffer = [0u8; 8192];
        loop {
            let bytes = reader.read(&mut buffer)?;
            if bytes == 0 {
                break;
            }
            if let Some(progress) = &self.progress {
                progress.update_bytes(bytes);
            }
            hasher.update(&buffer[..bytes]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Return true if the path is a dir, or a followed symlink to a dir
    #[inline]
    pub fn is_dir_no_link(&self, path: &Path) -> bool {
        path.is_dir() && (self.follow_links || !path.is_symlink())
    }
}

impl Hasher {
    // TODO: make this HashSet<PathBuf>
    fn get_output_paths(&self) -> Vec<PathBuf> {
        let mut real_paths: Vec<PathBuf> = self
            .paths
            .iter()
            .flat_map(|p| {
                if self.recursive && p.is_dir() {
                    // either allows two iterators to be the same type
                    Either::Left(
                        WalkDir::new(p)
                            .follow_links(self.follow_links)
                            .follow_root_links(self.follow_links)
                            .into_iter()
                            // filter out hidden files if `all` is not set, and if they are not the root
                            // so if the user gives .dir, I will include it even without `all`
                            .filter_entry(|e| self.all || !e.is_hidden() || e.depth() == 0)
                            .filter_map(Result::ok)
                            // filter out directories if `directory` is false
                            .filter(|e| self.hash_directory || !e.file_type().is_dir())
                            .map(|e| e.into_path()), // convert to PathBuf
                    )
                } else {
                    Either::Right(std::iter::once(p.to_path_buf()))
                }
            })
            .collect();

        real_paths.sort_unstable();
        // remove same consecutive elements, since this is sorted it will remove all duplicates
        real_paths.dedup();

        real_paths
    }

    /// get a list which says in what order the paths should be hashed
    fn get_hash_dependencies(&self, paths: &[PathBuf]) -> Vec<HashSet<PathBuf>> {
        // BTreeMap is a sorted HashMap
        let mut dependencies: BTreeMap<usize, HashSet<PathBuf>> = BTreeMap::new();
        // only directories are in this HashMap, files are immediately added to dependencies
        let mut depths: HashMap<PathBuf, usize> = HashMap::new();

        for p in paths {
            if !self.is_dir_no_link(p) {
                dependencies.entry(0).or_default().insert(p.to_path_buf());
                continue;
            }
            // if this dir has already been added, then don't add it again
            if depths.contains_key(p) {
                continue;
            }
            // if you give a file to WalkDir, it will just return it
            // by default, WalkDir will return directory before its contents (can change with `contents_first`)
            WalkDir::new(p)
                .follow_links(self.follow_links)
                // this doesn't really matter, because I already make sure that `p`
                // is something that needs to be followed by using `is_dir_no_link`
                .follow_root_links(self.follow_links)
                .into_iter()
                .filter_entry(|e| self.all || !e.is_hidden() || e.depth() == 0)
                .filter_map(Result::ok)
                .for_each(|e| {
                    // WalkDir DirEntry will only return true if it is dir, or symlink AND `follow_links` is true
                    // more efficient than `self.is_dir_no_link(&path)` because WalkDir already knows this info without sys calls
                    if e.file_type().is_dir() {
                        depths.insert(e.into_path(), 0);
                    // if this is file or file symlink or unfollowed symlink
                    } else {
                        // entry 0 is for all files or empty directories (they dont have any dependencies)
                        // skip the file itself
                        for (i, parent) in e.path().ancestors().enumerate().skip(1) {
                            // if someone already gave a higher depth to my parent,
                            // then that someone also gave a better depth to all of my parents, so break
                            if let Some(depth) = depths.get_mut(parent)
                                && *depth < i
                            {
                                *depth = i;
                            } else {
                                break;
                            }
                        }
                        dependencies.entry(0).or_default().insert(e.into_path());
                    }
                });
        }

        // convert depths HashMap to dependencies BTreeMap
        for (p, depth) in depths {
            dependencies.entry(depth).or_default().insert(p);
        }

        // get all of the values (which are sorted by depth) and collect them to Vec
        dependencies.into_values().collect()
    }

    /// Hash a directory <br>
    /// The way this works is: <br>
    /// it will hash everything inside of the directory,
    /// sort all of those hashes, and then hash them all together
    fn hash_dir(&self, path: &Path) -> io::Result<String> {
        let mut hashes: Vec<String> = replace_when! {
            self.use_parallel,
            fs::read_dir(path)?
                .filter_map(Result::ok)
                // HACK: if use_parallel is false, I call by_ref in here, to effectively do nothing,
                // because i need to call par_bridge if use_parallel is true, but not call it when use_parallel is false
                .[par_bridge | by_ref]()
                .filter_map(|entry| {
                    if !self.all && entry.is_hidden() {
                        return None;
                    }

                    // ignore all the errors
                    // self.hash(&entry.path()).ok()
                    let hash = self.hash_no_error(&entry.path());
                    if !hash.starts_with("ERROR:") {
                        Some(hash)
                    } else {
                        None
                    }

                })
                .collect()
        };

        // sort the hashes, because order in which fd::read_dir returns files is not consistent across platforms
        hashes.sort_unstable();
        // hash all of the hashes together
        let mut hasher = Sha256::new();
        for h in hashes {
            hasher.update(h.as_bytes());
        }

        let hash = hex::encode(hasher.finalize());

        Ok(hash)
    }

    /// Hash a file
    fn hash_file(&self, path: &Path) -> io::Result<String> {
        let file = fs::File::open(path)?;
        let mut reader = io::BufReader::new(file);
        self.hash_reader(&mut reader)
    }

    /// Hash a symlink
    ///
    /// This will hash the target path of the symlink (something like "../README.md")
    fn hash_link(&self, path: &Path) -> io::Result<String> {
        let target: PathBuf = fs::read_link(path)?;
        let target_str = target.to_string_lossy().to_string();

        let hash = hex::encode(Sha256::digest(target_str));
        Ok(hash)
    }
}

pub trait HasherProgress: Send + Sync {
    /// Will be called before all other methods once
    /// [`all_paths`] is a list of all the paths that will be hashed (not just the paths that will be returned)
    /// So if [`Hasher::paths`] is a single dir, [`all_paths`] will be a list of all the files and directories in that dir
    fn init(&self, all_paths: Vec<&Path>);
    /// Will be called every time a file is finished hashing
    fn update_file(&self, path: &Path, hash: &str);
    /// Will be called every time a few bytes are hashed
    /// [`bytes`] is the number of bytes hashed since the last call
    /// So its not the total number of bytes hashed, instead it will likely be like a few thousands every call
    fn update_bytes(&self, bytes: usize);
    /// Will be called at the end, when everything has been hashed, right before returning
    fn finish(&self);
}

pub fn result_to_hash(result: &io::Result<String>) -> String {
    match result {
        Ok(s) => s.clone(),
        Err(e) => format!("ERROR: {}", e),
    }
}

#[cfg(test)]
mod tests;
