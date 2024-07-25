use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    sol,
};
use eyre::Result;
use tracing::debug;

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    OperatorRangeManager,
    "../contracts/abi/operator_range_manager.json"
);

pub type OperatorRangeContract = OperatorRangeManager::OperatorRangeManagerInstance<
    alloy::transports::http::Http<reqwest::Client>,
    alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>>,
>;

pub fn new_vrf_range_backend(rpc: &str, address: &str) -> Result<OperatorRangeContract> {
    let endpoint = reqwest::Url::parse(rpc)?;
    let provider = ProviderBuilder::new().on_http(endpoint);

    // Create a contract instance.
    let contract = OperatorRangeManager::new(address.parse()?, provider);

    Ok(contract)
}

pub async fn get_num_operators(contract: OperatorRangeContract) -> Result<ruint::Uint<256, 4>> {
    let OperatorRangeManager::getNumOperatorsReturn { _0 } =
        contract.getNumOperators().call().await?;

    debug!("Total operator numbers: {_0} ");
    Ok(_0)
}

pub async fn get_operator_range_by_seed(
    contract: OperatorRangeContract,
    random_seed: u64,
) -> Result<Vec<Address>> {
    let number_value = U256::from(random_seed);
    let OperatorRangeManager::getOperatorsInRangeReturn { _0 } =
        contract.getOperatorsInRange(number_value).call().await?;

    debug!("All aos operator is {:?}", _0);
    Ok(_0)
}

pub async fn get_range_by_address(
    contract: OperatorRangeContract,
    query_addr: Address,
) -> Result<u64> {
    let OperatorRangeManager::operatorRangesReturn { start, end } =
        contract.operatorRanges(query_addr).call().await?;

    debug!("Operator {:?} ragnes: {:?}-{:?}", query_addr, start, end);
    let threshold = start - end;
    let th: u64 = threshold.try_into()?;
    Ok(th)
}





    