//! Integration coverage for alphabetical CLI subcommand help.

use std::process::Command;

const MAX_HELP_EXAMPLES: usize = 3;

#[test]
fn command_help_is_ordered_and_concise() {
    check_command_group(&[]);
}

fn check_command_group(path: &[String]) {
    assert!(
        path.len() < 8,
        "unexpectedly deep command tree at `canic {}`",
        path.join(" ")
    );

    let help = command_help(path);
    if !help_usage_matches(path, &help) {
        return;
    }

    let examples = example_commands(&help);
    assert!(
        examples.len() <= MAX_HELP_EXAMPLES,
        "`canic {} --help` has {} examples; expected at most {MAX_HELP_EXAMPLES}\n\n{help}",
        path.join(" "),
        examples.len()
    );

    let subcommands = functional_subcommands(&help);
    let mut expected = subcommands.clone();
    expected.sort_unstable();
    assert_eq!(
        subcommands,
        expected,
        "functional subcommands are not alphabetical for `canic {}`\n\n{help}",
        path.join(" ")
    );

    for subcommand in subcommands {
        let mut child = path.to_vec();
        child.push(subcommand);
        check_command_group(&child);
    }
}

fn example_commands(help: &str) -> Vec<&str> {
    let mut in_examples = false;
    let mut examples = Vec::new();

    for line in help.lines() {
        if line.trim() == "Examples:" {
            in_examples = true;
            continue;
        }
        if !in_examples || line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            break;
        }
        if line.trim_start().starts_with("canic ") {
            examples.push(line.trim());
        }
    }

    examples
}

fn command_help(path: &[String]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_canic"))
        .args(path)
        .arg("--help")
        .output()
        .expect("run canic help");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    strip_ansi(&text)
}

fn help_usage_matches(path: &[String], help: &str) -> bool {
    if path.is_empty() {
        return true;
    }

    let prefix = format!("Usage: canic {}", path.join(" "));
    help.lines().any(|line| {
        line.strip_prefix(&prefix)
            .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(char::is_whitespace))
    })
}

fn functional_subcommands(help: &str) -> Vec<String> {
    let mut in_commands = false;
    let mut subcommands = Vec::new();

    for line in help.lines() {
        if line.trim() == "Commands:" {
            in_commands = true;
            continue;
        }
        if !in_commands {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }

        let indentation = line.len() - line.trim_start().len();
        if indentation == 0 {
            break;
        }
        if indentation != 2 {
            continue;
        }

        let name = line
            .split_whitespace()
            .next()
            .expect("indented command line has a name");
        if name != "help" {
            subcommands.push(name.to_string());
        }
    }

    subcommands
}

fn strip_ansi(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == 'm' {
                    break;
                }
            }
        } else {
            plain.push(ch);
        }
    }
    plain
}
