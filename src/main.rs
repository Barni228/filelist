use cached::proc_macro::cached;
use clap::{arg, command, value_parser};
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
    cell::RefCell,
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Read, Write},
    path::PathBuf,
    rc::Rc,
    vec,
};
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

const MAX_HASH_LENGTH: i32 = 64;
const MAX_HASH_LENGTH_STR: &str = "64";

fn main() {
    let matches = get_clap_command().get_matches();

    let no_hash = matches.get_flag("no-hash");
    // HACK: in order to make no_hash work with progress_func, I will just say that sep and hash are both empty
    // So only file will be left :)
    let hash_length = match no_hash {
        true => 0,
        false => *matches.get_one::<i32>("length").unwrap(),
    };
    debug_assert!((0..=MAX_HASH_LENGTH).contains(&hash_length));
    let sep = match no_hash {
        true => &String::new(),
        false => matches.get_one::<String>("separator").unwrap(),
    };

    let all = matches.get_flag("all");
    let always_print = matches.get_flag("print");
    let hash_directory = matches.get_flag("directory");
    let recursive = !matches.get_flag("no-recursive");
    let progress_hash = matches.get_flag("progress-hash");
    let progress_bar = matches.get_flag("progress-bar");
    let use_color = match matches.get_one::<String>("color").unwrap().as_str() {
        "always" => true,
        "never" => false,
        "auto" => io::stdout().is_terminal() && io::stderr().is_terminal(),
        _ => unreachable!(),
    };

    let output = matches.get_one::<PathBuf>("output");

    if let Some(output) = output {
        if output.exists() && !matches.get_flag("force") {
            if use_color {
                eprintln!(
                    "{}: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    "Error".red(),
                    path_to_string(output).bold()
                );
            } else {
                eprintln!(
                    "Error: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    path_to_string(output)
                );
            }
            std::process::exit(1);
        }
    }

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
        // I will add / to directories in the path_to_string function
        .collect::<Vec<_>>();

    paths.sort_unstable();
    // remove same consecutive elements, since this is sorted it will remove all duplicates
    paths.dedup();
    let pb = match progress_bar {
        true => {
            // create a progress bar
            let pb = ProgressBar::new(paths.len());
            // show it, so something like 0/69 is shown
            pb.display();
            // return thing that allows me to modify progress bar in different places
            Some(Rc::new(RefCell::new(pb)))
        }
        false => None,
    };

    let fmt_line = |hash: &str, path: &str| {
        let hash_cut = match hash.starts_with("ERROR:") {
            true => hash,
            false => &hash[0..hash_length as usize],
        };
        format!("{hash_cut}{sep}{path}\n")
    };

    let mut progress_func = |hash: &str, path: &str| {
        if let Some(pb) = pb.as_ref() {
            // increment the progress bar
            pb.borrow_mut().inc();
        }

        if progress_hash {
            if use_color {
                eprint_respect_progress(fmt_line(hash, path).yellow().dim(), &pb);
            } else {
                eprint_respect_progress(fmt_line(hash, path), &pb);
            }
        }
    };

    let lines = paths.iter().map(|p| {
        let path = path_to_string(p);
        if no_hash {
            let result = format!("{}\n", path);
            progress_func("", &path);
            result
        } else {
            // ignore the output file, because we cannot hash it since we dont know what it is yet
            let ignore = match output {
                Some(p) => p,
                None => &PathBuf::new(),
            };
            fmt_line(&hash_no_error(p, all, ignore, &mut progress_func), &path)
        }
    });

    if let Some(output) = matches.get_one::<PathBuf>("output") {
        let mut file = File::create(output).unwrap();
        for line in lines {
            file.write_all(line.as_bytes()).unwrap();
            if always_print {
                print_respect_progress(line, &pb);
            }
        }
    } else {
        for line in lines {
            print_respect_progress(line, &pb);
        }
    }

    // if you don't finalize it, it will disappear after the program finishes
    // if let Some(pb) = pb.as_ref() {
    //     pb.borrow_mut().finalize();
    // }
}

fn path_to_string(path: &PathBuf) -> String {
    if path.is_dir() {
        format!("{}/", path.display())
    } else {
        path.display().to_string()
    }
}

