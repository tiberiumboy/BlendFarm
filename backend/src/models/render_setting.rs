#[allow(dead_code)]
enum RenderStrategy {
    SplitHorizontal,
    SplitVertical,
    Chunked,
    SplitChunekd,
}

#[allow(dead_code)]
enum TaskOrder {
    Default,
    Center,
}

#[allow(dead_code)]
struct RenderSetting {
    pub strategy: RenderStrategy,
    pub order: TaskOrder,
    pub frame: i32,
    pub scene: String,
    pub camera: String,
    pub fps: i32,
    pub output_width: i32,
    pub output_height: i32,
    pub samples: i32,
    // pub engine: EngineType,
}
