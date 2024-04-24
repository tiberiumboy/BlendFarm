#[derive(Debug)]
pub enum Mode {
    Frame(i32),
    Animation,
    Section(i32, i32),
}
