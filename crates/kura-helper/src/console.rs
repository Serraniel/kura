// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs a program and passes the command line and everything it prints on
//! as [`Event::Output`], for the details panel.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::Sender;

use kura_alpm::apply::Event;

/// The command as a shell would show it.
pub fn command_line(cmd: &Command) -> String {
    std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(OsStr::to_string_lossy)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sends each line of `stream` as output, with `tag` in front, and returns
/// everything read.
pub(crate) fn forward(stream: impl Read, tag: &str, events: &Sender<Event>) -> Vec<u8> {
    let mut all = Vec::new();
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    while reader.read_until(b'\n', &mut line).is_ok_and(|n| n > 0) {
        let text = String::from_utf8_lossy(&line);
        let _ = events.send(Event::Output {
            line: format!("{tag}{}", text.trim_end()),
        });
        all.append(&mut line);
    }
    all
}

/// Runs `cmd` without input. `tag` goes in front of every line, so parallel
/// builds stay apart.
pub fn run(cmd: &mut Command, tag: &str, events: &Sender<Event>) -> std::io::Result<Output> {
    let _ = events.send(Event::Output {
        line: format!("{tag}$ {}", command_line(cmd)),
    });
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let reader = |stream: Option<_>| {
        let (events, tag) = (events.clone(), tag.to_owned());
        std::thread::spawn(move || stream.map(|s| forward(s, &tag, &events)).unwrap_or_default())
    };
    let out = reader(child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>));
    let err = reader(child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>));
    let status = child.wait()?;
    Ok(Output {
        status,
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_on_the_command_and_its_output() {
        let (tx, rx) = std::sync::mpsc::channel();
        let out = run(
            Command::new("sh").args(["-c", "echo one; echo two >&2"]),
            "[x] ",
            &tx,
        )
        .unwrap();
        drop(tx);
        assert!(out.status.success());
        assert_eq!(out.stdout, b"one\n");
        assert_eq!(out.stderr, b"two\n");
        let lines: Vec<String> = rx
            .iter()
            .map(|e| match e {
                Event::Output { line } => line,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(lines[0], "[x] $ sh -c echo one; echo two >&2");
        assert!(lines.contains(&"[x] one".to_owned()));
        assert!(lines.contains(&"[x] two".to_owned()));
    }
}
