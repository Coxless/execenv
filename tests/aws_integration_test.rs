// End-to-end tests against LocalStack.
//
// Start LocalStack and seed fixtures before running:
//
//   docker run -d -p 4566:4566 localstack/localstack
//
//   aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
//       --name test/execenv \
//       --secret-string '{"HELLO":"world"}'
//
//   aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
//       --name test/execenv2 \
//       --secret-string '{"GREETING":"hi"}'
//
//   aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
//       --name test/execenv-dotenv \
//       --secret-string $'HELLO=world\n'
//
// Then run:  cargo test --test aws_integration_test -- --include-ignored

use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::Write;

fn localstack_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("AWS_ENDPOINT_URL", "http://localhost:4566"),
        ("AWS_DEFAULT_REGION", "us-east-1"),
        ("AWS_ACCESS_KEY_ID", "test"),
        ("AWS_SECRET_ACCESS_KEY", "test"),
    ]
}

fn aws_cmd(args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args(args);
    cmd
}

// ---------------------------------------------------------------------------
// Single --secret-id
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires localstack"]
fn aws_provider_json_secret_exec() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "test/execenv",
        "--", "sh", "-c", "echo $HELLO",
    ])
    .assert()
    .success()
    .stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_dotenv_secret_exec() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "test/execenv-dotenv",
        "--format", "dotenv",
        "--", "sh", "-c", "echo $HELLO",
    ])
    .assert()
    .success()
    .stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_region_flag() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "test/execenv",
        "--aws-region", "us-east-1",
        "--", "sh", "-c", "echo $HELLO",
    ])
    .assert()
    .success()
    .stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_missing_secret_fails() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "does-not-exist",
        "--", "true",
    ])
    .assert()
    .failure();
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_clean_env_only_exposes_secret_vars() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "test/execenv",
        "--clean-env",
        "--", "sh", "-c", "echo ${HELLO:-missing}",
    ])
    .assert()
    .success()
    .stdout("world\n");
}

// ---------------------------------------------------------------------------
// --secrets-file
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires localstack"]
fn aws_provider_secrets_file_single() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "# comment").unwrap();
    writeln!(f, "test/execenv").unwrap();
    writeln!(f).unwrap(); // blank line

    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secrets-file", f.path().to_str().unwrap(),
        "--", "sh", "-c", "echo $HELLO",
    ])
    .assert()
    .success()
    .stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_secrets_file_multiple_merged() {
    // test/execenv  → HELLO=world
    // test/execenv2 → GREETING=hi
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "test/execenv").unwrap();
    writeln!(f, "test/execenv2").unwrap();

    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secrets-file", f.path().to_str().unwrap(),
        "--", "sh", "-c", "echo $HELLO $GREETING",
    ])
    .assert()
    .success()
    .stdout("world hi\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_secrets_file_and_secret_id_combined() {
    // --secret-id and --secrets-file can be combined; results are merged.
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "test/execenv2").unwrap();

    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--secret-id", "test/execenv",
        "--secrets-file", f.path().to_str().unwrap(),
        "--", "sh", "-c", "echo $HELLO $GREETING",
    ])
    .assert()
    .success()
    .stdout("world hi\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_no_id_no_file_fails() {
    aws_cmd(&[
        "--provider", "aws-secrets-manager",
        "--", "true",
    ])
    .assert()
    .failure()
    .stderr(predicates::str::contains("--secret-id or --secrets-file"));
}
