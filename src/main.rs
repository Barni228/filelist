use clap::{arg, command, value_parser};
use filelist::FileList;
use std::{
    io::{self, IsTerminal},
    path::{Path, PathBuf},
};

fn main() {
    let matches = get_clap_command().get_matches();
    let mut fl = FileList::new();

    let use_color = match matches.get_one::<String>("color").unwrap().as_str() {
        "always" => true,
        "never" => false,
        "auto" => io::stdout().is_terminal() && io::stderr().is_terminal(),
        _ => unreachable!(),
    };
    let progress_bar_type = match matches
        .get_one::<String>("progress-bar-type")
        .unwrap()
        .as_str()
    {
        "auto" => filelist::ProgressBarType::Auto,
        "files" => filelist::ProgressBarType::Files,
        "bytes" => filelist::ProgressBarType::Bytes,
        _ => unreachable!(),
    };

    if let Some(relative_to) = matches.get_one::<PathBuf>("relative-to") {
        fl.set_relative_to(relative_to);
    }

    fl.set_output(matches.get_one::<PathBuf>("output").map(|p| p.as_path()))
        .set_hash_length(*matches.get_one::<i32>("length").unwrap() as usize)
        .set_absolute(matches.get_flag("absolute"))
        .set_use_dot_prefix(matches.get_flag("keep-dot"))
        .set_sep(matches.get_one::<String>("sep").unwrap().clone())
        .set_use_progress_hash(matches.get_flag("progress-hash"))
        .set_use_progress_bar(!matches.get_flag("no-progress-bar"))
        .set_progress_bar_type(progress_bar_type)
        .set_use_color(use_color)
        .set_force(matches.get_flag("force"))
        .hasher_mut()
        .set_no_hash(matches.get_flag("no-hash"))
        .set_all(matches.get_flag("all"))
        .set_recursive(!matches.get_flag("no-recursive"))
        .set_follow_links(matches.get_flag("link"))
        .set_hash_directory(matches.get_flag("directory"))
        .set_use_parallel(!matches.get_flag("no-parallel"));

    let mut paths: Vec<&Path> = matches
        .get_many::<PathBuf>("PATHS")
        .map(|p| p.map(|p| p.as_path()).collect())
        .unwrap_or_else(|| {
            // if something is piped to stdin, hash stdin
            if !io::stdin().is_terminal() {
                vec![Path::new("-")]
            } else {
                // just hash the current dir
                vec![Path::new(".")]
            }
        });

    if let Some(stdin_index) = paths.iter().position(|&p| p == Path::new("-")) {
        // include stdin in the output, with path "-"
        fl.set_include_stdin(Some("-".to_string()));
        paths.remove(stdin_index);
    }

    fl.run(&paths).unwrap();
}

fn get_clap_command() -> clap::Command {
    command!().args([
        arg!([PATHS]... "Paths to scan (can be directories or files)")
            .value_parser(value_parser!(PathBuf)),
        arg!(-o --output <FILE> "Output file").value_parser(value_parser!(PathBuf)),
        arg!(-f --force "Overwrite output file if it exists"),
        arg!(-l --length <LEN> "Length of hashes")
            .default_value("64")
            .value_parser(value_parser!(i32).range(0..=64_i64)),
        arg!(-a --all "Include hidden files"),
        arg!(-d --directory "Include directory entries in output"),
        arg!(-'0' --"no-hash" "List files without computing hashes"),
        arg!(-A --absolute "Convert all paths to absolute paths"),
        arg!(-s --link "Follow symlinks"),
        // overrides with will make it so that when this is specified, the other one gets forgotten
        // basically -r AND -R will never both be true, either both false or one false one true
        arg!(-r --recursive "Hash directories recursively, default").overrides_with("no-recursive"),
        arg!(-R --"no-recursive" "Don't hash directories recursively").overrides_with("recursive"),
        arg!(--"relative-to" <PATH> "Make all paths relative to PATH")
            .value_parser(value_parser!(PathBuf))
            .conflicts_with("absolute"),
        arg!(-k --"keep-dot" "Use '.' as prefix for relative paths (`./hi` instead of `hi`)")
            .alias("dot-prefix")
            .conflicts_with("absolute"),
        arg!(-e --"progress-hash" "print what has been hashed so far to stderr"),
        arg!(-p --"progress-bar" "print progress bar to stderr").overrides_with("no-progress-bar"),
        arg!(-P --"no-progress-bar" "Dont print progress bar to stderr")
            .overrides_with("progress-bar"),
        arg!(--"progress-bar-type" <TYPE> "Type of progress bar to use")
            .default_value("auto")
            .value_parser(["auto", "files", "bytes"]),
        // if you want '\t' to be tab in shell, use $'\t'
        arg!(--sep <SEP> "Separator between hash and path, has no effect if --no-hash")
            .alias("separator")
            .default_value("  "),
        arg!(--color <WHEN> "When to use colors (*auto*, never, always).")
            .default_value("auto")
            .value_parser(["auto", "always", "never"]),
        arg!(--"parallel" "Enable parallel hashing (default)").overrides_with("no-parallel"),
        arg!(--"no-parallel" "Disable parallel hashing, use a single thread")
            .overrides_with("parallel"),
    ])
}
