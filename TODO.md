- [ ] Make progress bar check file sizes, instead of advancing only when it finishes a single file
- [ ] Optimize it somehow, maybe use multiple threads or something like that
- [x] Display progress bar in stderr, not stdout (maybe use different progress bar library)
- [ ] TODO: separate run into a function that hashes many paths and returns Vec<String>
- [ ] TODO: canonicalize paths that get cached, so I never hash same file twice

