use common::types::RunCommandOutput;

use super::error::Error;

#[derive(thiserror::Error, Debug, Clone)]
pub enum CommandError {
    #[error("Attempted to call empty command")]
    EmptyCommand,
    #[error("Call failed: {0}")]
    CallFailed(String),
    #[error("System error: {0}")]
    SystemError(String),
    #[error("Failed to retrieve stdin system pipe")]
    StdinPipe,
}

#[allow(dead_code)]
async fn run_command(
    cmd_with_args: &[String],
    stdin: Option<String>,
) -> Result<RunCommandOutput, CommandError> {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, BufWriter},
        try_join,
    };

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

    // Pipe stdin, if desired by the caller
    let mut child_stdin = child.stdin.take();
    if let Some(stdin_string) = stdin {
        match child_stdin.as_mut() {
            Some(stdin_pipe) => {
                // Create Vec from the input string
                let stdin_data = {
                    use std::io::Write;
                    let mut write_buffer = Vec::new();
                    let mut writer = std::io::BufWriter::new(&mut write_buffer);
                    writeln!(&mut writer, "{}", stdin_string).expect("Cannot fail in memory write");
                    drop(writer);
                    write_buffer
                };

                // Write the data to stdin asynchronously
                let mut async_writer = BufWriter::new(stdin_pipe);
                async_writer
                    .write_all(&stdin_data)
                    .await
                    .map_err(|e| CommandError::SystemError(e.to_string()))?;

                // Flush the writer to ensure all data is sent
                async_writer
                    .flush()
                    .await
                    .map_err(|e| CommandError::SystemError(e.to_string()))?;
            }
            None => return Err(CommandError::StdinPipe),
        }
    }

    // Signal we're done with stdin by dropping it
    drop(child_stdin);

    // Capture the stdout handle of the child process
    let mut stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_string = String::new();
    let mut stderr_string = String::new();

    let (_, _) = try_join!(
        // Read stdout/stderr to a string
        AsyncReadExt::read_to_string(&mut stdout, &mut stdout_string),
        AsyncReadExt::read_to_string(&mut stderr, &mut stderr_string),
    )
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

pub async fn chain_commands(
    commands: &Vec<Vec<String>>,
    initial_stdin: Option<String>,
) -> Result<RunCommandOutput, Error> {
    if commands.is_empty() {
        return Err(Error::NoCommandsProvided);
    }
    let mut current_stdin = initial_stdin;

    let mut result = RunCommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        error_code: 254,
    };

    for command in commands {
        result = match run_command(command, current_stdin).await {
            Ok(result) => result,
            Err(e) => {
                return Ok(RunCommandOutput {
                    stdout: String::new(),
                    stderr: e.to_string(),
                    error_code: 253,
                })
            }
        };

        if result.error_code != 0 {
            break;
        }

        current_stdin = Some(result.stdout.clone());
    }

    Ok(result)
}
