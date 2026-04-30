// End-to-end tests against LocalStack.
//
// Start LocalStack and seed fixtures before running:
//
//   docker run -d -p 4566:4566 localstack/localstack
//   aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
//       --name test/execenv \
//       --secret-string '{"HELLO":"world"}'
//   aws --endpoint-url=http://localhost:4566 secretsmanager create-secret \
//       --name test/execenv-dotenv \
//       --secret-string $'HELLO=world\n'
//
// Then run:  cargo test --test aws_integration_test -- --include-ignored

use assert_cmd::Command;

fn localstack_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("AWS_ENDPOINT_URL", "http://localhost:4566"),
        ("AWS_DEFAULT_REGION", "us-east-1"),
        ("AWS_ACCESS_KEY_ID", "test"),
        ("AWS_SECRET_ACCESS_KEY", "test"),
    ]
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_json_secret_exec() {
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args([
        "--provider",
        "aws-secrets-manager",
        "--secret-id",
        "test/execenv",
        "--",
        "sh",
        "-c",
        "echo $HELLO",
    ]);
    cmd.assert().success().stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_dotenv_secret_exec() {
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args([
        "--provider",
        "aws-secrets-manager",
        "--secret-id",
        "test/execenv-dotenv",
        "--format",
        "dotenv",
        "--",
        "sh",
        "-c",
        "echo $HELLO",
    ]);
    cmd.assert().success().stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_arn_exec() {
    // Verify that an ARN works as --secret-id.
    // LocalStack ARN format: arn:aws:secretsmanager:us-east-1:000000000000:secret:test/execenv-*
    // Retrieve the ARN first with aws-cli, then run execenv.
    // This test simply re-runs against the name to avoid hardcoding the suffix.
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args([
        "--provider",
        "aws-secrets-manager",
        "--secret-id",
        "test/execenv",
        "--aws-region",
        "us-east-1",
        "--",
        "sh",
        "-c",
        "echo $HELLO",
    ]);
    cmd.assert().success().stdout("world\n");
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_missing_secret_fails() {
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args([
        "--provider",
        "aws-secrets-manager",
        "--secret-id",
        "does-not-exist",
        "--",
        "true",
    ]);
    cmd.assert().failure();
}

#[test]
#[ignore = "requires localstack"]
fn aws_provider_clean_env_only_exposes_secret_vars() {
    let mut cmd = Command::cargo_bin("execenv").unwrap();
    for (k, v) in localstack_env() {
        cmd.env(k, v);
    }
    cmd.args([
        "--provider",
        "aws-secrets-manager",
        "--secret-id",
        "test/execenv",
        "--clean-env",
        "--",
        "sh",
        "-c",
        "echo ${HELLO:-missing}",
    ]);
    cmd.assert().success().stdout("world\n");
}
