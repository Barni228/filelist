- [ ] When hashing followed symlinks, instead of hashing their target again get it from cache
- [ ] Add option to print progress bar info in stderr, that other programs could parse
- [ ] Add option to respect gitignore / special ignore file
- [ ] maybe `FileList::run` should be something that main.rs handles,
      and `FileList` should just be library that handle library stuff, like having nice API, not writing files

## Easy

## Bugs

- [ ] `filelist ~/Library` will be very slow, add a `break` to hash_file hashing loop and try to hash Library (with `-dp` flags preferably)
      so that it does not take ages to finish

## Done

- [x] Add an option to have paths like "./regular" instead of always cleaning the "." away
- [x] Add function that returns `BTreeMap` that maps Relative paths (`PathBuf` or `RelativePathBuf`) to formatted hash (`String`)
- [x] Maybe make a nicer detection of stdin (right now i check if path == "-")
- [x] `filelist | less` if you press `q` before it prints everything, it throws an error
- [x] Canonicalize paths that get cached, so I never hash same file twice
- [x] Make progress bar check file sizes, instead of advancing only when it finishes a single file
- [x] Decide EXACTLY what -R flag does (`filelist -R .`, what should this do? Nothing?)
- [x] Check if it works with `..` paths (parent paths)
- [x] `get_hash_dependencies` will ignore non existing files,
      because `WalkDir` returns `Err` for those and I filter all Errors out
      I think it should still return them, because I still hash them ("ERROR: ..." is the hash)
- [x] Maybe make `-p` the default
- [x] don't clean every path, instead only use PathClean where needed (user input)
      I think I dont actually need to clean any paths at all, since if user gives a weird "dirty" path
      As long as its valid path, everyone in the program will just use that dirty path so it works
      (if u give ./././this, WalkDir will just use ./././this prefix for everything it returns)
- [x] Make a `-` argument mean stdin
- [x] Make `printf hi | filelist` hash `hi`
- [x] Support /dev/fd/... files (so `filelist <(echo "hi")` works)
- [x] Test if it works correctly with symlinks (dont follow symlinks by default, make it optional, like -s)
- [x] Optimize it somehow, maybe use multiple threads or something like that
- [x] Display progress bar in stderr, not stdout (maybe use different progress bar library)
- [x] Separate run into a function that hashes many paths and returns `Vec<String>`
- [x] In README, write exactly how this works (what affects the hash, what doesn't)
      because right now its not clear at all (like -a changes the hash of directories, but its not documented, or do error files like no_read affect the hash)
