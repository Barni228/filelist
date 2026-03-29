use crossterm::style::Stylize;
use getset::{CopyGetters, Getters, MutGetters, Setters, WithSetters};
use indicatif::{ProgressBar, ProgressStyle};
use relative_path::PathExt;
use std::{
    fs::{self, File},
    io::{self, IsTerminal, Write},
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
            hash_length: 64,
            sep: String::from("  "),
            absolute: false,
            relative_to: get_current_dir(),
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
    pub fn hash_paths(&mut self, mut paths: Vec<PathBuf>) -> Vec<String> {
        // handle defaults
        if paths.is_empty() {
            // if something is piped to stdin, hash stdin
            if !io::stdin().is_terminal() {
                paths.push(PathBuf::from("-"));
            } else {
                paths.push(PathBuf::from("."));
            }
        };

        // canonicalize every path, so that every new path generated will also be canonical
        for path in paths.iter_mut() {
            *path = self.absolute_path(path);
        }

        // if user gave "-", remove it and hash stdin
        let stdin_path = PathBuf::from("-");
        let stdin_index = paths.iter().position(|p| p == &stdin_path);
        if let Some(index) = stdin_index {
            paths.remove(index);
        }
        let add_stdin: bool = stdin_index.is_some();

        // create a progress bar if needed
        self.progress_bar = if self.use_progress_bar {
            // ProgressBarUpdater will configure this progress bar later
            Some(Arc::new(ProgressBar::new(0)))
        } else {
            None
        };

        self.hasher.set_paths(paths);

        // TODO: I HATE this piece of code, but have no idea how to improve it for now (self.clone())
        let pb_updater = ProgressBarUpdater {
            fl: self.clone(),
            progress_bar_type: Arc::new(Mutex::new(self.progress_bar_type)),
        };
        self.hasher.set_progress(Arc::new(pb_updater));

        let mut result = self.hasher.start();
        if add_stdin {
            result.insert(stdin_path, hasher::result_to_hash(&self.hash_stdin()));
        };

        result
            .into_iter()
            .map(|(path, hash)| self.fmt_line(&path, &hash))
            .collect()
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

        Ok(())
    }
}

// Actual Logic, all private
impl FileList {
    /// Hash stdin
    fn hash_stdin(&self) -> io::Result<String> {
        // because I hash stdin after hashing everything else, I don't need any extra logic to suspend progress bar like before
        let stdin = io::stdin();
        // hash stdin, without letting the progress bar update
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
        if self.hasher.no_hash() {
            return format!("{path_formatted}\n");
        }

        let hash_cut = self.fmt_hash(hash);
        format!("{hash_cut}{sep}{path_formatted}\n", sep = self.sep)
    }

    fn fmt_hash<'a>(&self, hash: &'a str) -> &'a str {
        match hash.starts_with("ERROR:") {
            true => hash,
            false => &hash[0..self.hash_length],
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
        if self.hasher.is_dir_no_link(path) {
            format!("{}/", formatted.display())
        } else {
            formatted.display().to_string()
        }
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