// if I print regularly, text will combine with the progress bar and make everything weird
// so text will be like
// abc123 file.txt====>    ] 0/69
// Note: s should end with `\n`, otherwise weird stuff will happen
fn print_to_respect_progress(
    out: &mut impl Write,
    s: impl std::fmt::Display,
    pb: &Option<Rc<RefCell<ProgressBar>>>,
) -> io::Result<()> {
    if let Some(pb) = pb {
        // clear the old progress bar, and print s
        queue!(out, Clear(ClearType::UntilNewLine), Print(s))?;
        // re-print the progress bar again
        // this will also probably flush the stdout, so queue above is fine
        pb.borrow().display();
    } else {
        // if there is no progress bar, just print regularly
        // print!("{}", s);
        write!(out, "{}", s)?;
    };
    Ok(())
}

fn print_respect_progress(s: impl std::fmt::Display, pb: &Option<Rc<RefCell<ProgressBar>>>) {
    print_to_respect_progress(&mut io::stdout(), s, pb).unwrap();
}

fn eprint_respect_progress(s: impl std::fmt::Display, pb: &Option<Rc<RefCell<ProgressBar>>>) {
    print_to_respect_progress(&mut io::stderr(), s, pb).unwrap();
}

fn _hash_file(path: &PathBuf) -> io::Result<String> {
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

fn _hash_dir(
    path: &PathBuf,
    use_hidden: bool,
    ignore_file: &PathBuf,
    progress_func: &mut impl FnMut(&str, &str),
) -> io::Result<String> {
    let mut hashes = vec![];
    for entry in fs::read_dir(path)?.filter_map(Result::ok) {
        if !use_hidden && entry.is_hidden() {
            continue;
        }
        let path = entry.path().clean();
        if path == ignore_file.as_path() {
            continue;
        }

        let hash = hash_no_error(&path, use_hidden, ignore_file, progress_func);
        if !hash.starts_with("ERROR:") {
            hashes.push(hash);
        }
    }

    hashes.sort_unstable();

    let hash = hex::encode(Sha256::digest(hashes.join("").as_bytes()));
    Ok(hash)
}

// cache ignores progress_func, and just caches by (path, use_hidden)
#[cached(key = "(PathBuf, bool)", convert = r#"{ (path.clone(), use_hidden) }"#)]
fn hash_no_error(
    path: &PathBuf,
    use_hidden: bool,
    ignore_file: &PathBuf,
    progress_func: &mut impl FnMut(&str, &str),
) -> String {
    let hash = if path.is_dir() {
        match _hash_dir(path, use_hidden, ignore_file, progress_func) {
            Ok(s) => s,
            Err(e) => format!("ERROR: {}", e),
        }
    } else {
        match _hash_file(path) {
            Ok(s) => s,
            Err(e) => format!("ERROR: {}", e),
        }
    };

    progress_func(&hash, &path_to_string(path));

    hash
}

fn get_clap_command() -> clap::Command {
    command!().args([
        arg!([PATHS]... "Paths to scan (can be directories or files)")
            .default_value(".")
            .value_parser(value_parser!(PathBuf)),
        arg!(-o --output <FILE> "Output file").value_parser(value_parser!(PathBuf)),
        arg!(-l --length <LEN> "Length of hashes")
            .default_value(MAX_HASH_LENGTH_STR)
            .value_parser(value_parser!(i32).range(0..=MAX_HASH_LENGTH as i64)),
        arg!(-'0' --"no-hash" "Don't hash files"),
        arg!(-a --all "Include hidden files"),
        // overrides with will make it so that when this is specified, the other one gets forgotten
        // basically -r AND -R will never both be true, either both false or one false one true
        arg!(-r --recursive "Hash directories recursively, default").overrides_with("no-recursive"),
        arg!(-R --"no-recursive" "Don't hash directories recursively").overrides_with("recursive"),
        // if you want '\t' to be tab is shell, use $'\t'
        arg!(-s --separator <SEP> "Separator between hash and path, has no effect if --no-hash")
            .default_value("  "),
        arg!(-P --print "always print to stdout, even if --output is set"),
        arg!(-d --directory "Include directories when hashing recursively"),
        arg!(-e --"progress-hash" "print what has been hashed so far to stderr"),
        arg!(-p --"progress-bar" "print progress bar to stderr"),
        arg!(-f --force "Overwrite output file if it exists"),
        arg!(--color <WHEN> "When to use colors (*auto*, never, always).")
            .default_value("auto")
            .value_parser(["auto", "always", "never"]),
    ])
}
