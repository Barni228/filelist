// Here are some guides for these tests:
// I ALWAYS compare expected to actual,
// so like
//
// ```
// assert_eq!(expected, actual);
// ```
// so left is correct, right is actual output

use assert_cmd::{Command, prelude::*};
use crossterm::style::Stylize;
use itertools::Itertools;
use std::io::Write;
use tempfile::NamedTempFile;

// NOTE: if you clone this repo, make sure to create test_files/no_read file
// touch test_files/no_read
// chmod 000 test_files/no_read

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn run(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> String {
    let output = cmd_output(args);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    String::from_utf8(output.stdout).unwrap()
}

fn cmd_output(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> std::process::Output {
    Command::cargo_bin("filelist")
        .unwrap()
        .args(args)
        .output()
        .unwrap()
}

fn bytes_to_vec(bytes: impl AsRef<[u8]>) -> Vec<String> {
    String::from_utf8(bytes.as_ref().to_vec())
        .unwrap()
        .split('\n')
        .map(String::from)
        .collect()
}

fn bytes_to_vec_sorted(bytes: impl AsRef<[u8]>) -> Vec<String> {
    bytes_to_vec(bytes).into_iter().sorted_unstable().collect()
}

#[test]
fn test_simple_cli() {
    assert_eq!(
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ),
        run(["test_files"])
    );
}

#[test]
fn test_clean_path() {
    for i in [".", "./.", "./././", "./dir/..", "dir/./..", "dir/.."] {
        let output = Command::cargo_bin("filelist")
            .unwrap()
            .current_dir("test_files")
            .args(["-ed", i])
            .output()
            .unwrap();

        assert!(output.status.success());
        let expected = concat!(
            "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  ./\n",
            "11f9c53c2abc7d5a9f442687280f80bd5419feaf55af2e598e26d9b285d63ffd  dir/\n",
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  dir/regular\n",
            "ERROR: Permission denied (os error 13)  no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  regular\n",
        );
        assert_eq!(expected, String::from_utf8(output.stdout).unwrap());
        assert_eq!(
            bytes_to_vec_sorted(expected),
            bytes_to_vec_sorted(output.stderr)
        )
    }
}

#[test]
fn test_no_args() {
    // unfortunately, assert_cmd makes filelist think that it has stdin piped to it
    // `printf "" | filelist`
    // so because of that, without real terminal I cannot test what it does without stdin piped
    // so just test that yourself, and here i give it "." (current dir)
    let output = Command::cargo_bin("filelist")
        .unwrap()
        .current_dir("test_files")
        .arg(".")
        .output()
        .unwrap();

    assert!(output.status.success());

    assert_eq!(
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  dir/regular\n",
            "ERROR: Permission denied (os error 13)  no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  regular\n",
        ),
        String::from_utf8(output.stdout).unwrap()
    );
}

#[test]
fn test_length_0() {
    assert_eq!(
        concat!(
            "  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "  test_files/regular\n",
        ),
        run(["test_files", "-l", "0"])
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
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();
    let out = run(["test_files", "-fo", path]);
    assert!(out.is_empty());
    let written = std::fs::read_to_string(path).unwrap();
    assert_eq!(
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ),
        written
    );
}

#[test]
fn test_output_file_exists() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();
    Command::cargo_bin("filelist")
        .unwrap()
        .args(["-o", path])
        .assert()
        .failure();
    let written = std::fs::read_to_string(path).unwrap();
    assert!(written.is_empty());
}

#[test]
fn test_default_len() {
    for i in ["-l=64", "--length=64", "-l64"] {
        assert_eq!(run(["test_files", i]), run(["test_files"]));
    }
}

#[test]
fn test_no_hash() {
    for i in ["-0", "--no-hash"] {
        assert_eq!(
            concat!(
                "test_files/dir/regular\n",
                "test_files/no_read\n",
                "test_files/regular\n",
            ),
            run(["test_files", i])
        );
    }
}

#[test]
fn test_all() {
    for i in ["-a", "--all"] {
        assert_eq!(
            concat!(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  test_files/.hidden\n",
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13)  test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            ),
            run(["test_files", i])
        );
    }
}

#[test]
fn test_separator() {
    for i in ["--sep", "--separator"] {
        assert_eq!(
            concat!(
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f \t test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13) \t test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95 \t test_files/regular\n",
            ),
            run(["test_files", i, " \t "])
        );
    }
}

#[test]
fn test_absolute() {
    for i in ["-A", "--absolute"] {
        assert_eq!(
            concat!(
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
                "ERROR: No such file or directory (os error 2)  test_files/no_exist\n",
                "ERROR: Permission denied (os error 13)  test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n"
            ).replace("test_files", &format!("{ROOT}/test_files")),
            run(["test_files", "test_files/no_exist", i])
        );
    }
}

#[test]
fn test_multiple_files() {
    assert_eq!(
        concat!(
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ),
        run(["test_files/no_read", "test_files/regular"])
    );
}

