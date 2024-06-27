use tee_vlc::nitro_clock::NitroSecureModule;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    NitroSecureModule::run().await
}
