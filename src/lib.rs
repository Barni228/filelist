use crossterm::style::Stylize;
use dashmap::DashMap;
use either::Either;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use relative_path::PathExt;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use walkdir::WalkDir;

// TODO: I don't like this macro just being in here, maybe move it somewhere else
/// Replace `[$left | $right]` with `$left` or `$right` depending on `$when`.
/// If `$when` is true, `$left` is used, otherwise `$right` is used.
/// This is NOT recursive, so in only replaces in one level of tokens
/// this means that `[par_iter | iter]` will work, but `{ [par_iter | iter] }` will not
macro_rules! replace_when {
    ($when:expr, $($tokens:tt)*) => {
        if $when {
            replace_when!(@replace_left [] $($tokens)*)
        } else {
            replace_when!(@replace_right [] $($tokens)*)
        }
    };

    (@replace_left [ $($current:tt)* ]) => {
        $($current)*
    };
    (@replace_left [ $($current:tt)* ] [$left:ident | $right:ident] $($rest:tt)*) => {
        replace_when!(@replace_left [$($current)* $left] $($rest)*)
    };
    (@replace_left [ $($current:tt)* ] $head:tt $($rest:tt)*) => {
        replace_when!(@replace_left [$($current)* $head] $($rest)*)
    };

    (@replace_right [ $($current:tt)* ]) => {
        $($current)*
    };
    (@replace_right [ $($current:tt)* ] [$left:ident | $right:ident] $($rest:tt)*) => {
        replace_when!(@replace_right [$($current)* $right] $($rest)*)
    };
    (@replace_right [ $($current:tt)* ] $head:tt $($rest:tt)*) => {
        replace_when!(@replace_right [$($current)* $head] $($rest)*)
    };
}

const MAX_HASH_LENGTH: usize = 64; // max for SHA-256 hex

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressBarType {
    #[default]
    Auto,
    Files,
    Bytes,
}

// Arc allows me to edit something without mutable reference, and is also thread-safe
// but that something NEEDS to be thread-safe (HashMap is not, thats why I use DashMap)
/// Main configuration and execution type for file hashing.
///
/// Use setters to configure behavior, then call [`FileList::run`] to execute.
#[derive(Debug, Clone)]
pub struct FileList {
    no_hash: bool,
    hash_length: usize,
    sep: String,
    all: bool,
    hash_directory: bool,
    absolute: bool,
    relative_to: PathBuf, // relative_to is always absolute (canonicalized)
    recursive: bool,
    follow_links: bool,
    use_progress_hash: bool,
    use_progress_bar: bool,
    progress_bar_type: ProgressBarType,
    use_color: bool,
    use_parallel: bool,
    output: Option<PathBuf>,
    force: bool,
    // these are private (no setter or getter)
    cache: Arc<DashMap<PathBuf, String>>,
    progress_bar: Option<Arc<ProgressBar>>,
}

impl Default for FileList {
    fn default() -> Self {
        Self {
            no_hash: false,
            hash_length: 32,
            sep: String::from("  "),
            all: false,
            hash_directory: false,
            absolute: false,
            relative_to: get_current_dir(),
            recursive: true,
            follow_links: false,
            use_progress_hash: false,
            use_progress_bar: false,
            progress_bar_type: ProgressBarType::default(),
            use_color: false,
            use_parallel: true,
            output: None,
            force: false,
            cache: Arc::new(DashMap::new()),
            progress_bar: None,
        }
    }
}

// Constructors
impl FileList {
    /// Create a new `FileList` with default configuration.
    ///
    /// Equivalent to [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }
}

