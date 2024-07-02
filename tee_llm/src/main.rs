use tee_llm::nitro_llm::NitroEnclavesLlm;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    NitroEnclavesLlm::run(5005).await
}