#[test]
fn test_pass_hidden() {
    assert_eq!(
        concat!(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  test_files/.hidden\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ),
        run(["test_files/regular", "test_files/.hidden"])
    );
}

#[test]
fn test_same_files() {
    let same_paths = ["test_files/regular", "./test_files/regular"];
    for i in same_paths.into_iter().cartesian_product(same_paths) {
        let output = cmd_output(["-e", i.0, i.1]);
        assert_eq!(
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            String::from_utf8(output.stdout).unwrap()
        );
        assert_eq!(
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            String::from_utf8(output.stderr).unwrap()
        );
    }
}

#[test]
fn test_does_not_exist() {
    assert_eq!(
        "ERROR: No such file or directory (os error 2)  test_files/no_exist\n",
        run(["test_files/no_exist"])
    );
}

#[test]
fn test_files_and_dirs() {
    assert_eq!(
        concat!(
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ),
        run(["test_files/regular", "test_files"])
    );
}

#[test]
fn test_no_recursive() {
    for i in ["-R", "--no-recursive"] {
        assert_eq!(
            "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n",
            run(["test_files", i])
        );
    }
}

#[test]
fn test_many_recursive() {
    assert_eq!(
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n",
        run(["-rRrRrR", "test_files"])
    )
}

#[test]
fn test_directory() {
    for i in ["-d", "--directory"] {
        assert_eq!(
            concat!(
                "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n",
                "11f9c53c2abc7d5a9f442687280f80bd5419feaf55af2e598e26d9b285d63ffd  test_files/dir/\n",
                "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
                "ERROR: Permission denied (os error 13)  test_files/no_read\n",
                "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
            ),
            run(["test_files", i])
        );
    }
}

#[test]
fn test_hash_directory_all() {
    // hash of directory changes based on whether or not all is set
    assert_eq!(
        "ce0d379ccd77402b64055d6852c6e1a11485206517da05c988309fa6029e0e20  test_files/\n",
        run(["-R", "test_files"])
    );
    assert_eq!(
        "72676a6eb3c35529a7c450d195045d660137a77d47cd9b980e508a76ce396def  test_files/\n",
        run(["-Ra", "test_files"])
    );
}

#[test]
fn test_parent_directory() {
    let same_replaced = ["-0", "-a", "-l12", "--sep=___", "-d"];
    for i in same_replaced.iter().powerset() {
        // output should be exactly the same as usual, except that `test_files` is now `..`
        let expected = run(["test_files"].iter().chain(i.clone())).replace("test_files", "..");
        Command::cargo_bin("filelist")
            .unwrap()
            .current_dir("test_files/dir")
            .args([".."].iter().chain(i))
            .output()
            .unwrap()
            .assert()
            .success()
            .stdout(expected);
    }
}

#[test]
fn test_symlink() {
    assert_eq!(
        concat!(
            "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir/inside\n",
            "2b64c6d9afd8a34ed0dbf35f7de171a8825a50d9f42f05e98fe2b1addf00ab44  symlink_test_files/dir-link\n",
            "803d20d7842eea06d21fd4268c460341b74079dae74101dfa3054eb54fdf1073  symlink_test_files/link\n",
        ),
        run(["symlink_test_files"])
    );

    assert_eq!(
        concat!(
            "d5228f4fec446513faea914e38c3d11b76d10f4697b7e1f6a869bc99139d5314  symlink_test_files/\n",
            "382e80dff26044e8835c695603ad137d77bbe87244a8329346746565ab38cb91  symlink_test_files/dir/\n",
            "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir/inside\n",
            "2b64c6d9afd8a34ed0dbf35f7de171a8825a50d9f42f05e98fe2b1addf00ab44  symlink_test_files/dir-link\n",
            "803d20d7842eea06d21fd4268c460341b74079dae74101dfa3054eb54fdf1073  symlink_test_files/link\n",
        ),
        run(["symlink_test_files", "-d"])
    );
}

#[test]
fn test_symlink_follow() {
    for i in ["-s", "--link"] {
        assert_eq!(
            concat!(
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir/inside\n",
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir-link/inside\n",
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/link\n",
            ),
            run(["symlink_test_files", i])
        );

        assert_eq!(
            concat!(
                "a44b34cd400925735e609d894df5d62f9102244c6dc408bd4ce87f847f668f0b  symlink_test_files/\n",
                "382e80dff26044e8835c695603ad137d77bbe87244a8329346746565ab38cb91  symlink_test_files/dir/\n",
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir/inside\n",
                "382e80dff26044e8835c695603ad137d77bbe87244a8329346746565ab38cb91  symlink_test_files/dir-link/\n",
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/dir-link/inside\n",
                "9279f4a7c1c145d5ae930fda23ef386168f6720b4e0f0d3dee383c5ad8535737  symlink_test_files/link\n",
            ),
            run(["symlink_test_files", "-d", i])
        );
    }
}

