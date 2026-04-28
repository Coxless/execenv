use assert_cmd::Command;
use std::io::Write as _;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn sops_available() -> bool {
    std::process::Command::new("sops")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("age-keygen")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

/// Returns `true` if the test should run.
/// Returns `false` (after printing a skip message) if sops/age-keygen are missing.
/// Panics if `EXECENV_REQUIRE_E2E=1` and the tools are missing (used in CI).
pub fn require_e2e_or_skip(test_name: &str) -> bool {
    if sops_available() {
        return true;
    }
    if std::env::var("EXECENV_REQUIRE_E2E").as_deref() == Ok("1") {
        panic!("EXECENV_REQUIRE_E2E=1 but sops or age-keygen not on PATH (test: {test_name})");
    }
    eprintln!("skipping {test_name}: sops or age-keygen not on PATH");
    false
}

pub struct Fixture {
    pub _tmp: TempDir,
    pub enc_path: PathBuf,
    pub key_path: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Overwrite the private key with zeros before TempDir removes the directory.
        if let Ok(meta) = std::fs::metadata(&self.key_path) {
            let zeros = vec![0u8; meta.len() as usize];
            let _ = std::fs::write(&self.key_path, zeros);
        }
    }
}

/// Generate a fresh age key and sops-encrypt `plaintext` into a `TempDir`.
/// `format` is `"dotenv"` or `"json"`.
pub fn make_fixture(plaintext: &str, format: &str) -> Fixture {
    let tmp = TempDir::new().expect("TempDir::new");
    let key_path = tmp.path().join("key.txt");
    let enc_path = tmp.path().join(format!("secret.{format}.enc"));

    let status = std::process::Command::new("age-keygen")
        .arg("-o")
        .arg(&key_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("age-keygen failed to start");
    assert!(status.success(), "age-keygen exited non-zero");

    let key_content = std::fs::read_to_string(&key_path).expect("read age key file");
    let recipient = key_content
        .lines()
        .find(|l| l.starts_with("# public key: "))
        .expect("public key line not found in age-keygen output")
        .trim_start_matches("# public key: ")
        .trim()
        .to_string();

    let mut child = std::process::Command::new("sops")
        .args([
            "--encrypt",
            "--age",
            &recipient,
            "--input-type",
            format,
            "--output-type",
            format,
            "/dev/stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("sops --encrypt failed to start");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(plaintext.as_bytes())
        .expect("write plaintext to sops stdin");

    let output = child
        .wait_with_output()
        .expect("sops --encrypt wait failed");
    assert!(
        output.status.success(),
        "sops --encrypt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(&enc_path, &output.stdout).expect("write encrypted fixture");

    Fixture {
        _tmp: tmp,
        enc_path,
        key_path,
    }
}

/// Returns an `assert_cmd::Command` targeting the `execenv` binary, with
/// `SOPS_AGE_KEY_FILE` set to the fixture key and the working directory set to
/// a subdirectory of the fixture `TempDir` (so no ambient `.sops.yaml` is picked up).
pub fn execenv_cmd(fx: &Fixture) -> Command {
    let work_dir = fx._tmp.path().join("work");
    std::fs::create_dir_all(&work_dir).expect("create work dir");

    let mut cmd = Command::cargo_bin("execenv").expect("execenv binary not found");
    cmd.env("SOPS_AGE_KEY_FILE", &fx.key_path)
        .current_dir(&work_dir);
    cmd
}
