// Copyright 2025 Vivian Voss. Licensed under the Apache License, Version 2.0.
// SPDX-License-Identifier: Apache-2.0
// Scope: Entry point for tric-server — creates QNX Core, registers modules, starts supervision.

mod core;
mod modules;

use std::sync::Arc;

use crate::core::create_core;
use crate::core::data_bus::{create_tric_bus, DataBus};
use crate::modules::placeholder::PlaceholderModule;

fn main() {
    let data_bus: Arc<dyn DataBus> = Arc::new(create_tric_bus());
    let mut core = create_core(data_bus);
    core.register_module(|| Box::new(PlaceholderModule));
    core.run_supervision_loop();
}
