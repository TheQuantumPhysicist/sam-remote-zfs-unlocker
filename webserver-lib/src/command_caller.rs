use common::types::RunCommandOutput;
use tokio::io::AsyncReadExt;

#[derive(thiserror::Error, Debug, Clone)]
pub enum CommandError {
    #[error("Attempted to call empty command")]
    EmptyCommand,
    #[error("Call failed: {0}")]
    CallFailed(String),
    #[error("System error: {0}")]
    SystemError(String),
}

pub async fn run_command(cmd_with_args: &[String]) -> Result<RunCommandOutput, CommandError> {
    let (program, args) = cmd_with_args
        .split_first()
        .map(|(first, rest)| (first.clone(), rest.to_vec()))
        .ok_or(CommandError::EmptyCommand)?;

    let mut cmd = args
        .iter()
        .fold(tokio::process::Command::new(program), |mut cmd, arg| {
            cmd.arg(arg);
            cmd
        });

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CommandError::CallFailed(e.to_string()))?;

    // Capture the stdout handle of the child process
    let mut stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut stderr = child.stderr.take().expect("Failed to capture stderr");

    // Read stdout/stderr to a string
    let mut stdout_string = String::new();
    AsyncReadExt::read_to_string(&mut stdout, &mut stdout_string)
        .await
        .map_err(|e| CommandError::SystemError(e.to_string()))?;
    let mut stderr_string = String::new();
    AsyncReadExt::read_to_string(&mut stderr, &mut stderr_string)
        .await
        .map_err(|e| CommandError::SystemError(e.to_string()))?;

    // Wait for the command to complete
    let status = child
        .wait()
        .await
        .map_err(|e| CommandError::SystemError(e.to_string()))?;

    if status.success() {
        Ok(RunCommandOutput {
            stdout: stdout_string,
            stderr: stderr_string,
            error_code: status.code().unwrap_or(0),
        })
    } else {
        Ok(RunCommandOutput {
            stdout: stdout_string,
            stderr: stderr_string,
            error_code: status.code().unwrap_or(255),
        })
    }
}
