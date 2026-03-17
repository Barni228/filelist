use super::*;

#[test]
fn test_get_output_paths_files() {
    let fl = FileList::new();
    let real_paths = fl.get_output_paths(&["test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("test_files/dir/regular"),
            PathBuf::from("test_files/no_read"),
            PathBuf::from("test_files/regular"),
        ],
        real_paths
    );
}

#[test]
fn test_get_output_paths_dir() {
    let mut fl = FileList::new();
    fl.set_hash_directory(true);

    let real_paths = fl.get_output_paths(&["test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("test_files"),
            PathBuf::from("test_files/dir"),
            PathBuf::from("test_files/dir/regular"),
            PathBuf::from("test_files/no_read"),
            PathBuf::from("test_files/regular"),
        ],
        real_paths
    );
}

#[test]
fn test_get_output_paths_hidden() {
    let mut fl = FileList::new();
    fl.set_hash_directory(true);
    fl.set_all(true);

    let real_paths = fl.get_output_paths(&["test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("test_files"),
            PathBuf::from("test_files/.hidden"),
            PathBuf::from("test_files/dir"),
            PathBuf::from("test_files/dir/regular"),
            PathBuf::from("test_files/no_read"),
            PathBuf::from("test_files/regular"),
        ],
        real_paths
    );
}

#[test]
fn test_get_output_paths_link() {
    let mut fl = FileList::new();

    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );

    fl.set_hash_directory(true);
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files"),
            PathBuf::from("symlink_test_files/dir"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );
}

#[test]
fn test_get_output_paths_link_follow() {
    let mut fl = FileList::new();
    fl.set_follow_links(true);

    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link/inside"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );

    fl.set_hash_directory(true);
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files"),
            PathBuf::from("symlink_test_files/dir"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/dir-link/inside"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );
}

#[test]
fn test_get_output_paths_does_not_exist() {
    let fl = FileList::new();
    let real_paths = fl.get_output_paths(&["test_files/no_exist".into()]);
    assert_eq!(vec![PathBuf::from("test_files/no_exist"),], real_paths);
}

#[test]
fn test_get_hash_dependencies_files() {
    let fl = FileList::new();
    let real_paths = fl.get_output_paths(&["test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([
            PathBuf::from("test_files/dir/regular"),
            PathBuf::from("test_files/no_read"),
            PathBuf::from("test_files/regular"),
        ])],
        dependencies
    );
}

#[test]
fn test_get_hash_dependencies_dir() {
    let mut fl = FileList::new();
    fl.set_hash_directory(true);
    let real_paths = fl.get_output_paths(&["test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![
            HashSet::from([
                PathBuf::from("test_files/dir/regular"),
                PathBuf::from("test_files/no_read"),
                PathBuf::from("test_files/regular"),
            ]),
            HashSet::from([PathBuf::from("test_files/dir")]),
            HashSet::from([PathBuf::from("test_files")]),
        ],
        dependencies
    );
}

#[test]
fn test_get_hash_dependencies_hidden() {
    let mut fl = FileList::new();
    fl.set_hash_directory(true);
    fl.set_all(true);
    let real_paths = fl.get_output_paths(&["test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![
            HashSet::from([
                PathBuf::from("test_files/.hidden"),
                PathBuf::from("test_files/dir/regular"),
                PathBuf::from("test_files/no_read"),
                PathBuf::from("test_files/regular"),
            ]),
            HashSet::from([PathBuf::from("test_files/dir")]),
            HashSet::from([PathBuf::from("test_files")]),
        ],
        dependencies
    );
}

#[test]
fn test_get_hash_dependencies_link() {
    let mut fl = FileList::new();
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/link"),
        ])],
        dependencies
    );

    fl.set_hash_directory(true);
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![
            HashSet::from([
                PathBuf::from("symlink_test_files/dir-link"),
                PathBuf::from("symlink_test_files/dir/inside"),
                PathBuf::from("symlink_test_files/link"),
            ]),
            HashSet::from([PathBuf::from("symlink_test_files/dir")]),
            HashSet::from([PathBuf::from("symlink_test_files")]),
        ],
        dependencies
    );
}

#[test]
fn test_get_hash_dependencies_link_follow() {
    let mut fl = FileList::new();
    fl.set_follow_links(true);
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([
            PathBuf::from("symlink_test_files/dir-link/inside"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/link"),
        ])],
        dependencies
    );

    fl.set_hash_directory(true);
    let real_paths = fl.get_output_paths(&["symlink_test_files".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![
            HashSet::from([
                PathBuf::from("symlink_test_files/dir-link/inside"),
                PathBuf::from("symlink_test_files/dir/inside"),
                PathBuf::from("symlink_test_files/link"),
            ]),
            HashSet::from([
                PathBuf::from("symlink_test_files/dir"),
                PathBuf::from("symlink_test_files/dir-link")
            ]),
            HashSet::from([PathBuf::from("symlink_test_files")]),
        ],
        dependencies
    );
}

#[test]
fn test_get_hash_dependencies_does_not_exist() {
    let fl = FileList::new();
    let real_paths = fl.get_output_paths(&["test_files/no_exist".into()]);
    let dependencies = fl.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([PathBuf::from("test_files/no_exist"),])],
        dependencies
    );
}
