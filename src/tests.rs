// use assert_cmd::Command;
// use clap::error::ErrorKind;

// use super::*;
// use std::path::PathBuf;

// fn run(args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> String {
//     let output = Command::cargo_bin("filelist")
//         .unwrap()
//         .args(args)
//         .output()
//         .unwrap();

//     assert!(output.status.success());

//     String::from_utf8(output.stdout).unwrap()
// }

// #[test]
// fn test_dir() {
//     let args = vec!["filelist", "this/"];
//     let matches = get_clap_command().get_matches_from(args);

//     assert_eq!(
//         // matches.get_many::<PathBuf>("DIR").unwrap(),
//         matches
//             .get_many::<PathBuf>("PATHS")
//             .unwrap()
//             .cloned()
//             .collect::<Vec<_>>(),
//         vec![PathBuf::from("this/")]
//     );
// }

// #[test]
// fn test_no_args() {
//     let args = vec!["filelist"];
//     let matches = get_clap_command().try_get_matches_from(args);

//     assert_eq!(
//         matches.unwrap_err().kind(),
//         ErrorKind::MissingRequiredArgument
//     );
// }

// #[test]
// fn test_file() {
//     let args = vec!["filelist", ".", "-o", "some/path"];
//     let matches = get_clap_command().get_matches_from(args);

//     assert_eq!(
//         matches.get_one::<PathBuf>("output").unwrap(),
//         &PathBuf::from("some/path")
//     );
// }

// #[test]
// fn test_permission_error() {
//     let no_read_file = PathBuf::from(format!(
//         "{}/{}",
//         env!("CARGO_MANIFEST_DIR"),
//         "test_files/no_read"
//     ));

//     assert_eq!(
//         hash_no_error_len(&no_read_file, 0),
//         "ERROR: Permission denied (os error 13)"
//     );
// }

// #[test]
// fn test_simple_cli() {
//     assert_eq!(
//         run(["test_files"]),
//         concat!(
//             "ERROR: Permission denied (os error 13)  test_files/no_read\n",
//             "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
//         )
//     );
// }

// #[test]
// fn test_no_args_cli() {
//     Command::cargo_bin("filelist").unwrap().assert().failure();
// }

// #[test]
// fn test_length_0() {
//     assert_eq!(
//         run(["test_files", "-l", "0"]),
//         concat!(
//             "ERROR: Permission denied (os error 13)  test_files/no_read\n",
//             "  test_files/regular\n",
//         )
//     );
// }

// #[test]
// fn test_length_too_big() {
//     Command::cargo_bin("filelist")
//         .unwrap()
//         .arg("-l65")
//         .assert()
//         .failure();
// }

// #[test]
// fn test_write_file() {
//     let file = tempfile::NamedTempFile::new().unwrap();
//     let path = file.path().to_str().unwrap();
//     assert!(run(["test_files", "-o", path]).is_empty());
//     let out = std::fs::read_to_string(path).unwrap();
//     assert_eq!(
//         out,
//         concat!(
//             "ERROR: Permission denied (os error 13)  test_files/no_read\n",
//             "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
//         )
//     );
// }

// #[test]
// fn test_default_len() {
//     // if you don't use =, then -1 will be treated like option -1 and not negative 1
//     for i in ["-l=-1", "--length=-1"] {
//         assert_eq!(run(["test_files", i]), run(["test_files"]),);
//     }
// }

// #[test]
// fn test_no_hash() {
//     for i in ["-0", "--no-hash"] {
//         assert_eq!(
//             run(["test_files", i]),
//             "test_files/no_read\n\
//             test_files/regular\n",
//         );
//     }
// }

// #[test]
// fn test_all() {
//     for i in ["-a", "--all"] {
//         assert_eq!(
//             run(["test_files", i]),
//             concat!(
//                 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  test_files/.hidden\n",
//                 "ERROR: Permission denied (os error 13)  test_files/no_read\n",
//                 "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
//             )
//         );
//     }
// }

// #[test]
// fn test_separator() {
//     for i in ["-s", "--separator"] {
//         assert_eq!(
//             run(["test_files", i, " \t "]),
//             concat!(
//                 "ERROR: Permission denied (os error 13) \t test_files/no_read\n",
//                 "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95 \t test_files/regular\n",
//             )
//         );
//     }
// }

// #[test]
// fn test_print() {
//     let expected = concat!(
//         "ERROR: Permission denied (os error 13)  test_files/no_read\n",
//         "7f44ae7d5074b592265a407f5495aa1207ff15f60353d71b3a085588f90ffe95  test_files/regular\n",
//     );

//     for i in ["-p", "--print"] {
//         let file = tempfile::NamedTempFile::new().unwrap();
//         let path = file.path().to_str().unwrap();
//         assert_eq!(run(["test_files", "-o", path, i]), expected);
//         // you can give it a file or path, both work
//         let out = std::fs::read_to_string(file).unwrap();
//         assert_eq!(out, expected);
//     }
// }
