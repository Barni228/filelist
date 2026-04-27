use crossterm::style::Stylize;
use getset::{CopyGetters, Getters, MutGetters, Setters, WithSetters};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

mod hasher;
mod helper;

use crate::hasher::Hasher;

const MAX_HASH_LENGTH: usize = 64; // max for SHA-256 hex

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressBarType {
    #[default]
    Auto,
    Files,
    Bytes,
}

/// Main configuration and execution type for file hashing.
///
/// Use setters to configure behavior, then call [`FileList::run`] to execute.
#[derive(Debug, Clone, Getters, Setters, WithSetters, MutGetters, CopyGetters)]
pub struct FileList {
    /// The length of the hash, from 0 to [`MAX_HASH_LENGTH`].
    #[getset(get_copy = "pub")]
    hash_length: usize,

    /// The separator between the hash and the path.
    #[getset(get = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    sep: String,

    /// if true, all paths will be absolute (canonicalized)
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    absolute: bool,

    /// If absolute is false, all paths will be relative to this path
    #[getset(get = "pub")]
    relative_to: PathBuf, // relative_to is always absolute (canonicalized)

    /// if true, will return `./hi` instead of `hi`
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    use_dot_prefix: bool,

    /// If Some, include stdin in the output, labeled with this string as a path (usually `"-"``)
    /// Example:
    /// ```
    /// use filelist::FileList;
    /// assert!(
    ///     FileList::new()
    ///         .with_include_stdin(Some("-".to_string()))
    ///         .hash_all::<&str>(&[]).contains_key("-")
    /// );
    /// ```
    #[getset(get = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    include_stdin: Option<String>,

    /// If true, print what has been hashed so far to stderr
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    use_progress_hash: bool,

    /// If true, print a progress bar
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    use_progress_bar: bool,

    /// The type of progress bar to use
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    progress_bar_type: ProgressBarType,

    /// If true, print colored output
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    use_color: bool,

    /// The path to write the output to
    #[getset(get = "pub")]
    output: Option<PathBuf>,

    /// If true, overwrite existing output file
    #[getset(get_copy = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    force: bool,

    /// The hasher to use, see [`Hasher`] for additional configuration
    #[getset(get = "pub", set = "pub", get_mut = "pub", set_with = "pub")]
    hasher: Hasher,

    // these are private (no setter or getter)
    progress_bar: Option<Arc<ProgressBar>>,
}

