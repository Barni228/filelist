# filelist

A configurable file and directory hashing utility.

`FileList` can:

- Hash files using SHA-256
- Hash directories deterministically (by hashing sorted child hashes)
- Recurse into directories
- Skip hidden files (unless enabled)
- Show a progress bar
- Output to stdout or a file

## Directory Hashing

A directory hash is computed by:

1.  Hashing each entry inside the directory
2.  Sorting those hashes
3.  Hashing the concatenated result

This makes directory hashes stable and order-independent.

## Example

```rust
use std::path::PathBuf;
use filelist::FileList;

fn main() -> std::io::Result<()> {
    let mut fl = FileList::new();
    fl.set_recursive(true)
      .set_hash_length(16)
      .set_use_progress_bar(true);

    fl.run(vec![PathBuf::from("./README.md")])?;
    Ok(())
}
```
