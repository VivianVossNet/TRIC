// Copyright 2025 Vivian Voss. Licensed under the Apache License, Version 2.0.
// SPDX-License-Identifier: Apache-2.0
// Scope: tric CLI binary — connects to admin socket, sends command, displays response. REPL via `tric shell`.

use std::io::{self, BufRead, Write};
use std::os::unix::net::UnixDatagram;

const DEFAULT_ADMIN_PATH: &str = "/var/run/tric/admin.sock";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    if args[0] == "shell" {
        run_shell();
    } else {
        let command = args.join(" ");
        let response = send_command(&command);
        print!("{response}");
    }
}

fn run_shell() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        let _ = write!(stdout, "tric> ");
        let _ = stdout.flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if trimmed == "exit" || trimmed == "quit" {
                    break;
                }
                let response = send_command(trimmed);
                print!("{response}");
            }
            Err(_) => break,
        }
    }
}

fn send_command(command: &str) -> String {
    let admin_path =
        std::env::var("TRIC_ADMIN_SOCKET").unwrap_or_else(|_| DEFAULT_ADMIN_PATH.to_string());
    let client_path = format!("/tmp/tric-cli-{}.sock", std::process::id());
    let _ = std::fs::remove_file(&client_path);

    let client = match UnixDatagram::bind(&client_path) {
        Ok(socket) => socket,
        Err(error) => {
            return format!("error: failed to bind {client_path}: {error}\n");
        }
    };

    if client.connect(&admin_path).is_err() {
        let _ = std::fs::remove_file(&client_path);
        return format!("error: cannot connect to {admin_path}\n");
    }

    if client.send(command.as_bytes()).is_err() {
        let _ = std::fs::remove_file(&client_path);
        return "error: failed to send command\n".to_string();
    }

    let mut buffer = [0u8; 65536];
    let result = match client.recv(&mut buffer) {
        Ok(length) => String::from_utf8_lossy(&buffer[..length]).to_string(),
        Err(error) => format!("error: failed to receive response: {error}\n"),
    };

    let _ = std::fs::remove_file(&client_path);
    result
}

fn print_usage() {
    eprintln!("usage: tric <command> [args...]");
    eprintln!("       tric status");
    eprintln!("       tric keys [-p prefix]");
    eprintln!("       tric inspect <key>");
    eprintln!("       tric dump -f <path>");
    eprintln!("       tric restore -f <path>");
    eprintln!("       tric reload");
    eprintln!("       tric shutdown");
    eprintln!("       tric shell");
    eprintln!("       tric help");
}
