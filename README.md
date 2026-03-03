# filelist

A configurable file and directory hashing utility.

`FileList` can:

- Hash files using SHA-256
- Hash directories deterministically (by hashing sorted child hashes)
- Recurse into directories
- Skip hidden files (unless enabled)
- Show a progress bar
- Output to stdout or a file

## How to use it

Here is how this works, and how to use it:

if you type `filelist` without any arguments, it will default to `.`  
if you pass a file, it will hash that file  
if you pass a directory to `filelist`, by default it will hash everything inside of that directory recursively,
however if you use `-R` (`--no-recursive`) it will not go into directories, and instead it will hash the directories provided (as if you used `-d`)
you can pass multiple paths to this, and it will just hash all of them

### Files

When hashing files, it just hashes the file (same as `sha256sum` or `sha256` command)  
`-l` or `--length` will trim the hash to match the gives length
so `abcdef123456` with length 4 will become `abcd`

### Directories

A directory is hashed like this:

1.  Hash each file and dir in the directory
2.  Ignore every error
3.  Sort those hashes
4.  Hash all of those hashes together

This makes directory hashes stable and order-independent.

it will hash everything in parallel (unless `--no-parallel` is set)  
This means that `--length` will not change the directory hash, only truncate it,
file names have no effect on the directory hash, only file content
The directory hash WILL change depending on if `--all` is set or no, because if `--all` is enabled, hidden files
in directory will get hashed, while if `--all` is not enabled, then hidden files will be ignored
This is by design, because usually you don't consider a directory different if it has some weird file like `.DS_Store` added to it by your folder, so if you want to include hidden files make sure to use `--all`

### Symlinks

By default, this will hash the target path of the symlink (something like "../README.md")
If you want it to hash the thing symlink points to (file or dir), use `--link`
with `--link`, this treat every symlink as the thing it points to (so same as file or dir)

## Example

```
.
├── .hidden
├── dir
│   └── regular
└── regular
```

```sh
filelist -pl16 ./dir
```

outputs

```
dd57c65a5219917d  dir/regular
```

```sh
filelist -l=32 -d -a
```

```
72676a6eb3c35529a7c450d195045d66  ./
e3b0c44298fc1c149afbf4c8996fb924  .hidden
11f9c53c2abc7d5a9f442687280f80bd  dir/
dd57c65a5219917d4c423ce6a0bf2d95  dir/regular
ERROR: Permission denied (os error 13)  no_read
7f44ae7d5074b592265a407f5495aa12  regular
```
