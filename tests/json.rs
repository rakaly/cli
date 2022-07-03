use std::path::Path;

use assert_cmd::Command;

#[test]
fn test_json() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(r#"{"a":"b","a":1}"#, actual);
}

#[test]
fn test_pretty_json() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .arg("--pretty")
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!("{\n  \"a\": \"b\",\n  \"a\": 1\n}", actual);
}

#[test]
fn test_json_duplicate_key_group() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .arg("--duplicate-keys")
        .arg("group")
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(r#"{"a":["b",1]}"#, actual);
}

#[test]
fn test_json_duplicate_key_typed() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .arg("--duplicate-keys")
        .arg("key-value-pairs")
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(r#"{"type":"obj","val":[["a","b"],["a",1]]}"#, actual);
}

#[test]
fn test_json_utf8() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .arg("--format")
        .arg("utf-8")
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(r#"{"a":"b","a":1}"#, actual);
}

#[test]
fn test_json_windows1252() {
    let mut cmd = Command::cargo_bin("rakaly").unwrap();
    let assert = cmd
        .arg("json")
        .arg(&Path::new("tests").join("fixtures").join("json.txt"))
        .arg("--format")
        .arg("windows-1252")
        .assert()
        .success();

    let out = assert.get_output();
    let actual = std::str::from_utf8(&out.stdout).unwrap();
    assert_eq!(r#"{"a":"b","a":1}"#, actual);
}
