use crossterm::style::Stylize;
use dashmap::DashMap;
use either::Either;
use indicatif::{ProgressBar, ProgressStyle};
use path_clean::PathClean;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Read, Write},
    ops::Deref,
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
    recursive: bool,
    follow_links: bool,
    use_progress_hash: bool,
    use_progress_bar: bool,
    use_color: bool,
    use_parallel: bool,
    output: Option<CleanPath>,
    force: bool,
    // these are private (no setter or getter)
    cache: Arc<DashMap<CleanPath, String>>,
    progress_bar: Option<Arc<ProgressBar>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CleanPath {
    path: PathBuf,
}

impl<P: AsRef<Path>> From<P> for CleanPath {
    fn from(value: P) -> Self {
        Self {
            path: value.as_ref().clean(),
        }
    }
}

// this makes &CleanPath become &Path
impl Deref for CleanPath {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
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
            recursive: true,
            follow_links: false,
            use_progress_hash: false,
            use_progress_bar: false,
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
    pub fn set_use_parallel(&mut self, value: bool) -> &mut Self {
        self.use_parallel = value;
        self
    }
    pub fn set_use_color(&mut self, value: bool) -> &mut Self {
        self.use_color = value;
        self
    }
    pub fn set_output<P: AsRef<Path>>(&mut self, path: Option<P>) -> &mut Self {
        self.output = path.map(CleanPath::from);
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
        self.hash_clean(&CleanPath::from(path))
    }

