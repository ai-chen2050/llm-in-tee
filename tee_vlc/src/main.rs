use tee_vlc::nitro_clock::NitroEnclavesClock;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    NitroEnclavesClock::run(5055).await
}

// use tee_vlc::nitro_clock::NitroSecureModule;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     NitroSecureModule::run().await
// }
