use assert_cmd::Command;
use itertools::Itertools;

// NOTE: if you clone this repo, make sure to create test_files/no_read file
// touch test_files/no_read
// chmod 000 test_files/no_read

fn run(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> String {
    let output = Command::cargo_bin("filelist")
        .unwrap()
        .args(args)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

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
    assert!(run(["test_files", "-r", "-fo", path]).is_empty());
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
fn test_output_file_exists() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();
    Command::cargo_bin("filelist")
        .unwrap()
        .args(["-r", "-o", path])
        .assert()
        .failure();
    let out = std::fs::read_to_string(path).unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_default_len() {
    for i in ["-l=64", "--length=64"] {
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

    for i in ["-P", "--print"] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        assert_eq!(run(["test_files", "-r", "-fo", path, i]), expected);
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
            "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n"
        );
    }
}

#[test]
fn test_many_recursive() {
    assert_eq!(
        run(["-rRrRrR", "test_files"]),
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n"
    )
}

#[test]
fn test_directory() {
    for i in ["-d", "--directory"] {
        assert_eq!(
            run(["test_files", "-r", i]),
            concat!(
                "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n",
                "11f9c53c2abc7d5a9f442687280f80bd5419feaf55af2e598e26d9b285d63ffd  test_files/dir/\n",
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
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n"
    );
    assert_eq!(
        run(["-Ra", "test_files"]),
        "72676a6eb3c35529a7c450d195045d660137a77d47cd9b980e508a76ce396def  test_files/\n"
    );
}

#[test]
fn test_progress_hash() {
    let same_unordered = vec!["-0", "-a", "-l12", "-s=___", "-d", "--parallel"];
    // powerset will give us all possible combinations, like
    for i in same_unordered.iter().powerset() {
        let output = Command::cargo_bin("filelist")
            .unwrap()
            .args(["-e", "test_files"].iter().chain(i))
            .output()
            .unwrap();
        let s_out = String::from_utf8(output.stdout).unwrap();
        let s_err = String::from_utf8(output.stderr).unwrap();
        let out = s_out.split('\n').sorted_unstable();
        let err = s_err.split('\n').sorted_unstable();
        assert_eq!(out.collect_vec(), err.collect_vec());
    }
}

#[test]
fn test_progress_hash_file() {
    let same_output = vec!["-0", "-a", "-l12", "-s=___", "-d"];

    for i in same_output.iter().powerset() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        let output = Command::cargo_bin("filelist")
            .unwrap()
            .args(["-e", "test_files", "-fo", path].iter().chain(i))
            .output()
            .unwrap();

        let s_out = String::from_utf8(output.stdout).unwrap();
        let s_err = String::from_utf8(output.stderr).unwrap();
        let err = s_err.split('\n').sorted_unstable().collect_vec();
        let s_file_content = std::fs::read_to_string(path).unwrap();
        let file_content = s_file_content.split('\n').sorted_unstable().collect_vec();

        assert!(s_out.is_empty());

        assert_eq!(err, file_content);
    }
}

#[test]
fn test_progress_hash_no_recursion() {
    let same_output = vec!["-a", "-l12", "-s=___"];

    for i in same_output.iter().powerset() {
        let output = Command::cargo_bin("filelist")
            .unwrap()
            .args(["-feR", "test_files"].iter().chain(i.clone()))
            .output()
            .unwrap();

        assert!(output.status.success());

        let s_err = String::from_utf8(output.stderr).unwrap();
        let err = s_err.split('\n').sorted_unstable();
        let real_output = run(["-d", "test_files"].iter().chain(i));
        assert_eq!(
            err.collect_vec(),
            real_output.split('\n').sorted_unstable().collect_vec()
        );
    }
}

#[test]
fn test_progress_bar() {
    // if this fails, then maybe you changed progress bar logic
    let output = Command::cargo_bin("filelist")
        .unwrap()
        .args(["-p", "test_files"])
        .output()
        .unwrap();
    let s_out = String::from_utf8(output.stdout).unwrap();
    let expected_out = concat![
        "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
        "ERROR: Permission denied (os error 13)  test_files/no_read\n",
        "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
    ];
    assert_eq!(s_out, expected_out);

    let s_err = String::from_utf8(output.stderr).unwrap();
    let expected_err = concat!["",];
    assert_eq!(s_err, expected_err);
}

#[test]
fn test_color_auto() {
    let output_auto = Command::cargo_bin("filelist")
        .unwrap()
        .args(["test_files", "-e", "--color=auto"])
        .output()
        .unwrap();

    let s_auto_out = String::from_utf8(output_auto.stdout).unwrap();
    let s_auto_err = String::from_utf8(output_auto.stderr).unwrap();
    let auto_out = s_auto_out.split('\n').collect_vec();
    let auto_err = s_auto_err.split('\n').sorted_unstable().collect_vec();

    let output_never = Command::cargo_bin("filelist")
        .unwrap()
        .args(["test_files", "-e", "--color=never"])
        .output()
        .unwrap();

    let s_never_out = String::from_utf8(output_never.stdout).unwrap();
    let s_never_err = String::from_utf8(output_never.stderr).unwrap();
    let never_out = s_never_out.split('\n').collect_vec();
    let never_err = s_never_err.split('\n').sorted_unstable().collect_vec();

    assert_eq!(auto_out, never_out);
    assert_eq!(auto_err, never_err);
}

#[test]
fn test_color_always() {
    // if this test fails, then maybe you just changed the style of -e output
    let output = Command::cargo_bin("filelist")
        .unwrap()
        .args(["test_files", "-e", "--color=always"])
        .output()
        .unwrap();

    assert!(output.status.success());

    // let s_err = String::from_utf8(output.stderr).unwrap();
    let s_err = String::from_utf8(output.stderr.clone()).unwrap();
    let err = s_err.split('\n').sorted_unstable().collect_vec();

    // you can write to vector as if its stdout
    // since stdout is technically a Vec<u8>
    let mut expected = Vec::new();
    let mut expected_lines = [
        "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
        "ERROR: Permission denied (os error 13)  test_files/no_read\n",
        "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
    ];

    // order the lines in the same way filelist printed them
    expected_lines.sort_unstable_by_key(|l| {
        std::cmp::Reverse(
            err.iter()
                .position(|e| e.contains(&l.replace('\n', "")))
                .unwrap(),
        )
    });

    for line in expected_lines {
        let mut attributes = crossterm::style::Attributes::none();
        attributes.set(crossterm::style::Attribute::Dim);

        let style = crossterm::style::ContentStyle {
            foreground_color: Some(crossterm::style::Color::Yellow),
            background_color: None,
            underline_color: None,
            attributes,
            // ..Default::default()
        };

        // there is no need to flush vector, because its not real terminal so it doesn't buffer anything
        // both execute! and queue! will immediately add bytes to the vector
        crossterm::queue!(
            expected,
            crossterm::style::PrintStyledContent(style.apply(line))
        )
        .unwrap();
    }
    assert_eq!(s_err, String::from_utf8(expected).unwrap());
}

#[test]
fn test_everything() {
    let expected = concat!(
        "test_files/\n",
        "test_files/.hidden\n",
        "test_files/dir/\n",
        "test_files/dir/regular\n",
        "test_files/no_read\n",
        "test_files/regular\n",
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    assert_eq!(
        run(["test_files", "-0arPdl12", "-fo", path, "--separator", "sep"]),
        expected
    );
    // you can give it a file or path, both work
    let out = std::fs::read_to_string(file).unwrap();
    assert_eq!(out, expected);
}
