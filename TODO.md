- [ ] Make progress bar check file sizes, instead of advancing only when it finishes a single file
- [ ] Canonicalize paths that get cached, so I never hash same file twice
- [ ] Check if it works with `..` paths (parent paths)
- [x] Make a `-` argument mean stdin
- [ ] Make `printf hi | filelist` hash `hi`
- [ ] `get_hash_dependencies` will ignore non existing files,
      because `WalkDir` returns `Err` for those and I filter all Errors out
      I think it should still return them, because I still hash them ("ERROR: ..." is the hash)

## Easy

- [ ] don't clean every path, instead only use PathClean where needed (user input)
      I think I dont actually need to clean any paths at all, since if user gives a weird "dirty" path
      As long as its valid path, everyone in the program will just use that dirty path so it works
      (if u give ./././this, WalkDir will just use ./././this prefix for everything it returns)
- [ ] Decide EXACTLY what -R flag does (`filelist -R .`, what should this do? Nothing?)
- [ ] Maybe make `-p` the default

## Bugs

- [ ] `filelist ~/Library` will be very slow, add a `break` to hash_file hashing loop and try to hash Library (with `-dp` flags preferably)
      so that it does not take ages to finish

## Done

- [x] Support /dev/fd/... files (so `filelist <(echo "hi")` works)
- [x] Test if it works correctly with symlinks (dont follow symlinks by default, make it optional, like -s)
- [x] Optimize it somehow, maybe use multiple threads or something like that
- [x] Display progress bar in stderr, not stdout (maybe use different progress bar library)
- [x] Separate run into a function that hashes many paths and returns `Vec<String>`
- [x] In README, write exactly how this works (what affects the hash, what doesn't)
      because right now its not clear at all (like -a changes the hash of directories, but its not documented, or do error files like no_read affect the hash)
