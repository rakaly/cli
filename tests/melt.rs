mod utils;

use assert_cmd::Command;
use std::{collections::HashMap, path::Path};

#[test]
fn test_eu4_melt() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd.arg("melt").arg(&file).assert();

    assert.success();

    let melted_path = file.with_file_name("kandy2.bin_melted.eu4");
    assert!(melted_path.exists());

    let data = std::fs::read(&melted_path).unwrap();
    let file = eu4save::Eu4File::from_slice(&data).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_melt_stdout() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd.arg("melt").arg("--to-stdout").arg(&file).assert();

    let out = assert.get_output();
    let file = eu4save::Eu4File::from_slice(&out.stdout).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_specify_format() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let off_path = file.with_extension("");
    std::fs::copy(file, &off_path).unwrap();

    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("melt")
        .arg("--to-stdout")
        .arg("--format")
        .arg("eu4")
        .arg(&off_path)
        .assert();

    let out = assert.get_output();
    let file = eu4save::Eu4File::from_slice(&out.stdout).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_melt_to_out() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let output_path = Path::new("assets").join("saves").join("my_save");
    cmd.arg("melt")
        .arg("--out")
        .arg(&output_path)
        .arg(&file)
        .assert()
        .success();

    let data = std::fs::read(&output_path).unwrap();
    let file = eu4save::Eu4File::from_slice(&data).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_melt_stdin_to_stdout() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("melt")
        .arg("--format")
        .arg("eu4")
        .pipe_stdin(&file)
        .unwrap()
        .assert();

    let out = assert.get_output();
    let file = eu4save::Eu4File::from_slice(&out.stdout).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_melt_retain() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("melt")
        .arg("--retain")
        .arg("--to-stdout")
        .arg(&file)
        .assert();

    let out = assert.get_output();
    let file = eu4save::Eu4File::from_slice(&out.stdout).unwrap();
    let _save = file.parse_save(&HashMap::<u16, &str>::new()).unwrap();
    assert_eq!(file.encoding(), eu4save::Encoding::Text)
}

#[test]
fn test_eu4_no_filename() {
    let file = utils::request("eu4saves-test-cases", "kandy2.bin.eu4");
    let off_path = file.with_file_name(".eu4");
    std::fs::copy(&file, &off_path).unwrap();

    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd.arg("melt").arg(&off_path).assert();
    assert.success();

    let melted_path = file.with_file_name("melted.eu4");
    assert!(melted_path.exists());
}
