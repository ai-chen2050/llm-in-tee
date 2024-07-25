use alloy::{
    hex::{self, ToHexExt},
    primitives::{keccak256, Address, Signature as Alloy_Signature, B256, U256},
    signers::{local::PrivateKeySigner, Signer, SignerSync},
};
// use const_hex::FromHex;
use std::str::FromStr;
use eyre::Result;
use rand::rngs::OsRng;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{Keypair, Message, PublicKey, Secp256k1, SecretKey, SECP256K1};

use crate::signature::Signature;

/// Recovers the address of the sender using secp256k1 pubkey recovery.
///
/// Converts the public key into an ethereum address by hashing the public key with keccak256.
///
/// This does not ensure that the `s` value in the signature is low, and _just_ wraps the
/// underlying secp256k1 library.
pub fn recover_signer_unchecked(
    sig: &[u8; 65],
    msg: &[u8; 32],
) -> Result<Address, secp256k1::Error> {
    let sig =
        RecoverableSignature::from_compact(&sig[0..64], RecoveryId::from_i32(sig[64] as i32)?)?;

    let public = SECP256K1.recover_ecdsa(&Message::from_digest(*msg), &sig)?;
    Ok(public_key_to_address(public))
}

/// Signs message with the given secret key.
/// Returns the corresponding signature.
pub fn sign_message(secret: [u8; 32], message: [u8; 32]) -> Result<Signature, secp256k1::Error> {
    let secret = B256::new(secret);
    let message = B256::new(message);
    let sec = SecretKey::from_slice(secret.as_ref())?;
    let s = SECP256K1.sign_ecdsa_recoverable(&Message::from_digest(message.0), &sec);
    let (rec_id, data) = s.serialize_compact();

    let signature: Signature = Signature {
        r: U256::try_from_be_slice(&data[..32]).expect("The slice has at most 32 bytes"),
        s: U256::try_from_be_slice(&data[32..64]).expect("The slice has at most 32 bytes"),
        odd_y_parity: rec_id.to_i32() != 0,
    };
    Ok(signature)
}

/// Signs message with the given secret key and chainid by alloy.
/// Returns the corresponding signature and signer address.
pub fn sign_message_with_chainid(secret: [u8; 32], raw_message: &str, chain_id: u64) -> anyhow::Result<(Alloy_Signature, String), anyhow::Error> {
    let secret = B256::new(secret);
    let signer = PrivateKeySigner::from_bytes(&secret).map_err(|_| secp256k1::Error::InvalidSecretKey)?;

    // Optionally, the wallet's chain id can be set, in order to use EIP-155
    // replay protection with different chains.
    let signer = signer.with_chain_id(Some(chain_id));

    // Sign the message asynchronously with the signer.
    let signature = signer.sign_message_sync(raw_message.as_bytes())?;

    // println!("Signature produced by {}: {:?}", signer.address(), signature);
    Ok((signature, signer.address().to_string()))
}

// recover signer with alloy lib
pub fn recover_signer_alloy(
    sig: String,
    raw_msg: &str,
) -> anyhow::Result<Address, anyhow::Error> {
    
    let sig = Alloy_Signature::from_str(&sig)?;

    let recovered = sig.recover_address_from_msg(raw_msg)?;
    Ok(recovered)
}

/// Converts a public key into an ethereum address by hashing the encoded public key with
/// keccak256.
pub fn public_key_to_address(public: PublicKey) -> Address {
    // strip out the first byte because that should be the SECP256K1_TAG_PUBKEY_UNCOMPRESSED
    // tag returned by libsecp's uncompressed pubkey serialization
    let hash = keccak256(&public.serialize_uncompressed()[1..]);
    Address::from_slice(&hash[12..])
}

pub fn generate_eth_account() -> ([u8; 32], String, String) {
    let secp = Secp256k1::new();
    let pair = Keypair::new(&secp, &mut OsRng);
    let secret_key_hex = pair.secret_key().secret_bytes();
    let public_key_hex = hex::encode(pair.public_key().serialize());
    let address = public_key_to_address(pair.public_key());
    (
        secret_key_hex,
        public_key_hex,
        address.encode_hex_with_prefix(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::hex;
    use common::crypto::core::DigestHash;
    use alloy::primitives::address;

    #[test]
    fn gen_account() {
        let rst = generate_eth_account();
        let pri_hex = hex::encode(rst.0);
        println!(
            "account: prikey:{}, pubkey:{}, address:{}",
            pri_hex, rst.1, rst.2
        );
    }

    #[test]
    fn recover_address() {
        use DigestHash as _;

        let rst = generate_eth_account();
        let pri_hex = hex::encode(rst.0);
        println!(
            "account: prikey:{}, pubkey:{}, address:{}",
            pri_hex, rst.1, rst.2
        );

        let message = "Hello, Ethereum!".to_owned();
        let msg = message.sha256().to_fixed_bytes();
        let sig = sign_message(rst.0, msg).unwrap();
        let sig_hex = sig.to_hex_bytes().to_string();
        println!("sig_hex: {}", sig_hex);
        
        let sig_bytes = hex::decode(sig_hex.clone()).expect("hex decode error");
        println!("len = {}", sig_bytes.len());
        let recover_sig = Signature::from_hex_str(sig_hex);
        let msg_fixed = B256::new(msg);
        let signer = recover_sig.recover_signer(msg_fixed);
        println!("signer: {:?}", signer);
    }

    #[test]
    fn recover_alloy_address() {
        let rst = generate_eth_account();
        let pri_hex = hex::encode(rst.0);
        println!(
            "account: prikey:{}, pubkey:{}, address:{}",
            pri_hex, rst.1, rst.2
        );

        let message = "Hello, Ethereum!";
        // chain_id = 1 , eth mainnet: 1
        let sig = sign_message_with_chainid(rst.0, message, 1).unwrap();
        println!("signature: {:?}, signer: {:?}", sig.0, sig.1);

        let sig_hex = hex::encode(sig.0.as_bytes());
        let address = recover_signer_alloy(sig_hex, &message).unwrap();
        assert_eq!(address.encode_hex_with_prefix(), rst.2);
    }
    
    #[test]
    fn sanity_ecrecover_call() {
        let sig = hex!("650acf9d3f5f0a2c799776a1254355d5f4061762a237396a99a0e0e3fc2bcd6729514a0dacb2e623ac4abd157cb18163ff942280db4d5caad66ddf941ba12e0300");
        let hash = hex!("47173285a8d7341e5e972fc677286384f802f8ef42a5ec5f03bbfa254cb01fad");
        let out = address!("c08b5542d177ac6686946920409741463a15dddb");

        assert_eq!(recover_signer_unchecked(&sig, &hash), Ok(out));
    }
}
