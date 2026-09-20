use super::computer_spec::ComputerSpec;

// Treat this struct as server found on network
#[derive(Debug)]
pub struct Worker {
    pub spec: ComputerSpec,
    // internally, we should at least documented the logs and entry.
    // logs: Vec<ServerEvent>,
}

impl Worker {
    pub fn new(spec: ComputerSpec) -> Self {
        Self {
            spec,
            /*logs: Vec::new()*/
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_new_worker_succeed() {
        let spec = ComputerSpec::new();
        let worker = Worker::new(spec.clone());
        assert_eq!(worker.spec, spec);
    }
}
