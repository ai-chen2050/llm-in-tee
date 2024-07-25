use std::path::PathBuf;
use thiserror::Error;

pub struct ErrorCodes;

impl ErrorCodes {
    pub const PROCESS_EXIT: i32 = 42;

    pub const CONFIG_MISSING: u32 = 1001;
    pub const SERIALIZATION_ERROR: u32 = 1002;
    pub const IO_ERROR: u32 = 1003;
    pub const ILLEGAL_NODE_ID: u32 = 1004;
    pub const ILLEGAL_SIGNER: u32 = 1005;
    
    pub const API_FAIL_TO_JSON: u32 = 2001;

    pub const OP_CUSTOM_ERROR: u32 = 3001;
    pub const OP_FAIL_REGISTER: u32 = 3002;
    pub const OP_CONNECT_TEE_ERROR: u32 = 3003;
    pub const OP_SEND_PROMPT_ERROR: u32 = 3004;
    pub const OP_DECODE_SIGNER_KEY_ERROR: u32 = 3005;
    pub const OP_NEW_VRF_RANGE_CONTRACT_ERROR: u32 = 3006;
    pub const OP_GET_RANGE_CONTRACT_ERROR: u32 = 3007;
    
}

pub type OperatorConfigResult<T> = Result<T, OperatorConfigError>;

#[derive(Error, Debug)]
pub enum OperatorConfigError {
    #[error(
        "No operator config found at this path: {0} (Error Code: {})",
        ErrorCodes::CONFIG_MISSING
    )]
    ConfigMissing(PathBuf),

    #[error(
        "Config deserialization error: {0} (Error Code: {})",
        ErrorCodes::SERIALIZATION_ERROR
    )]
    SerializationError(#[from] serde_yaml::Error),

    #[error(
        "Error while performing IO for the Operator: {0} (Error Code: {})",
        ErrorCodes::IO_ERROR
    )]
    IoError(#[from] std::io::Error),

    #[error(
        "Error nodeid illegal, must be hex format, and 40 bits (Error Code: {})",
        ErrorCodes::ILLEGAL_NODE_ID
    )]
    IllegalNodeId,

    #[error(
        "Error signer illegal, must be hex format, and 64 bits (Error Code: {})",
        ErrorCodes::ILLEGAL_SIGNER
    )]
    IllegalSignerKey,
}

pub type OperatorAPIResult<T> = Result<T, OperatorAPIError>;

#[derive(Error, Debug)]
pub enum OperatorAPIError {
    #[error(
        "Error failed to serialize struct to JSON (Error Code: {})",
        ErrorCodes::API_FAIL_TO_JSON
    )]
    APIFailToJson,
}


pub type OperatorResult<T> = Result<T, OperatorError>;

#[derive(Error, Debug)]
pub enum OperatorError {
    #[error(
        "Error: some error happened, detail: {0} (Error Code: {})",
        ErrorCodes::OP_CUSTOM_ERROR
    )]
    CustomError(String),

    #[error(
        "Error: register to dispatcher failed, detail: {0} (Error Code: {})",
        ErrorCodes::OP_FAIL_REGISTER
    )]
    OPSetupRegister(#[from] reqwest::Error),

    #[error(
        "Error: connect to tee service failed, detail: {0}  (Error Code: {})",
        ErrorCodes::OP_CONNECT_TEE_ERROR
    )]
    OPConnectTEEError(String),

    #[error(
        "Error: send promtp to tee service failed, detail: {0}  (Error Code: {})",
        ErrorCodes::OP_SEND_PROMPT_ERROR
    )]
    OPSendPromptError(String),

    #[error(
        "Error: decode signer private key error failed, detail: {0}  (Error Code: {})",
        ErrorCodes::OP_DECODE_SIGNER_KEY_ERROR
    )]
    OPDecodeSignerKeyError(#[from] alloy_primitives::hex::FromHexError),

    #[error(
        "Error: new vrf range contract failed, detail: {0}  (Error Code: {})",
        ErrorCodes::OP_NEW_VRF_RANGE_CONTRACT_ERROR
    )]
    OPNewVrfRangeContractError(#[from] eyre::ErrReport),

    #[error(
        "Error: get vrf range contract failed, detail: {0}  (Error Code: {})",
        ErrorCodes::OP_GET_RANGE_CONTRACT_ERROR
    )]
    OPGetVrfRangeContractError(String),
}