// Getters and Setters
impl FileList {
    // Getters
    pub fn no_hash(&self) -> bool {
        self.no_hash
    }
    pub fn hash_length(&self) -> usize {
        self.hash_length
    }
    pub fn sep(&self) -> &str {
        &self.sep
    }
    pub fn all(&self) -> bool {
        self.all
    }
    pub fn hash_directory(&self) -> bool {
        self.hash_directory
    }
    pub fn absolute(&self) -> bool {
        self.absolute
    }
    pub fn relative_to(&self) -> &Path {
        &self.relative_to
    }
    pub fn recursive(&self) -> bool {
        self.recursive
    }
    pub fn follow_links(&self) -> bool {
        self.follow_links
    }
    pub fn use_progress_hash(&self) -> bool {
        self.use_progress_hash
    }
    pub fn use_progress_bar(&self) -> bool {
        self.use_progress_bar
    }
    pub fn progress_bar_type(&self) -> ProgressBarType {
        self.progress_bar_type
    }
    pub fn use_parallel(&self) -> bool {
        self.use_parallel
    }
    pub fn use_color(&self) -> bool {
        self.use_color
    }
    pub fn output(&self) -> Option<&Path> {
        self.output.as_deref()
    }
    pub fn force(&self) -> bool {
        self.force
    }

    // Setters
    pub fn set_no_hash(&mut self, value: bool) -> &mut Self {
        self.no_hash = value;
        self
    }
    pub fn set_hash_length(&mut self, length: usize) -> &mut Self {
        self.hash_length = length.min(MAX_HASH_LENGTH);
        self
    }
    pub fn set_sep<S: Into<String>>(&mut self, sep: S) -> &mut Self {
        self.sep = sep.into();
        self
    }
    pub fn set_all(&mut self, value: bool) -> &mut Self {
        self.all = value;
        self
    }
    pub fn set_hash_directory(&mut self, value: bool) -> &mut Self {
        self.hash_directory = value;
        self
    }
    pub fn set_absolute(&mut self, value: bool) -> &mut Self {
        self.absolute = value;
        self
    }
    pub fn set_relative_to(&mut self, path: &Path) -> &mut Self {
        self.relative_to = self.absolute_path(path);
        self
    }
    pub fn set_recursive(&mut self, value: bool) -> &mut Self {
        self.recursive = value;
        self
    }
    pub fn set_follow_links(&mut self, value: bool) -> &mut Self {
        self.follow_links = value;
        self
    }
    pub fn set_use_progress_hash(&mut self, value: bool) -> &mut Self {
        self.use_progress_hash = value;
        self
    }
    pub fn set_use_progress_bar(&mut self, value: bool) -> &mut Self {
        self.use_progress_bar = value;
        self
    }
    pub fn set_progress_bar_type(&mut self, value: ProgressBarType) -> &mut Self {
        self.progress_bar_type = value;
        self
    }
    pub fn set_use_parallel(&mut self, value: bool) -> &mut Self {
        self.use_parallel = value;
        self
    }
    pub fn set_use_color(&mut self, value: bool) -> &mut Self {
        self.use_color = value;
        self
    }
    pub fn set_output(&mut self, path: Option<PathBuf>) -> &mut Self {
        self.output = path;
        self
    }
    pub fn set_force(&mut self, force: bool) -> &mut Self {
        self.force = force;
        self
    }
}

// Public Functions
impl FileList {
    /// Hash a single file or directory and return the formatted output line.
    ///
    /// This respects all current configuration flags.
    pub fn hash(&self, path: &Path) -> String {
        if self.no_hash {
            let result = format!("{}\n", self.path_to_string(path));
            self.handle_progress(path, &result);
            result
        } else {
            let hash = self.hash_no_error(path.to_path_buf());
            self.fmt_line(path, &hash)
        }
    }

    pub fn hash_paths(&mut self, mut paths: Vec<PathBuf>) -> Vec<String> {
        // all the paths that the user gives us are immediately converted to absolute, so all new paths i generate will also be absolute
        for path in paths.iter_mut() {
            *path = self.absolute_path(path);
        }

        let real_paths = self.get_output_paths(&paths);
        let dependencies = self.get_hash_dependencies(&real_paths);

        self.create_progress_bar(&dependencies);

        // cache every single path, in such order that we never hash the same file twice
        // don't bother caching stuff if `no_hash` is true
        if !self.no_hash {
            for set in dependencies {
                replace_when! {
                    self.use_parallel,
                    set.[par_iter | iter]().for_each(|p| {
                        self.hash(p);
                    })
                };
            }
        }

        // convert every path into a hash, and collect as a vector, so order is preserved
        let result: Vec<String> = replace_when! {
            self.use_parallel,
            real_paths.[par_iter | iter]().map(|path| self.hash(path)).collect()
        };

        result
    }

