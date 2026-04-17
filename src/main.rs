// Copyright 2025 Vivian Voss. Licensed under the Apache License, Version 2.0.
// SPDX-License-Identifier: Apache-2.0
// Scope: tric-server entry point — creates Core, registers modules, starts supervision.

use std::sync::Arc;

use tric::core::create_core;
use tric::core::data_bus::{create_tric_bus, DataBus};
use tric::modules::cli::{create_cli, CliConfig};
use tric::modules::metrics::create_metrics;
use tric::modules::server::{create_server, ServerConfig};

fn main() {
    let socket_dir =
        std::env::var("TRIC_SOCKET_DIR").unwrap_or_else(|_| "/var/run/tric".to_string());
    let udp_bind = std::env::var("TRIC_UDP_BIND").unwrap_or_else(|_| "0.0.0.0:7483".to_string());

    if let Err(error) = std::fs::create_dir_all(&socket_dir) {
        eprintln!("failed to create socket directory {socket_dir}: {error}");
        std::process::exit(1);
    }

    let local_path = format!("{socket_dir}/server.sock");
    let admin_path = format!("{socket_dir}/admin.sock");

    let data_bus: Arc<dyn DataBus> = Arc::new(create_tric_bus());
    let metrics = Arc::new(create_metrics());
    let mut core = create_core(data_bus);

    let metrics_for_server = Arc::clone(&metrics);
    let udp_bind_clone = udp_bind.clone();
    let local_path_clone = local_path.clone();
    core.register_module(move || {
        Box::new(create_server(
            ServerConfig {
                local_path: local_path_clone.clone(),
                udp_bind: udp_bind_clone.clone(),
                max_sessions: 10000,
            },
            Arc::clone(&metrics_for_server),
        ))
    });

    let metrics_for_cli = Arc::clone(&metrics);
    let admin_path_clone = admin_path.clone();
    core.register_module(move || {
        Box::new(create_cli(
            CliConfig {
                admin_path: admin_path_clone.clone(),
                auth_keys_path: None,
            },
            Arc::clone(&metrics_for_cli),
        ))
    });

    tric::modules::logger::log_info(&format!(
        "startup; local={local_path} udp={udp_bind} admin={admin_path}"
    ));

    core.run_supervision_loop();
}
