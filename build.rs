use burn_onnx::{LoadStrategy, ModelGen};

fn main() {
    ModelGen::new()
        .input("src/model/mnist-8.onnx")
        .out_dir("model/")
        .load_strategy(LoadStrategy::Embedded)
        .run_from_script();
}
