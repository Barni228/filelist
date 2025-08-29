use assert_cmd::Command;

fn run(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> String {
    let output = Command::cargo_bin("filelist")
        .unwrap()
        .args(args)
        .output()
        .unwrap();

    assert!(output.status.success());

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn test_simple_cli() {
    assert_eq!(
        run(["test_files", "-r"]),
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_no_args() {
    let out = Command::cargo_bin("filelist")
        .unwrap()
        .current_dir("test_files")
        .output()
        .unwrap();

    assert!(out.status.success());

    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  dir/regular\n",
            "ERROR: Permission denied (os error 13)  no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  regular\n",
        )
    );
}

#[test]
fn test_length_0() {
    assert_eq!(
        run(["test_files", "-rl", "0"]),
        concat!(
            "  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "  test_files/regular\n",
        )
    );
}

#[test]
fn test_length_too_big() {
    Command::cargo_bin("filelist")
        .unwrap()
        .arg("-l65")
        .assert()
        .failure();
}

#[test]
fn test_write_file() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();
    assert!(run(["test_files", "-r", "-o", path]).is_empty());
    let out = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        out,
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_default_len() {
    // if you don't use =, then -1 will be treated like option -1 and not negative 1
    for i in ["-l=-1", "--length=-1"] {
        assert_eq!(run(["test_files", i]), run(["test_files"]),);
    }
}

#[test]
fn test_no_hash() {
    for i in ["-0", "--no-hash"] {
        assert_eq!(
            run(["test_files", "-r", i]),
            concat!(
                "test_files/dir/regular\n",
                "test_files/no_read\n",
                "test_files/regular\n",
            )
        );
    }
}

#[test]
fn test_all() {
    for i in ["-a", "--all"] {
        assert_eq!(
            run(["test_files", "-r", i]),
            concat!(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  test_files/.hidden\n",
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13)  test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            )
        );
    }
}

#[test]
fn test_separator() {
    for i in ["-s", "--separator"] {
        assert_eq!(
            run(["test_files", "-r", i, " \t "]),
            concat!(
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f \t test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13) \t test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95 \t test_files/regular\n",
            )
        );
    }
}

#[test]
fn test_print() {
    let expected = concat!(
        "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
        "ERROR: Permission denied (os error 13)  test_files/no_read\n",
        "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
    );

    for i in ["-p", "--print"] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        assert_eq!(run(["test_files", "-r", "-o", path, i]), expected);
        // you can give it a file or path, both work
        let out = std::fs::read_to_string(file).unwrap();
        assert_eq!(out, expected);
    }
}

#[test]
fn test_multiple_files() {
    assert_eq!(
        run(["test_files/no_read", "test_files/regular"]),
        concat!(
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_pass_hidden() {
    assert_eq!(
        run(["test_files/regular", "test_files/.hidden"]),
        concat!(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  test_files/.hidden\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_same_files() {
    assert_eq!(
        run(["test_files/regular", "test_files/regular"]),
        concat!(
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_files_and_dirs() {
    assert_eq!(
        run(["-r", "test_files/regular", "test_files"]),
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        )
    );
}

#[test]
fn test_no_recursive() {
    for i in ["-R", "--no-recursive"] {
        assert_eq!(
            run(["test_files", i]),
            "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files\n"
        );
    }
}

#[test]
fn test_many_recursive() {
    assert_eq!(
        run(["-rRrRrR", "test_files"]),
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files\n"
    )
}

#[test]
fn test_directory() {
    for i in ["-d", "--directory"] {
        assert_eq!(
            run(["test_files", "-r", i]),
            concat!(
                "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files\n",
                "11f9c53c2abc7d5a9f442687280f80bd5419feaf55af2e598e26d9b285d63ffd  test_files/dir\n",
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13)  test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            )
        );
    }
}

#[test]
fn test_hash_directory_all() {
    // hash of directory changes based on whether or not all is set
    assert_eq!(
        run(["-R", "test_files"]),
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files\n"
    );
    assert_eq!(
        run(["-Ra", "test_files"]),
        "72676a6eb3c35529a7c450d195045d660137a77d47cd9b980e508a76ce396def  test_files\n"
    );
}

#[test]
fn test_everything() {
    let expected = concat!(
        "test_files\n",
        "test_files/.hidden\n",
        "test_files/dir\n",
        "test_files/dir/regular\n",
        "test_files/no_read\n",
        "test_files/regular\n",
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    assert_eq!(
        run(["test_files", "-0arpdl12", "-o", path, "--separator", "sep"]),
        expected
    );
    // you can give it a file or path, both work
    let out = std::fs::read_to_string(file).unwrap();
    assert_eq!(out, expected);
}