    /// Execute hashing for the provided paths.
    ///
    /// This will:
    /// - Expand directories (if recursive)
    /// - Filter hidden files (unless `all` is true)
    /// - Hash files and/or directories
    /// - Optionally show a progress bar
    /// - Print results to stdout or a file
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output file fails.
    pub fn run(&mut self, paths: Vec<PathBuf>) -> io::Result<()> {
        if let Some(output) = &self.output
            && output.exists()
            && !self.force
        {
            if self.use_color {
                eprintln!(
                    "{}: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    "Error".red(),
                    self.path_to_string(output).bold()
                );
            } else {
                eprintln!(
                    "Error: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    self.path_to_string(output)
                );
            }
            std::process::exit(1);
        }

        let result = self.hash_paths(paths);

        if let Some(output) = &self.output {
            let mut file = File::create(output).unwrap();
            for line in result {
                file.write_all(line.as_bytes()).unwrap();
            }
        } else {
            for line in result {
                self.print_respect_progress(line);
            }
        }

        if let Some(pb) = &self.progress_bar {
            pb.finish_and_clear();
        }
        Ok(())
    }
}

// Actual Logic, all private
impl FileList {
    /// Hash a file or directory, and cache the result
    /// Return "ERROR: <error>" if there is an error
    /// THIS IS THE ONLY CACHED FUNCTION, ALL OTHER FUNCTIONS SHOULD CALL THIS FUNC TO GET THE HASH
    fn hash_no_error(&self, path: PathBuf) -> String {
        if let Some(hash) = self.cache.get(&path) {
            return hash.clone();
        }

        // if we dont follow symlinks and the path is a symlink, hash the target path
        let hash_result = if path.as_os_str() == "-" {
            self.hash_stdin()
        } else if path.is_symlink() && !self.follow_links {
            self.hash_link(&path)
        } else if path.is_dir() {
            self.hash_dir(&path)
        // if this is something else, like a file, /dev/fd/* or non existing path, treat it as file
        } else {
            self.hash_file(&path)
        };

        let hash = match hash_result {
            Ok(s) => s,
            Err(e) => format!("ERROR: {}", e),
        };

        self.handle_progress(&path, &hash);
        self.cache.insert(path, hash.clone());

        hash
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
                    let entry_path = entry.path();

                    // ignore the output file, because we cannot hash it since we dont know what it is yet
                    if self.output.as_ref() == Some(&entry_path) {
                        return None;
                    }

                    let hash = self.hash_no_error(entry_path);
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
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
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

    /// Hash stdin
    fn hash_stdin(&self) -> io::Result<String> {
        let f = || {
            let stdin = io::stdin();
            let hash = self.hash_reader_update_progress(stdin.lock(), false);
            println!();
            hash
        };
        if let Some(pb) = self.progress_bar.as_ref() {
            // do not update progress bar while reading from stdin (otherwise it will freeze)
            // when suspending, any method on pb will freeze until `f` finishes
            // but `f` will call `self.handle_progress_bytes` which will update the progress bar, and thus freeze
            pb.suspend(f)
        } else {
            f()
        }
    }

    fn hash_reader_update_progress(
        &self,
        mut reader: impl Read,
        update: bool,
    ) -> io::Result<String> {
        let mut hasher = Sha256::new();

        let mut buffer = [0u8; 8192];
        loop {
            let bytes = reader.read(&mut buffer)?;
            if bytes == 0 {
                break;
            }
            if update {
                self.handle_progress_bytes(bytes as u64);
            }
            hasher.update(&buffer[..bytes]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// Hash something that implements Read
    ///
    /// Could be a file, stdin, or anything else
    fn hash_reader(&self, reader: impl Read) -> io::Result<String> {
        self.hash_reader_update_progress(reader, true)
    }

    /// will return a list of all paths that this program should output
    /// So every path that the user wants to see (not necessarily all paths that we should hash)
    fn get_output_paths(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        // this is kind of weird because the type of paths is &[PathBuf], so i cannot modify it
        // and i have to deal with lifetimes since it is a reference
        let mut default = [PathBuf::default()];
        let paths_not_empty = if paths.is_empty() {
            // if something is piped to stdin, hash stdin
            if !io::stdin().is_terminal() {
                default[0] = PathBuf::from("-")
            } else {
                default[0] = PathBuf::from(".")
            }
            &default
        } else {
            paths
        };

        let mut real_paths: Vec<PathBuf> = paths_not_empty
            .iter()
            .flat_map(|p| {
                // "-" is a special argument that means stdin
                if self.recursive && p.is_dir() && p.as_os_str() != "-" {
                    // either allows two iterators to be the same type
                    Either::Left(
                        WalkDir::new(p)
                            .follow_links(self.follow_links)
                            .follow_root_links(self.follow_links)
                            .into_iter()
                            // filter out hidden files if `all` is not set, and if they are not the root
                            // so if the user gives .dir, I will include it even without --all
                            .filter_entry(|e| self.all || !e.is_hidden() || e.depth() == 0)
                            .filter_map(Result::ok)
                            // filter out directories if --directory is not set
                            .filter(|e| self.hash_directory || !e.file_type().is_dir())
                            .map(|e| e.into_path()), // convert to PathBuf
                    )
                } else {
                    Either::Right(std::iter::once(p.clone()))
                }
            })
            // I will add / to directories in the path_to_string function
            .collect();

        real_paths.sort_unstable();
        // remove same consecutive elements, since this is sorted it will remove all duplicates
        real_paths.dedup();

        real_paths
    }

    // TODO: maybe do this in parallel (paths.par_iter())
    // get a list which says in what order the paths should be hashed
    fn get_hash_dependencies(&self, paths: &[PathBuf]) -> Vec<HashSet<PathBuf>> {
        // BTreeMap is a sorted HashMap
        let mut dependencies: BTreeMap<usize, HashSet<PathBuf>> = BTreeMap::new();
        // only directories are in this HashMap, files are immediately added to dependencies
        let mut depths: HashMap<PathBuf, usize> = HashMap::new();

        for p in paths {
            if !self.is_dir_no_link(p) {
                dependencies.entry(0).or_default().insert(p.clone());
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

    /// Return true if the path is a dir, or a followed symlink to a dir
    #[inline]
    fn is_dir_no_link(&self, path: &Path) -> bool {
        path.is_dir() && (self.follow_links || !path.is_symlink())
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        if path.as_os_str() == "-" {
            return Ok(0);
        }
        let metadata = match self.follow_links {
            true => fs::metadata(path)?,
            false => fs::symlink_metadata(path)?,
        };
        if metadata.is_file() || metadata.is_symlink() {
            Ok(metadata.len())
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} is not a file", path.display()),
            ))
        }
    }

    /// Create a progress bar if `use_progress_bar` is true
    fn create_progress_bar(&mut self, dependencies: &[HashSet<PathBuf>]) {
        if !self.use_progress_bar {
            return;
        }
        // how many entries to hash
        let len: usize = dependencies.iter().fold(0, |acc, s| acc + s.len());

        if self.progress_bar_type == ProgressBarType::Auto {
            self.progress_bar_type = match len {
                ..=100 => ProgressBarType::Bytes,
                _ => ProgressBarType::Files,
            };
        };

        let pb = match self.progress_bar_type {
            ProgressBarType::Files => {
                let pb = ProgressBar::new(len as u64);
                // here are all style options: https://docs.rs/indicatif/0.18.4/indicatif/index.html#templates
                pb.set_style(
                    ProgressStyle::with_template("[{bar:60}] {pos}/{len} {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
                pb
            }
            ProgressBarType::Bytes => {
                // find the total number of bytes for all the files
                let total: u64 = dependencies[0]
                    .iter()
                    .fold(0, |acc, file| acc + self.file_size(file).unwrap_or(0));

                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::with_template("[{bar:60}] ({bytes}) / ({total_bytes}) {eta}")
                        .unwrap()
                        .progress_chars("=> "),
                );
                pb
            }
            _ => unreachable!(),
        };

        // draw the progress bar, so something like 0/69 is shown
        pb.tick();
        self.progress_bar = Some(Arc::new(pb));
    }

    /// Handle progress bar / progress logs
    /// `path` has to be clean, because it will be printed
    fn handle_progress(&self, path: &Path, hash: &str) {
        if self.use_progress_bar
            && self.progress_bar_type == ProgressBarType::Files
            && let Some(pb) = &self.progress_bar
        {
            pb.inc(1);
        }

        if self.use_progress_hash {
            if self.use_color {
                self.eprint_respect_progress(self.fmt_line(path, hash).yellow().dim());
            } else {
                self.eprint_respect_progress(self.fmt_line(path, hash));
            }
        }
    }

    fn handle_progress_bytes(&self, bytes: u64) {
        if self.use_progress_bar
            && self.progress_bar_type == ProgressBarType::Bytes
            // make sure that we should be updating the progress bar
            && let Some(pb) = self.progress_bar.as_ref()
        {
            pb.inc(bytes);
        }
    }

    /// canonicalize the given [`path`], even if it doesn't exist
    ///
    /// If `path` is `-`, return `-`
    fn absolute_path(&self, path: &Path) -> PathBuf {
        if path.as_os_str() == "-" {
            return PathBuf::from("-");
        }
        // canonicalize the path, or if file does not exist, join it with canonical current directory
        path.canonicalize()
            .unwrap_or_else(|_| get_current_dir().join(path))
    }
    // format path and hash to be shown according to the flags
    fn fmt_line(&self, path: &Path, hash: &str) -> String {
        let path_formatted = self.path_to_string(path);
        if self.no_hash {
            return format!("{path_formatted}\n");
        }

        let hash_cut = match hash.starts_with("ERROR:") {
            true => hash,
            false => &hash[0..self.hash_length],
        };
        format!("{hash_cut}{sep}{path_formatted}\n", sep = self.sep)
    }

    /// if I print regularly, text will combine with the progress bar and make everything weird
    /// so text will be like
    /// abc123 file.txt====>    ] 0/69
    /// Note: s should end with `\n`
    fn print_to_respect_progress(
        &self,
        out: &mut impl Write,
        s: impl std::fmt::Display,
    ) -> io::Result<()> {
        if let Some(pb) = self.progress_bar.as_ref() {
            // suspend will remove the progress bar, execute something, and then put it back
            pb.suspend(|| write!(out, "{}", s))?;
        } else {
            // if there is no progress bar, just print regularly
            write!(out, "{}", s)?;
        };
        Ok(())
    }

    fn print_respect_progress(&self, s: impl std::fmt::Display) {
        self.print_to_respect_progress(&mut io::stdout(), s)
            .unwrap();
    }

    fn eprint_respect_progress(&self, s: impl std::fmt::Display) {
        self.print_to_respect_progress(&mut io::stderr(), s)
            .unwrap();
    }

    /// Convert a path into its display form.
    ///
    /// Directories are suffixed with `/`.
    fn path_to_string(&self, path: &Path) -> String {
        if path.as_os_str() == "-" {
            return String::from("-");
        }
        let formatted = if self.absolute {
            path.to_path_buf()
        } else {
            let relative = path.relative_to(&self.relative_to).unwrap().to_path("");
            if relative.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                relative
            }
        };

        // if the ORIGINAL is a directory, add a `/`
        // because of formatting, `formatted` could be invalid path
        // since it is relative to `self.relative_to`, and `self.is_dir_no_link` doesn't know about `self.relative_to`
        if self.is_dir_no_link(path) {
            format!("{}/", formatted.display())
        } else {
            formatted.display().to_string()
        }
    }
}

fn get_current_dir() -> PathBuf {
    let path = std::env::current_dir();
    match path {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            std::process::exit(e.raw_os_error().unwrap_or(1));
        }
    }
}

trait IsHidden {
    fn is_hidden(&self) -> bool;
}

impl IsHidden for walkdir::DirEntry {
    fn is_hidden(&self) -> bool {
        self.file_name()
            .to_str()
            .map(|s| s.starts_with('.') && s.len() > 1)
            .unwrap_or(false)
    }
}

impl IsHidden for fs::DirEntry {
    fn is_hidden(&self) -> bool {
        self.file_name()
            .to_str()
            .map(|s| s.starts_with('.') && s.len() > 1)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests;