    pub fn hash_paths(&mut self, paths: Vec<PathBuf>) -> Vec<String> {
        let real_paths = self.get_output_paths(&paths);
        let dependencies = self.get_hash_dependencies(&real_paths);

        // create a progress bar
        if self.use_progress_bar {
            let len = dependencies.iter().fold(0, |acc, s| acc + s.len());
            let pb = ProgressBar::new(len as u64);
            // here are all style options: https://docs.rs/indicatif/0.18.4/indicatif/index.html#templates
            pb.set_style(
                ProgressStyle::with_template("[{bar:60}] {pos}/{len} {msg} {eta}")
                    .unwrap()
                    .progress_chars("=> "),
            );
            // draw the progress bar, so something like 0/69 is shown
            pb.tick();
            self.progress_bar = Some(Arc::new(pb));
        }

        // cache every single path, in such order that we never hash the same file twice
        // don't bother caching stuff if `no_hash` is true
        if !self.no_hash {
            for set in dependencies {
                replace_when! {
                    self.use_parallel,
                    set.[par_iter | iter]().for_each(|p| {
                        self.hash_clean(p);
                    })
                };
            }
        }

        // convert every path into a hash, and collect as a vector, so order is preserved
        let result: Vec<String> = replace_when! {
            self.use_parallel,
            real_paths.[par_iter | iter]().map(|path| self.hash_clean(path)).collect()
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
            let mut file = File::create(&output.path).unwrap();
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
    fn hash_clean(&self, path: &CleanPath) -> String {
        if self.no_hash {
            let result = format!("{}\n", self.path_to_string(path));
            self.handle_progress(path, &result);
            result
        } else {
            let hash = self.hash_no_error(path.clone());
            self.fmt_line(path, &hash)
        }
    }

    /// Hash a file or directory, and cache the result
    /// Return "ERROR: <error>" if there is an error
    /// THIS IS THE ONLY CACHED FUNCTION, ALL OTHER FUNCTIONS SHOULD CALL THIS FUNC TO GET THE HASH
    fn hash_no_error(&self, path: CleanPath) -> String {
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
                    let entry_path = CleanPath::from(entry.path());

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
            self.hash_reader(stdin.lock())
        };
        if let Some(pb) = self.progress_bar.as_ref() {
            pb.suspend(f)
        } else {
            f()
        }
    }

    /// Hash something that implements Read
    ///
    /// Could be a file, stdin, or anything else
    fn hash_reader(&self, mut reader: impl Read) -> io::Result<String> {
        let mut hasher = Sha256::new();

        let mut buffer = [0u8; 8192];
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// will return a list of all paths that this program should output
    /// So every path that the user wants to see (not necessarily all paths that we should hash)
    fn get_output_paths(&self, paths: &[PathBuf]) -> Vec<CleanPath> {
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

        let mut real_paths: Vec<CleanPath> = paths_not_empty
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
            .map(|p| {
                if self.absolute {
                    // canonicalize the path, or if file does not exist, join it with canonical current directory
                    p.canonicalize().unwrap_or_else(|_| {
                        std::env::current_dir()
                            .unwrap_or(PathBuf::from("."))
                            .join(p)
                    })
                } else {
                    p
                }
            })
            // clean the path, so that ./hi and ./foo/../hi both become just hi
            .map(CleanPath::from)
            // I will add / to directories in the path_to_string function
            .collect();

        real_paths.sort_unstable();
        // remove same consecutive elements, since this is sorted it will remove all duplicates
        real_paths.dedup();

        real_paths
    }

    // TODO: maybe do this in parallel (paths.par_iter())
    // get a list which says in what order the paths should be hashed
    fn get_hash_dependencies(&self, paths: &[CleanPath]) -> Vec<HashSet<CleanPath>> {
        // BTreeMap is a sorted HashMap
        let mut dependencies: BTreeMap<usize, HashSet<CleanPath>> = BTreeMap::new();
        // only directories are in this HashMap, files are immediately added to dependencies
        let mut depths: HashMap<CleanPath, usize> = HashMap::new();

        for p in paths {
            if self.is_dir_no_link(p) {
                // if this dir has already been added, then don't add it again
                if depths.contains_key(p) {
                    continue;
                }
                // if you give a file to WalkDir, it will just return it
                // by default, WalkDir will return directory before its contents (can change with `contents_first`)
                WalkDir::new(&p.path)
                    .follow_links(self.follow_links)
                    // this doesn't really matter, because I already make sure that `p`
                    // is something that needs to be followed by using `is_dir_no_link`
                    .follow_root_links(self.follow_links)
                    .into_iter()
                    .filter_entry(|e| self.all || !e.is_hidden() || e.depth() == 0)
                    .filter_map(Result::ok)
                    .for_each(|e| {
                        let path = CleanPath::from(e.path());
                        // WalkDir DirEntry will only return true if it is dir, or symlink AND `follow_links` is true
                        // more efficient than `self.is_dir_no_link(&path)` because WalkDir already knows this info without sys calls
                        if e.file_type().is_dir() {
                            depths.insert(path, 0);
                        // if this is file or file symlink or unfollowed symlink
                        } else {
                            // entry 0 is for all files or empty directories (they dont have any dependencies)
                            dependencies.entry(0).or_default().insert(path);
                            // skip the file itself
                            for (i, parent) in e.path().ancestors().enumerate().skip(1) {
                                // if someone already gave a higher depth to my parent,
                                // then that someone also gave a better depth to all of my parents, so break
                                if let Some(depth) = depths.get_mut(&CleanPath::from(parent))
                                    && *depth < i
                                {
                                    *depth = i;
                                } else {
                                    break;
                                }
                            }
                        }
                    });
            } else {
                dependencies.entry(0).or_default().insert(p.clone());
            }
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

    /// Handle progress bar / progress logs
    /// `path` has to be clean, because it will be printed
    fn handle_progress(&self, path: &CleanPath, hash: &str) {
        if let Some(pb) = &self.progress_bar {
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

    // format path and hash to be shown according to the flags
    fn fmt_line(&self, path: &CleanPath, hash: &str) -> String {
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
    fn path_to_string(&self, path: &CleanPath) -> String {
        if self.is_dir_no_link(path) {
            format!("{}/", path.display())
        } else {
            path.display().to_string()
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
