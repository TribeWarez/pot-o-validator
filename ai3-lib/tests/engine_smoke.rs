use ai3_lib::{AI3Engine, MiningTask, Tensor, TensorData, TensorEngine, TensorShape};

#[test]
fn engine_trait_executes_task() {
    let engine = AI3Engine::new();
    let input =
        Tensor::new(TensorShape::new(vec![1, 1]), TensorData::F32(vec![1.0_f32])).expect("tensor");
    let task = MiningTask::new(
        "relu".to_string(), // "identity" is not a registered operation; use relu
        vec![input],
        1,
        0,
        300,
        "test".to_string(),
    );

    let tensor = TensorEngine::execute_task(&engine, &task).expect("exec");
    assert_eq!(tensor.shape.dims, vec![1, 1]);
}
