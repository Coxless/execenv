mod helpers;

use assert_cmd::Command;
use helpers::{execenv_cmd, make_fixture, require_e2e_or_skip};
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

// ── CLI surface (no sops required) ──────────────────────────────────────────

#[test]
fn cli_help_succeeds() {
    Command::cargo_bin("execenv")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--provider"))
        .stdout(contains("--file"));
}

#[test]
fn cli_missing_command_errors() {
    Command::cargo_bin("execenv")
        .unwrap()
        .args(["--provider", "sops", "--file", "foo.enc"])
        .assert()
        .failure()
        .code(2);
}

// ── End-to-end (requires sops + age-keygen on PATH) ─────────────────────────

#[test]
fn decrypts_and_execs_dotenv() {
    if !require_e2e_or_skip("decrypts_and_execs_dotenv") {
        return;
    }
    let fx = make_fixture("FOO=bar\nPATH=/usr/bin:/bin\n", "dotenv");
    execenv_cmd(&fx)
        .args(["--clean-env", "--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .assert()
        .success()
        .stdout(contains("FOO=bar"))
        .stdout(contains("HOME=").not());
}

#[test]
fn decrypts_and_execs_json() {
    if !require_e2e_or_skip("decrypts_and_execs_json") {
        return;
    }
    let fx = make_fixture(r#"{"FOO":"bar","PATH":"/usr/bin:/bin"}"#, "json");
    execenv_cmd(&fx)
        .args(["--clean-env", "--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--format", "json", "--", "/usr/bin/env"])
        .assert()
        .success()
        .stdout(contains("FOO=bar"))
        .stdout(contains("HOME=").not());
}

#[test]
fn secret_value_does_not_leak_to_stderr() {
    if !require_e2e_or_skip("secret_value_does_not_leak_to_stderr") {
        return;
    }
    let secret = "correct-horse-battery-staple";
    let fx = make_fixture(&format!("EXECENV_TEST_SECRET={secret}\n"), "dotenv");
    let output = execenv_cmd(&fx)
        .args(["--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .output()
        .unwrap();

    assert!(output.status.success(), "execenv failed unexpectedly");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(secret),
        "plaintext secret leaked to stderr: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("EXECENV_TEST_SECRET={secret}")),
        "secret not found in /usr/bin/env output"
    );
}

#[test]
fn missing_file_errors() {
    if !require_e2e_or_skip("missing_file_errors") {
        return;
    }
    Command::cargo_bin("execenv")
        .unwrap()
        .args([
            "--provider",
            "sops",
            "--file",
            "/nonexistent-path-execenv-test.enc",
            "--",
            "/usr/bin/env",
        ])
        .assert()
        .failure();
}

#[test]
fn wrong_age_key_errors() {
    if !require_e2e_or_skip("wrong_age_key_errors") {
        return;
    }
    let secret = "correct-horse";
    let fx = make_fixture(&format!("SECRET={secret}\nPATH=/usr/bin:/bin\n"), "dotenv");

    // Generate a different age key to use as the wrong decryption key.
    let wrong_tmp = tempfile::TempDir::new().unwrap();
    let wrong_key = wrong_tmp.path().join("wrong_key.txt");
    let status = std::process::Command::new("age-keygen")
        .arg("-o")
        .arg(&wrong_key)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    let output = execenv_cmd(&fx)
        .env("SOPS_AGE_KEY_FILE", &wrong_key)
        .args(["--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "expected failure with wrong key");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(secret),
        "plaintext leaked to stderr on decryption error: {stderr}"
    );
}

#[test]
fn default_inherits_parent_env() {
    if !require_e2e_or_skip("default_inherits_parent_env") {
        return;
    }
    let fx = make_fixture("EXECENV_TEST_VAR=hello\n", "dotenv");
    execenv_cmd(&fx)
        .args(["--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .assert()
        .success()
        .stdout(contains("EXECENV_TEST_VAR=hello"))
        .stdout(contains("HOME="));
}

#[test]
fn decrypted_value_wins_over_parent_env() {
    if !require_e2e_or_skip("decrypted_value_wins_over_parent_env") {
        return;
    }
    let fx = make_fixture("EXECENV_TEST_OVERRIDE=decrypted\n", "dotenv");
    execenv_cmd(&fx)
        .env("EXECENV_TEST_OVERRIDE", "parent")
        .args(["--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .assert()
        .success()
        .stdout(contains("EXECENV_TEST_OVERRIDE=decrypted"))
        .stdout(contains("EXECENV_TEST_OVERRIDE=parent").not());
}

#[test]
fn clean_env_drops_parent_env() {
    if !require_e2e_or_skip("clean_env_drops_parent_env") {
        return;
    }
    let fx = make_fixture("PATH=/usr/bin:/bin\n", "dotenv");
    execenv_cmd(&fx)
        .args(["--clean-env", "--provider", "sops", "--file"])
        .arg(&fx.enc_path)
        .args(["--", "/usr/bin/env"])
        .assert()
        .success()
        .stdout(contains("HOME=").not());
}
