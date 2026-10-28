use std::process::Command;

#[test]
fn help_describes_the_actual_skeleton() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_hearth"))
        .arg("--help")
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("Usage: hearth [--help | --version]"));
    assert!(stdout.contains("not implemented yet"));
    Ok(())
}

#[test]
fn unknown_commands_fail_instead_of_claiming_success() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_hearth"))
        .arg("promote")
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("hearth --help"));
    Ok(())
}

#[test]
fn slop_runner_stays_hidden_from_help() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_hearth"))
        .arg("--help")
        .output()?;
    assert!(!String::from_utf8(output.stdout)?.contains("slop"));
    Ok(())
}

#[test]
fn slop_runner_refuses_to_start_without_a_terminal() -> Result<(), Box<dyn std::error::Error>> {
    // Piped stdio is not a terminal, so the game must fail cleanly instead of
    // putting a non-interactive caller into raw mode.
    let output = Command::new(env!("CARGO_BIN_EXE_hearth"))
        .arg("--slop")
        .stdin(std::process::Stdio::null())
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)?.contains("needs an interactive terminal"));
    Ok(())
}
