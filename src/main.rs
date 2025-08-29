use cached::proc_macro::cached;
use clap::{arg, command, value_parser};
use either::Either;
use path_clean::PathClean;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::PathBuf;
use std::vec;
use walkdir::WalkDir;

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

const HASH_LENGTH: i32 = 64;

fn main() {
    let matches = get_clap_command().get_matches();

    let hash_length = match *matches.get_one::<i32>("length").unwrap() {
        -1 => HASH_LENGTH,
        l => l,
    };
    let no_hash = matches.get_flag("no-hash");
    let sep = matches.get_one::<String>("separator").unwrap();
    let all = matches.get_flag("all");
    let hash_directory = matches.get_flag("directory");
    let recursive = !matches.get_flag("no-recursive");

    let mut paths = matches
        .get_many::<PathBuf>("PATHS")
        .unwrap()
        .flat_map(|p| {
            if recursive && p.is_dir() {
                // either allows two iterators to be the same type
                Either::Left(
                    WalkDir::new(p)
                        // don't return the directory itself
                        .min_depth(1)
                        .into_iter()
                        // filter out hidden files if --all is not set
                        .filter_entry(|e| all || !e.is_hidden())
                        .filter_map(|e| e.ok().map(|e| e.into_path()))
                        // add the directory itself after the hidden files check
                        // so if the user gave us .dir, we will include it even without --all
                        .chain(std::iter::once(p.clone()))
                        // filter out directories if --directory is not set
                        .filter(|p| hash_directory || !p.is_dir()),
                )
            } else {
                // clone because clap doesn't give us the ownership over the path
                Either::Right(std::iter::once(p.clone()))
            }
        })
        // clean the path, so that ./hi and ./foo/../hi both become just hi
        // needs path_clean crate
        .map(|p| p.clean())
        .collect::<Vec<_>>();

    paths.sort_unstable();
    // remove same consecutive elements, since this is sorted it will remove all duplicates
    paths.dedup();

    let lines = paths.into_iter().map(|p| {
        let path = p.display().to_string();
        if no_hash {
            format!("{}\n", path)
        } else {
            format!(
                "{}{}{}\n",
                &hash_no_error(p, hash_length as usize, all),
                sep,
                path
            )
        }
    });

    if let Some(output) = matches.get_one::<PathBuf>("output") {
        let mut file = File::create(output).unwrap();
        for line in lines {
            file.write_all(line.as_bytes()).unwrap();
            if matches.get_flag("print") {
                print!("{}", line);
            }
        }
    } else {
        for line in lines {
            print!("{}", line);
        }
    }
}

// cache by path, and only remember successful hashes, Errors are not cached
// you can remove size to cache all files, but you need to keep result = true if your function returns a Result
// to cache this, you need to either take owned objects, or 'static objects
#[cached(result = true)]
fn hash_file(path: PathBuf) -> io::Result<String> {
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

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cached(result = true)]
fn hash_dir(path: PathBuf, use_hidden: bool) -> io::Result<String> {
    let mut hashes = vec![];
    for entry in fs::read_dir(path)?.filter_map(Result::ok) {
        if !use_hidden && entry.is_hidden() {
            continue;
        }
        let path = entry.path().clean();
        if path.is_dir() {
            hashes.push(hash_dir(path, use_hidden)?);
        } else {
            if let Ok(hash) = hash_file(path) {
                hashes.push(hash);
            }
        }
    }

    hashes.sort_unstable();

    Ok(hex::encode(Sha256::digest(hashes.join("").as_bytes())))
}

fn hash_no_error(path: PathBuf, len: usize, dir_use_hidden: bool) -> String {
    if path.is_dir() {
        match hash_dir(path, dir_use_hidden) {
            Ok(s) => s.chars().take(len as usize).collect(),
            Err(e) => format!("ERROR: {}", e),
        }
    } else {
        match hash_file(path) {
            Ok(s) => s.chars().take(len as usize).collect(),
            Err(e) => format!("ERROR: {}", e),
        }
    }
}

fn get_clap_command() -> clap::Command {
    command!().args([
        arg!([PATHS]... "Paths to scan (can be directories or files)")
            .default_value(".")
            .value_parser(value_parser!(PathBuf)),
        arg!(-o --output <FILE> "Output file").value_parser(value_parser!(PathBuf)),
        arg!(-l --length <LEN> "Length of hashes, -1 for default")
            .default_value("-1")
            .value_parser(value_parser!(i32).range(-1..=HASH_LENGTH as i64)),
        arg!(-'0' --"no-hash" "Don't hash files"),
        arg!(-a --all "Include hidden files"),
        // overrides with will make it so that when this is specified, the other one gets forgotten
        // basically -r AND -R will never both be true, either both false or one false one true
        arg!(-r --recursive "Hash directories recursively, default").overrides_with("no-recursive"),
        arg!(-'R' --"no-recursive" "Don't hash directories recursively")
            .overrides_with("recursive"),
        // if you want '\t' to be tab is shell, use $'\t'
        arg!(-s --separator <SEP> "Separator between hash and path, has no effect if --no-hash")
            .default_value("  "),
        arg!(-p --print "always print to stdout, even if --output is set"),
        arg!(-d --directory "Include directories when hashing recursively"),
    ])
}
