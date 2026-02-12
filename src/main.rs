use clap::{arg, command, value_parser};
use crossterm::style::Stylize;
use filelist::FileList;
use std::{
    io::{self, IsTerminal},
    path::PathBuf,
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
    let force = matches.get_flag("force");

    fl.set_output(matches.get_one::<PathBuf>("output"))
        .set_hash_length(*matches.get_one::<i32>("length").unwrap() as usize)
        .set_no_hash(matches.get_flag("no-hash"))
        .set_all(matches.get_flag("all"))
        .set_recursive(!matches.get_flag("no-recursive"))
        .set_sep(matches.get_one::<String>("separator").unwrap())
        .set_always_print(matches.get_flag("print"))
        .set_hash_directory(matches.get_flag("directory"))
        .set_use_progress_hash(matches.get_flag("progress-hash"))
        .set_use_progress_bar(matches.get_flag("progress-bar"))
        .set_use_color(use_color);

    if let Some(output) = fl.output() {
        if output.exists() && !force {
            if use_color {
                eprintln!(
                    "{}: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    "Error".red(),
                    fl.path_to_string(output).bold()
                );
            } else {
                eprintln!(
                    "Error: output file \"{}\" already exists.\n\
                    If you want to overwrite it, use the -f / --force flag.",
                    fl.path_to_string(output)
                );
            }
            std::process::exit(1);
        }
    }
    let paths: Vec<PathBuf> = matches
        .get_many::<PathBuf>("PATHS")
        .unwrap()
        .map(|p| p.clone())
        .collect();

    fl.run(paths).unwrap();
}

fn get_clap_command() -> clap::Command {
    command!().args([
        arg!([PATHS]... "Paths to scan (can be directories or files)")
            .default_value(".")
            .value_parser(value_parser!(PathBuf)),
        arg!(-o --output <FILE> "Output file").value_parser(value_parser!(PathBuf)),
        arg!(-l --length <LEN> "Length of hashes")
            .default_value("64")
            .value_parser(value_parser!(i32).range(0..=64 as i64)),
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
