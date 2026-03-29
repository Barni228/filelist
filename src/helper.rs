use std::fs;

/// Replace `[$left | $right]` with `$left` or `$right` depending on `$when`.
/// If `$when` is true, `$left` is used, otherwise `$right` is used.
/// This is NOT recursive, so in only replaces in one level of tokens
/// this means that `[par_iter | iter]` will work, but `{ [par_iter | iter] }` will not
macro_rules! replace_when {
    ($when:expr, $($tokens:tt)*) => {
        if $when {
            replace_when!(@replace_left [] $($tokens)*)
        } else {
            replace_when!(@replace_right [] $($tokens)*)
        }
    };

    (@replace_left [ $($current:tt)* ]) => {
        $($current)*
    };
    (@replace_left [ $($current:tt)* ] [$left:ident | $right:ident] $($rest:tt)*) => {
        replace_when!(@replace_left [$($current)* $left] $($rest)*)
    };
    (@replace_left [ $($current:tt)* ] $head:tt $($rest:tt)*) => {
        replace_when!(@replace_left [$($current)* $head] $($rest)*)
    };

    (@replace_right [ $($current:tt)* ]) => {
        $($current)*
    };
    (@replace_right [ $($current:tt)* ] [$left:ident | $right:ident] $($rest:tt)*) => {
        replace_when!(@replace_right [$($current)* $right] $($rest)*)
    };
    (@replace_right [ $($current:tt)* ] $head:tt $($rest:tt)*) => {
        replace_when!(@replace_right [$($current)* $head] $($rest)*)
    };
}

pub(crate) use replace_when;

pub(crate) trait IsHidden {
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