impl Default for FileList {
    fn default() -> Self {
        Self {
            include_stdin: None,
            hash_length: 64,
            sep: String::from("  "),
            absolute: false,
            relative_to: get_current_dir(),
            use_dot_prefix: false,
            use_progress_hash: false,
            use_progress_bar: false,
            progress_bar_type: ProgressBarType::default(),
            use_color: false,
            output: None,
            force: false,
            hasher: Hasher::default()
                .with_no_hash(false)
                .with_all(false)
                .with_hash_directory(false)
                .with_recursive(true)
                .with_follow_links(false)
                .with_use_parallel(true),
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

// Special Getters and Setters
impl FileList {
    pub fn set_hash_length(&mut self, length: usize) -> &mut Self {
        if length <= MAX_HASH_LENGTH {
            self.hash_length = length;
            self
        } else {
            panic!(
                "Hash length must be between 0 and {} (inclusive)",
                MAX_HASH_LENGTH
            );
        }
    }

    pub fn set_relative_to(&mut self, path: &Path) -> &mut Self {
        self.relative_to = self.absolute_path(path);
        self
    }

    pub fn set_output(&mut self, path: Option<&Path>) -> &mut Self {
        self.output = path.map(|p| self.absolute_path(p));
        self
    }

    pub fn with_hash_length(mut self, length: usize) -> Self {
        self.set_hash_length(length);
        self
    }

    pub fn with_relative_to(mut self, path: &Path) -> Self {
        self.set_relative_to(path);
        self
    }

    pub fn with_output(mut self, path: Option<&Path>) -> Self {
        self.set_output(path);
        self
    }
}

// Public Functions
impl FileList {
    // NOTE: BTreeMap returned by this function is sorted by relative path, not by absolute path
    /// Hash the paths and return a BTreeMap of paths to hashes
    /// This will only hash the `paths`, and not the [`FileList::include_stdin`],
    /// use [`FileList::hash_all`] if you want to hash everything
    /// ```
    /// use filelist::FileList;
    /// use std::collections::BTreeMap;
    /// use std::path::PathBuf;
    ///
    /// assert_eq!(
    ///     FileList::new().hash_paths(&["README.md"]),
    ///     BTreeMap::from([(
    ///         PathBuf::from("README.md"),
    ///         String::from("0e8d5acebaffa8a97378b315f4204006458f0ae793c4a8e5a29b6134dffed4c4")
    ///     )])
    /// );
    /// ```
    pub fn hash_paths<P: AsRef<Path>>(&mut self, paths: &[P]) -> BTreeMap<PathBuf, String> {
        let result: BTreeMap<_, _> = self
            .hash_paths_raw(paths)
            .into_iter()
            .map(|(path, hash)| {
                (
                    self.handle_relative(&path),
                    self.fmt_hash(&hash).to_string(),
                )
            })
            .collect();

        result
    }

    // NOTE: BTreeMap returned by this function is sorted by relative formatted path, not by absolute path
    /// Hash the paths and return a BTreeMap of paths to hashes
    /// This will hash the `paths` and the [`FileList::include_stdin`]
    /// Paths will be formatted as Strings, with all of the formatting options applied (like adding `/` to directories)
    /// ```
    /// use filelist::FileList;
    /// use std::collections::BTreeMap;
    /// use std::path::PathBuf;
    ///
    /// assert_eq!(
    ///     FileList::new().hash_all(&["README.md"]),
    ///     BTreeMap::from([(
    ///         "README.md".to_string(),
    ///         "0e8d5acebaffa8a97378b315f4204006458f0ae793c4a8e5a29b6134dffed4c4".to_string()
    ///     )])
    /// );
    /// ```
    pub fn hash_all<P: AsRef<Path>>(&mut self, paths: &[P]) -> BTreeMap<String, String> {
        let mut result: BTreeMap<String, String> = self
            .hash_paths(paths)
            .into_iter()
            .map(|(path, hash)| (self.fmt_formatted_path(&path), hash))
            .collect();

        if let Some(stdin) = &self.include_stdin {
            result.insert(
                stdin.to_string(),
                hasher::result_to_hash(&self.hash_stdin()),
            );
        };
        result
    }

    /// Hash the paths and return a Vec of formatted lines, ready to be printed
    /// ```
    /// use filelist::FileList;
    /// use std::path::PathBuf;
    ///
    /// assert_eq!(
    ///     FileList::new().hash_all_lines(&["README.md"]),
    ///     vec!["0e8d5acebaffa8a97378b315f4204006458f0ae793c4a8e5a29b6134dffed4c4  README.md\n".to_string()],
    /// );
    /// ```
    pub fn hash_all_lines<P: AsRef<Path>>(&mut self, paths: &[P]) -> Vec<String> {
        self.hash_all(paths)
            .into_iter()
            .map(|(path_str, hash)| self.join_path_hash(path_str, hash))
            .collect()
    }

    /// Hash the paths, and write the output to [`FileList::output`] or `stdout`
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the output file fails.
    pub fn run<P: AsRef<Path>>(&mut self, paths: &[P]) -> io::Result<()> {
        if let Some(output) = &self.output
            && output.exists()
            && !self.force
        {
            if self.use_color {
                eprintln!(
                    "{}: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    "Error".red(),
                    self.fmt_path(output).bold()
                );
            } else {
                eprintln!(
                    "Error: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    self.fmt_path(output)
                );
            }
            std::process::exit(1);
        }

        let result = self.hash_all_lines(paths);

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

        Ok(())
    }
}

// Actual Logic, all private
impl FileList {
    // NOTE: BTreeMap that this returns is sorted by PathBuf (absolute path), which is different than sorting by relative path
    /// Hash the paths and return a BTreeMap of paths to hashes
    /// This will NOT return formatted output, so paths will not be relative and hash will not be trimmed
    /// stdin will NOT be included
    fn hash_paths_raw<P: AsRef<Path>>(&mut self, paths: &[P]) -> BTreeMap<PathBuf, String> {
        // canonicalize every path, so that every new path generated will also be canonical
        let abs_paths: Vec<PathBuf> = paths.iter().map(|path| self.absolute_path(path)).collect();

        // create a progress bar if needed
        self.progress_bar = if self.use_progress_bar {
            // ProgressBarUpdater will configure this progress bar later
            Some(Arc::new(ProgressBar::new(0)))
        } else {
            None
        };

        self.hasher.set_paths(abs_paths);

        // TODO: I HATE this piece of code, but have no idea how to improve it for now (self.clone())
        let pb_updater = ProgressBarUpdater {
            fl: self.clone(),
            progress_bar_type: Arc::new(Mutex::new(self.progress_bar_type)),
        };
        self.hasher.set_progress(Arc::new(pb_updater));

        self.hasher.start()
    }

    /// Hash stdin
    fn hash_stdin(&self) -> io::Result<String> {
        // because I hash stdin after hashing everything else, I don't need any extra logic to suspend progress bar like before
        let stdin = io::stdin();
        let hash = self.hasher.hash_reader(stdin.lock());
        println!();
        hash
    }

    fn file_size(&self, path: &Path) -> io::Result<u64> {
        let metadata = match self.hasher.follow_links() {
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

    fn handle_progress_files(&self, path: &Path, hash: &str) {
        if self.use_progress_hash {
            if self.use_color {
                self.eprint_respect_progress(self.fmt_line(path, hash).yellow().dim());
            } else {
                self.eprint_respect_progress(self.fmt_line(path, hash));
            }
        }
    }

    /// canonicalize the given [`path`], even if it doesn't exist
    fn absolute_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        // canonicalize the path, or if file does not exist, join it with canonical current directory
        path.as_ref()
            .canonicalize()
            .unwrap_or_else(|_| get_current_dir().join(path))
    }

    fn join_path_hash(&self, path: String, hash: String) -> String {
        if self.hasher.no_hash() {
            format!("{path}\n")
        } else {
            format!("{hash}{sep}{path}\n", sep = self.sep)
        }
    }

    // format path and hash to be shown according to the flags
    fn fmt_line(&self, path: &Path, hash: &str) -> String {
        let path_formatted = self.fmt_path(path);

        if self.hasher.no_hash() {
            return format!("{path_formatted}\n");
        }

        let hash_cut = self.fmt_hash(hash);
        format!("{hash_cut}{sep}{path_formatted}\n", sep = self.sep)
    }

    /// Format hash to be shown according to the flags
    fn fmt_hash<'a>(&self, hash: &'a str) -> &'a str {
        if self.hasher.no_hash() {
            return "";
        }
        match hash.starts_with("ERROR:") {
            true => hash,
            false => &hash[0..self.hash_length],
        }
    }

    /// Make a path be relative to what it needs to be relative to
    /// This is a very cheap operation
    fn handle_relative(&self, abs_path: &Path) -> PathBuf {
        if self.absolute {
            abs_path.to_path_buf()
        } else {
            // this does not make any sys calls
            let base = match self.use_dot_prefix {
                true => ".",
                false => "",
            };
            let relative = pathdiff::diff_paths(abs_path, &self.relative_to).unwrap();
            if relative.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                PathBuf::from(base).join(relative)
            }
        }
    }

    fn fmt_formatted_path(&self, rel_path: &Path) -> String {
        let abs_path = if self.absolute {
            rel_path
        } else {
            &self.relative_to.join(rel_path)
        };

        if self.hasher.is_dir_no_link(abs_path) {
            format!("{}/", rel_path.display())
        } else {
            rel_path.display().to_string()
        }
    }
    /// Convert a path into its display form.
    ///
    /// Directories are suffixed with `/`. All paths are relative to `self.relative_to`.
    fn fmt_path(&self, abs_path: &Path) -> String {
        let formatted = self.handle_relative(abs_path);
        // if the ORIGINAL is a directory, add a `/`
        // because of formatting, `formatted` could be invalid path
        // since it is relative to `self.relative_to`, and `self.is_dir_no_link` doesn't know about `self.relative_to`
        if self.hasher.is_dir_no_link(abs_path) {
            format!("{}/", formatted.display())
        } else {
            formatted.display().to_string()
        }
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
        // ignore any errors that might happen while printing, like broken pipe (pressing `q` in `less`)
        let _ = self.print_to_respect_progress(&mut io::stdout(), s);
    }

    fn eprint_respect_progress(&self, s: impl std::fmt::Display) {
        let _ = self.print_to_respect_progress(&mut io::stderr(), s);
    }
}

struct ProgressBarUpdater {
    fl: FileList,
    progress_bar_type: Arc<Mutex<ProgressBarType>>,
}

impl hasher::HasherProgress for ProgressBarUpdater {
    /// Set up a progress bar
    fn init(&self, all_paths: Vec<&Path>) {
        if let Some(pb) = &self.fl.progress_bar {
            let mut pb_type = self.progress_bar_type.lock().unwrap();
            // how many entries to hash
            let len: usize = all_paths.len();
            if *pb_type == ProgressBarType::Auto {
                *pb_type = match len {
                    ..=100 => ProgressBarType::Bytes,
                    _ => ProgressBarType::Files,
                };
            }

            match *pb_type {
                ProgressBarType::Files => {
                    pb.set_length(len as u64);
                    // here are all style options: https://docs.rs/indicatif/0.18.4/indicatif/index.html#templates
                    pb.set_style(
                        ProgressStyle::with_template("[{bar:60}] {pos}/{len} {eta}")
                            .unwrap()
                            .progress_chars("=> "),
                    );
                }
                ProgressBarType::Bytes => {
                    // find the total number of bytes for all the files
                    let total: u64 = all_paths
                        .iter()
                        // .filter(|f| f.is_file())
                        .fold(0, |acc, file| acc + self.fl.file_size(file).unwrap_or(0));

                    pb.set_length(total);
                    pb.set_style(
                        ProgressStyle::with_template(
                            "[{bar:60}] ({bytes}) / ({total_bytes}) {eta}",
                        )
                        .unwrap()
                        .progress_chars("=> "),
                    );
                }
                _ => unreachable!(),
            };

            // draw the progress bar, so something like 0/69 is shown
            pb.tick();
        }
    }

    /// Handle progress bar / progress logs
    /// `path` has to be clean, because it will be printed
    fn update_file(&self, path: &Path, hash: &str) {
        if self.fl.use_progress_bar
            && *self.progress_bar_type.lock().unwrap() == ProgressBarType::Files
            && let Some(pb) = &self.fl.progress_bar
        {
            pb.inc(1);
        }
        self.fl.handle_progress_files(path, hash);
    }
    fn update_bytes(&self, bytes: usize) {
        if self.fl.use_progress_bar
            && *self.progress_bar_type.lock().unwrap() == ProgressBarType::Bytes
            // make sure that we should be updating the progress bar
            && let Some(pb) = &self.fl.progress_bar
        {
            pb.inc(bytes as u64);
        }
    }

    fn finish(&self) {
        if let Some(pb) = &self.fl.progress_bar {
            pb.finish_and_clear();
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
