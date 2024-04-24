#[derive(Debug)]
pub enum Device {
    CPU,
    CUDA,
    OPTIX,
    HIP,
    ONEAPI,
    METAL,
}
// append +CPU to gpu to include CPU into render cycle.