#[test]
fn test_stdin() {
    let expected_regular = "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  -\n";

    Command::cargo_bin("filelist")
        .unwrap()
        .arg("-")
        .pipe_stdin("test_files/regular")
        .unwrap()
        .output()
        .unwrap()
        .assert()
        .success()
        .stdout(expected_regular);

    let expected_hi = "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4  -\n";

    Command::cargo_bin("filelist")
        .unwrap()
        .arg("-")
        .write_stdin("hi")
        .output()
        .unwrap()
        .assert()
        .success()
        .stdout(expected_hi);
}

#[test]
fn test_stdin_piped() {
    let expected_regular = "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  -\n";

    Command::cargo_bin("filelist")
        .unwrap()
        .pipe_stdin("test_files/regular")
        .unwrap()
        .output()
        .unwrap()
        .assert()
        .success()
        .stdout(expected_regular);
}

#[test]
fn test_progress_hash() {
    let same_unordered = ["-0", "-a", "-l12", "--sep=___", "-d", "--parallel"];
    // powerset will give us all possible combinations, like for `[a, b]` it will give `[], [a], [b], [a, b]`
    for i in same_unordered.iter().powerset() {
        let output = cmd_output(["-e", "test_files"].iter().chain(i));

        let out = bytes_to_vec_sorted(output.stdout);
        let err = bytes_to_vec_sorted(output.stderr);
        assert_eq!(out, err);
    }
}

#[test]
fn test_progress_hash_file() {
    let same_output = ["-0", "-a", "-l12", "--sep=___", "-d"];

    for i in same_output.iter().powerset() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();

        let output = cmd_output(["-e", "test_files", "-fo", path].iter().chain(i));

        assert!(output.stdout.is_empty());

        let err = bytes_to_vec_sorted(output.stderr);
        let s_file_content = std::fs::read_to_string(path).unwrap();
        let file_content = s_file_content.split('\n').sorted_unstable().collect_vec();

        assert_eq!(file_content, err);
    }
}

#[test]
fn test_progress_hash_no_recursion() {
    let same_output = ["-a", "-l12", "--sep=___"];

    for i in same_output.iter().powerset() {
        let output = cmd_output(["-eR", "test_files"].iter().chain(i.clone()));
        assert!(output.status.success());

        let err = bytes_to_vec_sorted(output.stderr);
        let s_real_output = run(["-d", "test_files"].iter().chain(i));
        let real_output = bytes_to_vec_sorted(s_real_output);

        assert_eq!(real_output, err);
    }
}

#[test]
fn test_progress_bar() {
    for i in ["-p", "--progress-bar"] {
        // if this fails, then maybe you changed progress bar logic
        let output = cmd_output([i, "test_files"]);
        let s_out = String::from_utf8(output.stdout).unwrap();
        let expected_out = concat![
            "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
            "ERROR: Permission denied (os error 13)  test_files/no_read\n",
            "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
        ];
        assert_eq!(expected_out, s_out);

        let s_err = String::from_utf8(output.stderr).unwrap();
        let expected_err = concat![""];
        assert_eq!(expected_err, s_err);
    }
}

#[test]
fn test_color_auto() {
    let output_auto = cmd_output(["test_files", "-e", "--color=auto"]);

    let auto_out = bytes_to_vec(output_auto.stdout);
    let auto_err = bytes_to_vec_sorted(output_auto.stderr);

    let output_never = cmd_output(["test_files", "-e", "--color=never"]);

    let never_out = bytes_to_vec(output_never.stdout);
    let never_err = bytes_to_vec_sorted(output_never.stderr);

    assert_eq!(never_out, auto_out);
    assert_eq!(never_err, auto_err);
}

#[test]
fn test_color_always() {
    // if this test fails, then maybe you just changed the style of -e output
    let output = cmd_output(["test_files", "-e", "--color=always"]);
    assert!(output.status.success());

    let err = bytes_to_vec(output.stderr);

    // you can write to vector as if its stdout
    // since stdout is technically a Vec<u8>
    let mut expected_buffer = Vec::new();
    let mut expected_lines = [
        "dd57c65a5219917d4c423ce6a0bf2d9540b403ae9a0259406103fa08fe26117f  test_files/dir/regular\n",
        "ERROR: Permission denied (os error 13)  test_files/no_read\n",
        "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
    ];

    // order the lines in the same way filelist printed them
    expected_lines.sort_unstable_by_key(|l| {
        err.iter()
            .position(|e| e.contains(&l.replace('\n', "")))
            .unwrap()
    });

    for line in expected_lines {
        // there is no need to flush vector, because its not real terminal so it doesn't buffer anything
        // both execute! and queue! will immediately add bytes to the vector
        write!(expected_buffer, "{}", line.yellow().dim()).unwrap();
    }
    let expected = bytes_to_vec(expected_buffer);
    assert_eq!(expected, err);
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
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap();

    assert_eq!(
        "",
        run(["test_files", "-a0dl12", "-fo", path, "--separator", "sep"])
    );
    // you can give it a file or path, both work
    let out = std::fs::read_to_string(file).unwrap();
    assert_eq!(expected, out);
}
