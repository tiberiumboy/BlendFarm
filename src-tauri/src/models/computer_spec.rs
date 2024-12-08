use machine_info::Machine;
use serde::{Deserialize, Serialize};
use std::env::consts;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComputerSpec {
    host: String,
    os: String,
    arch: String,
    memory: u64,
    gpu: Option<String>,
    cpu: String,
    cores: usize,
}

impl ComputerSpec {
    pub fn new(machine: &mut Machine) -> Self {
        let sys_info = machine.system_info();
        let memory = &sys_info.memory;
        let host = &sys_info.hostname;
        let gpu = &sys_info
            .graphics
            .first()
            .and_then(|v| Some(v.name.to_owned()));
        let cores = &sys_info.total_processors;

        Self {
            host: host.to_owned(),
            os: consts::OS.to_owned(),
            arch: consts::ARCH.to_owned(),
            memory: memory.to_owned(),
            gpu: gpu.to_owned(),
            cpu: sys_info.processor.brand.to_owned(),
            cores: cores.to_owned(),
        }
    }
}
