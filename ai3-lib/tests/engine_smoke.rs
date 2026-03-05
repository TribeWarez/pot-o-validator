use ai3_lib::{AI3Engine, MiningTask, Tensor, TensorData, TensorEngine, TensorShape};

#[test]
fn engine_trait_executes_task() {
    let engine = AI3Engine::new();
    let input = Tensor::new(
        TensorShape::new(vec![1, 1]),
        TensorData::from_vec(vec![1.0_f32]),
    )
    .expect("tensor");
    let task = MiningTask {
        operation_type: "identity".to_string(),
        input_tensors: vec![input],
        metadata: Default::default(),
    };

    let tensor = TensorEngine::execute_task(&engine, &task).expect("exec");
    assert_eq!(tensor.shape().dims(), &[1, 1]);
}
