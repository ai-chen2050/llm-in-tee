use llama_cpp_rs::{
    options::{ModelOptions, PredictOptions},
    LLama,
};

async fn main() -> anyhow::Result<()> {
    let model_path = std::env::args().nth(1)?;
    let model_options = ModelOptions::default();

    let llama = LLama::new(
        model_path.into(),
        &model_options,
    )
    .unwrap();

    let predict_options = PredictOptions {
        token_callback: Some(Box::new(|token| {
            println!("token1: {}", token);

            true
        })),
        ..Default::default()
    };

    llama
        .predict(
            "How to combine AI and blockchain?".into(),
             predict_options,
        )
        .unwrap();
}
