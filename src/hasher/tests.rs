use super::*;

#[test]
fn test_get_output_paths_files() {
    let mut hasher = Hasher::new();
    hasher.set_paths(vec!["test_files".into()]);
    let real_paths = hasher.get_output_paths();
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
    let mut hasher = Hasher::new().with_hash_directory(true);
    hasher.set_paths(vec!["test_files".into()]);
    let real_paths = hasher.get_output_paths();
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
    let mut hasher = Hasher::new().with_hash_directory(true).with_all(true);

    hasher.set_paths(vec!["test_files".into()]);
    let real_paths = hasher.get_output_paths();
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
    let mut hasher = Hasher::new();

    hasher.set_paths(vec!["symlink_test_files".into()]);
    let real_paths = hasher.get_output_paths();
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );

    hasher.set_hash_directory(true);
    let real_paths = hasher.get_output_paths();
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
    let mut hasher = Hasher::new().with_follow_links(true);

    hasher.set_paths(vec!["symlink_test_files".into()]);
    let real_paths = hasher.get_output_paths();
    assert_eq!(
        vec![
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/dir-link/inside"),
            PathBuf::from("symlink_test_files/link"),
        ],
        real_paths
    );

    hasher.set_hash_directory(true);
    let real_paths = hasher.get_output_paths();
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
    let hasher = Hasher::new().with_paths(vec!["test_files/no_exist".into()]);
    let real_paths = hasher.get_output_paths();
    assert_eq!(vec![PathBuf::from("test_files/no_exist"),], real_paths);
}

#[test]
fn test_get_hash_dependencies_files() {
    let hasher = Hasher::new().with_paths(vec!["test_files".into()]);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
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
    let hasher = Hasher::new()
        .with_paths(vec!["test_files".into()])
        .with_hash_directory(true);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
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
    let hasher = Hasher::new()
        .with_hash_directory(true)
        .with_all(true)
        .with_paths(vec!["test_files".into()]);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
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
    let mut hasher = Hasher::new();
    hasher.set_paths(vec!["symlink_test_files".into()]);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([
            PathBuf::from("symlink_test_files/dir-link"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/link"),
        ])],
        dependencies
    );

    hasher.set_hash_directory(true);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
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
    let mut hasher = Hasher::new();
    hasher.set_paths(vec!["symlink_test_files".into()]);
    hasher.set_follow_links(true);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([
            PathBuf::from("symlink_test_files/dir-link/inside"),
            PathBuf::from("symlink_test_files/dir/inside"),
            PathBuf::from("symlink_test_files/link"),
        ])],
        dependencies
    );

    hasher.set_hash_directory(true);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
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
    let hasher = Hasher::new().with_paths(vec!["test_files/no_exist".into()]);
    let real_paths = hasher.get_output_paths();
    let dependencies = hasher.get_hash_dependencies(&real_paths);
    assert_eq!(
        vec![HashSet::from([PathBuf::from("test_files/no_exist"),])],
        dependencies
    );
}

#[test]
fn test_hash_multiple_times() {
    let mut hasher = Hasher::new();
    hasher.set_no_hash(true);
    hasher.set_paths(vec!["test_files".into()]);
    assert_eq!(
        BTreeMap::from([
            ("test_files/dir/regular".into(), "".into()),
            ("test_files/no_read".into(), "".into()),
            ("test_files/regular".into(), "".into())
        ]),
        hasher.start()
    );

    hasher.set_no_hash(false);
    assert_eq!(
        BTreeMap::from([
            (
                "test_files/dir/regular".into(),
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f".into()
            ),
            (
                "test_files/no_read".into(),
                "ERROR: Permission denied (os error 13)".into()
            ),
            (
                "test_files/regular".into(),
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95".into()
            )
        ]),
        hasher.start()
    );
}
