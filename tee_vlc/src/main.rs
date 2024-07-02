use tee_vlc::nitro_clock::NitroEnclavesClock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    NitroEnclavesClock::run(5006).await
}