#![doc = include_str!("../README.md")]

use crossterm::{
    queue,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};
use either::Either;
use path_clean::PathClean;
use progress_bar::pb::ProgressBar;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::PathBuf,
    vec,
};
use walkdir::WalkDir;

const MAX_HASH_LENGTH: usize = 64; // max for SHA-256 hex

/// Main configuration and execution type for file hashing.
///
/// Use setters to configure behavior, then call [`FileList::run`] to execute.
pub struct FileList {
    no_hash: bool,
    hash_length: usize,
    sep: String,
    all: bool,
    always_print: bool,
    hash_directory: bool,
    recursive: bool,
    use_progress_hash: bool,
    use_progress_bar: bool,
    use_color: bool,
    output: Option<PathBuf>,
    cache: HashMap<PathBuf, String>,
    progress_bar: Option<ProgressBar>,
}

impl Default for FileList {
    fn default() -> Self {
        Self {
            no_hash: false,
            hash_length: 32,
            sep: String::from("  "),
            all: false,
            always_print: false,
            hash_directory: false,
            recursive: true,
            use_progress_hash: false,
            use_progress_bar: false,
            use_color: false,
            output: None,
            cache: HashMap::new(),
            progress_bar: None,
        }
    }
}

impl FileList {
    /// Create a new `FileList` with default configuration.
    ///
    /// Equivalent to [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn always_print(&self) -> bool {
        self.always_print
    }
    pub fn hash_directory(&self) -> bool {
        self.hash_directory
    }
    pub fn recursive(&self) -> bool {
        self.recursive
    }
    pub fn use_progress_hash(&self) -> bool {
        self.use_progress_hash
    }
    pub fn use_progress_bar(&self) -> bool {
        self.use_progress_bar
    }
    pub fn use_color(&self) -> bool {
        self.use_color
    }
    pub fn output(&self) -> Option<&PathBuf> {
        self.output.as_ref()
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
    pub fn set_always_print(&mut self, value: bool) -> &mut Self {
        self.always_print = value;
        self
    }
    pub fn set_hash_directory(&mut self, value: bool) -> &mut Self {
        self.hash_directory = value;
        self
    }
    pub fn set_recursive(&mut self, value: bool) -> &mut Self {
        self.recursive = value;
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
    pub fn set_use_color(&mut self, value: bool) -> &mut Self {
        self.use_color = value;
        self
    }
    pub fn set_output<P: Into<PathBuf>>(&mut self, path: Option<P>) -> &mut Self {
        self.output = path.map(|p| p.into().clean());
        self
    }

    /// Hash a single file or directory and return the formatted output line.
    ///
    /// This respects all current configuration flags.
    pub fn hash(&mut self, path: &PathBuf) -> String {
        if self.no_hash {
            let result = format!("{}\n", self.path_to_string(path));
            self.handle_progress(path, &result);
            result
        } else {
            let hash = self.hash_no_error(path);
            self.fmt_line(path, &hash)
        }
    }

    /// Execute hashing for the provided paths.
    ///
    /// This will:
    /// - Expand directories (if recursive)
    /// - Filter hidden files (unless `--all`)
    /// - Hash files and/or directories
    /// - Optionally show a progress bar
    /// - Print results to stdout or a file
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output file fails.
    pub fn run(&mut self, paths: Vec<PathBuf>) -> io::Result<()> {
        let real_paths = self.get_all_paths(paths);
        if self.use_progress_bar {
            // create a progress bar
            let pb = ProgressBar::new(real_paths.len());
            // show it, so something like 0/69 is shown
            pb.display();
            self.progress_bar = Some(pb);
        }

        if let Some(output) = self.output.as_ref() {
            let mut file = File::create(output).unwrap();
            for path in real_paths {
                let line = self.hash(&path);
                file.write_all(line.as_bytes()).unwrap();
                if self.always_print {
                    self.print_respect_progress(line);
                }
            }
        } else {
            for path in real_paths {
                let line = self.hash(&path);
                self.print_respect_progress(line);
            }
        }

        // if you don't finalize it, it will disappear after the program finishes
        // if let Some(pb) = self.progress_bar.as_mut() {
        //     pb.finalize();
        // }
        Ok(())
    }

    /// Hash a file
    fn hash_file(&mut self, path: &PathBuf) -> io::Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();

        let mut buffer = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buffer) {
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let hash = hex::encode(hasher.finalize());

        Ok(hash)
    }

    /// Hash a directory <br>
    /// The way this works is: <br>
    /// it will hash everything inside of the directory,
    /// sort all of those hashes, and then hash them all together
    fn hash_dir(&mut self, path: &PathBuf) -> io::Result<String> {
        let mut hashes = vec![];
        for entry in fs::read_dir(path)?.filter_map(Result::ok) {
            if !self.all && entry.is_hidden() {
                continue;
            }
            let path = entry.path().clean();

            // ignore the output file, because we cannot hash it since we dont know what it is yet
            if self.output.as_ref() == Some(&path) {
                continue;
            }

            let hash = self.hash_no_error(&path);
            if !hash.starts_with("ERROR:") {
                hashes.push(hash);
            }
        }

        hashes.sort_unstable();

        let hash = hex::encode(Sha256::digest(hashes.join("").as_bytes()));

        Ok(hash)
    }

    /// Hash a file or directory, and cache the result
    /// Return "ERROR: <error>" if there is an error
    fn hash_no_error(&mut self, path: &PathBuf) -> String {
        if let Some(hash) = self.cache.get(path) {
            return hash.clone();
        }
        let hash = if path.is_dir() {
            match self.hash_dir(path) {
                Ok(s) => s,
                Err(e) => format!("ERROR: {}", e),
            }
        } else {
            match self.hash_file(path) {
                Ok(s) => s,
                Err(e) => format!("ERROR: {}", e),
            }
        };

        self.cache.insert(path.clone(), hash.clone());
        self.handle_progress(path, &hash);

        hash
    }

    fn get_all_paths(&self, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut real_paths: Vec<PathBuf> = paths
            .iter()
            .flat_map(|p| {
                if self.recursive && p.is_dir() {
                    // either allows two iterators to be the same type
                    Either::Left(
                        WalkDir::new(p)
                            // don't return the directory itself
                            .min_depth(1)
                            .into_iter()
                            // filter out hidden files if --all is not set
                            .filter_entry(|e| self.all || !e.is_hidden())
                            .filter_map(|e| e.ok().map(|e| e.into_path())) // convert to PathBuf
                            // add the directory itself after the hidden files check
                            // so if the user gave us .dir, we will include it even without --all
                            .chain(std::iter::once(p.clone()))
                            // filter out directories if --directory is not set
                            .filter(|p| self.hash_directory || !p.is_dir()),
                    )
                } else {
                    // clone because clap doesn't give us the ownership over the path
                    Either::Right(std::iter::once(p.clone()))
                }
            })
            // clean the path, so that ./hi and ./foo/../hi both become just hi
            // needs path_clean crate
            .map(|p| p.clean())
            // I will add / to directories in the path_to_string function
            .collect();

        real_paths.sort_unstable();
        // remove same consecutive elements, since this is sorted it will remove all duplicates
        real_paths.dedup();

        real_paths
    }

    /// Handle progress bar / progress logs
    fn handle_progress(&mut self, path: &PathBuf, hash: &str) {
        if let Some(pb) = self.progress_bar.as_mut() {
            pb.inc(); // increment the progress bar
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
    fn fmt_line(&self, path: &PathBuf, hash: &str) -> String {
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

    /// Convert a path into its display form.
    ///
    /// Directories are suffixed with `/`.
    pub fn path_to_string(&self, path: &PathBuf) -> String {
        if path.is_dir() {
            format!("{}/", path.display())
        } else {
            path.display().to_string()
        }
    }

    /// if I print regularly, text will combine with the progress bar and make everything weird
    /// so text will be like
    /// abc123 file.txt====>    ] 0/69
    /// Note: s should end with `\n`, otherwise weird stuff will happen
    fn print_to_respect_progress(
        &self,
        out: &mut impl Write,
        s: impl std::fmt::Display,
    ) -> io::Result<()> {
        if let Some(pb) = self.progress_bar.as_ref() {
            // clear the old progress bar, and print s
            queue!(out, Clear(ClearType::UntilNewLine), Print(s))?;
            // re-print the progress bar again, this will also probably flush the stdout, so queue above is fine
            pb.display();
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
