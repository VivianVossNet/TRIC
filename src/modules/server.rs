// Copyright 2025 Vivian Voss. Licensed under the Apache License, Version 2.0.
// SPDX-License-Identifier: Apache-2.0
// Scope: Server module — binds UDS DGRAM socket, spawns worker threads, routes requests via DataBus.

use std::os::unix::net::UnixDatagram;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::core::data_bus::DataBus;
use crate::core::module::{Module, ModuleContext};
use crate::modules::codec::{decode_local, encode_local};
use crate::modules::router::dispatch_request;

const MAX_DATAGRAM: usize = 2048;
const ERROR_MALFORMED: u8 = 0xA1;

pub struct ServerConfig {
    pub local_path: String,
}

pub struct ServerModule {
    config: ServerConfig,
}

pub fn create_server(config: ServerConfig) -> ServerModule {
    ServerModule { config }
}

impl Module for ServerModule {
    fn name(&self) -> &'static str {
        "server"
    }

    fn run(&self, context: ModuleContext) {
        let _ = std::fs::remove_file(&self.config.local_path);
        let socket = Arc::new(UnixDatagram::bind(&self.config.local_path).unwrap_or_else(
            |error| {
                panic!(
                    "failed to bind local socket {}: {error}",
                    self.config.local_path
                )
            },
        ));

        let worker_count = thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(4);

        let core_bus = context.core_bus.clone();
        core_bus.write_value(b"module:server", b"running");
        core_bus.write_ttl(b"module:server", Duration::from_secs(15));

        let mut handles = Vec::with_capacity(worker_count);

        for _ in 0..worker_count {
            let socket = Arc::clone(&socket);
            let core_bus = context.core_bus.clone();
            let data_bus = Arc::clone(&context.data_bus);

            handles.push(thread::spawn(move || {
                run_worker_loop(&socket, &core_bus, &data_bus);
            }));
        }

        for handle in handles {
            let _ = handle.join();
        }
    }
}

fn run_worker_loop(socket: &UnixDatagram, core_bus: &crate::Tric, data_bus: &Arc<dyn DataBus>) {
    let mut buffer = [0u8; MAX_DATAGRAM];
    loop {
        core_bus.write_ttl(b"module:server", Duration::from_secs(15));

        let (length, peer) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(_) => continue,
        };

        let request = match decode_local(&buffer[..length]) {
            Some(request) => request,
            None => {
                let error = encode_local(&crate::modules::codec::Response {
                    request_id: 0,
                    opcode: ERROR_MALFORMED,
                    payload: Vec::new(),
                });
                let _ = socket.send_to_addr(&error, &peer);
                continue;
            }
        };

        let responses = dispatch_request(&request, data_bus);
        for response in &responses {
            let encoded = encode_local(response);
            let _ = socket.send_to_addr(&encoded, &peer);
        }
    }
}
