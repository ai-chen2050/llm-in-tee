use alloy::{
    primitives::{U256, address},
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

    // let register_addr = address!("02a5592a6de1568f6efdc536da3ef887f98414cb");
    // let OperatorRangeManager::registerOperatorReturn { } =
    //     contract.registerOperator(register_addr, U256::from(10), U256::from(10000)).call().await?;

    let OperatorRangeManager::getNumOperatorsReturn { _0 } =
        contract.getNumOperators().call().await?;

    println!("Total operator numbers: {_0} ");

    let number_value = U256::from(1);
    let OperatorRangeManager::getOperatorsInRangeReturn { _0 } =
        contract.getOperatorsInRange(number_value).call().await?;

    println!("All aos operator is {:?}", _0);

    let query_addr = address!("cbee70d449ac3421138fb21cccd156456958baa4");
    let OperatorRangeManager::operatorRangesReturn { start, end } =
        contract.operatorRanges(query_addr).call().await?;

    println!("Operator 0xcbee70d449ac3421138fb21cccd156456958baa4 ragnes: {:?}-{:?}", start, end);

    Ok(())
}
