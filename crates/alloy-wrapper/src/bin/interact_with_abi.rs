use alloy::{
    primitives::U256,
    providers::ProviderBuilder, sol,
};
use eyre::Result;

// Reference: https://docs.telos.net/build/clients/alloy/
// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    OperatorRangeManager,
    "../contracts/abi/operator_range_manager.json"
);

#[tokio::main]
async fn main() -> Result<()> {
    let endpoint = reqwest::Url::parse("https://rpc.holesky.ethpandaops.io")?;
    let provider = ProviderBuilder::new().on_http(endpoint);

    // Create a contract instance.
    let contract  = OperatorRangeManager::new(
        "0x0a24a30E5a8Ca9B790c7f57F0826159569e8dc4B".parse()?,
        provider,
    );

    let OperatorRangeManager::getNumOperatorsReturn { _0 } =
        contract.getNumOperators().call().await?;

    println!("Total operator numbers: {_0} ");

    let number_value = U256::from(1);
    let OperatorRangeManager::getOperatorsInRangeReturn { _0 } =
        contract.getOperatorsInRange(number_value).call().await?;

    println!("All aos operator is {:?}", _0);

    Ok(())
}
