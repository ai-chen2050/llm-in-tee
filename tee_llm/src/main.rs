use std::io;
use std::io::Write;

use llama_cpp::standard_sampler::{SamplerStage, StandardSampler};
use llama_cpp::{
    CompletionHandle, EmbeddingsParams, LlamaModel, LlamaParams, SessionParams, TokensToStrings,
};

fn main() -> anyhow::Result<()> {
    let model_path = std::env::args().nth(1);

    // llama format
    let mut params = LlamaParams::default();

    // whether use gpu inference
    // params.n_gpu_layers = i32::MAX as u32;

    // Create a model from anything that implements `AsRef<Path>`:
    let model = LlamaModel::load_from_file(
        model_path
            .clone()
            .ok_or(anyhow::format_err!("missing model path"))?,
            params,
    )
    .expect("Could not load model");

    let session_params = SessionParams {
        n_ctx: 4096,
        n_batch: 2048,
        n_ubatch: 512,
        ..Default::default()
    };
    // A `LlamaModel` holds the weights shared across many _sessions_; while your model may be
    // several gigabytes large, a session is typically a few dozen to a hundred megabytes!
    let mut ctx = model
        .create_session(session_params)
        .expect("Failed to create session");

    // You can feed anything that implements `AsRef<[u8]>` into the model's context.
    ctx.advance_context("How to combine AI and blockchain?")
        .unwrap();

    // LLMs are typically used to predict the next word in a sequence. Let's generate some tokens!
    let max_tokens = 1024;
    let mut decoded_tokens = 0;

    let sampler_stages = vec![
        SamplerStage::RepetitionPenalty {
            repetition_penalty: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            last_n: 64,
        },
        SamplerStage::TopK(40),
        SamplerStage::TopP(0.95),
        SamplerStage::MinP(0.05),
        SamplerStage::Typical(1.0),
        SamplerStage::Temperature(0.0),
    ];

    let sampler = StandardSampler::new_mirostat_v2(sampler_stages, 0, 0.1, 5.0);
    // `ctx.start_completing_with` creates a worker thread that generates tokens. When the completion
    // handle is dropped, tokens stop generating!
    let completions = ctx
        .start_completing_with(sampler, 1024)?
        .into_strings();

    for completion in completions {
        print!("{completion}");
        let _ = io::stdout().flush();

        decoded_tokens += 1;

        if decoded_tokens > max_tokens {
            break;
        }
    }

    Ok(())
}
