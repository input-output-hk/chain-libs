use ethereum_types::H256;
use evm::ExitError;

use secp256k1::{
    ecdsa::RecoverableSignature, rand::rngs::ThreadRng, KeyPair, Message, Secp256k1, SecretKey,
};

pub fn generate_keypair() -> KeyPair {
    let secp = Secp256k1::new();
    let mut rng = ThreadRng::default();
    KeyPair::new(&secp, &mut rng)
}

pub fn sign_transaction_hash(
    tx_hash: &H256,
    secret: &SecretKey,
) -> Result<RecoverableSignature, ExitError> {
    let s = Secp256k1::new();
    let h = Message::from_slice(tx_hash.as_fixed_bytes()).map_err(|_| ExitError::InvalidCode)?;
    Ok(s.sign_ecdsa_recoverable(&h, secret))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum::LegacyTransactionMessage;
    use ethereum_types::{H160, U256};
    use secp256k1::PublicKey;
    use std::str::FromStr;

    // chain-id's listed in https://eips.ethereum.org/EIPS/eip-155
    const TEST_CHAIN_ID: u64 = 1;

    #[test]
    fn test_transaction_signature_is_recoverable() {
        let keypair = generate_keypair();
        let unsigned_tx =
            crate::transaction::EthereumTransaction::Legacy(LegacyTransactionMessage {
                nonce: U256::zero(),
                gas_price: U256::zero(),
                gas_limit: U256::zero(),
                action: ethereum::TransactionAction::Create,
                value: U256::zero(),
                input: Vec::new(),
                chain_id: Some(TEST_CHAIN_ID),
            });
        let tx_hash = unsigned_tx.hash();
        let seckey = SecretKey::from_slice(&keypair.secret_bytes()).unwrap();
        let signature = sign_transaction_hash(&tx_hash, &seckey).unwrap();

        let msg = Message::from_slice(tx_hash.as_fixed_bytes())
            .map_err(|_| ExitError::InvalidCode)
            .unwrap();
        let pubkey = PublicKey::from_secret_key_global(&seckey);

        assert_eq!(signature.recover(&msg), Ok(pubkey))
    }

    #[test]
    fn test_legacy_transaction_signature() {
        // This test takes values fount at https://eips.ethereum.org/EIPS/eip-155#example
        let unsigned_tx =
            crate::transaction::EthereumTransaction::Legacy(LegacyTransactionMessage {
                nonce: U256::from(9_u64),
                gas_price: U256::from(20_u64 * 10_u64.pow(9)),
                gas_limit: U256::from(21_000_u64),
                action: ethereum::TransactionAction::Call(
                    H160::from_str("0x3535353535353535353535353535353535353535").unwrap(),
                ),
                value: U256::from(10u64.pow(18)),
                input: Vec::new(),
                chain_id: Some(TEST_CHAIN_ID),
            });

        // test signing data
        assert_eq!(
            hex::encode(unsigned_tx.to_bytes().as_slice()),
            "ec098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a764000080018080"
        );

        // test signing hash
        let tx_hash = unsigned_tx.hash();
        assert_eq!(
            format!("{:x}", tx_hash),
            "daf5a779ae972f972197303d7b574746c7ef83eadac0f2791ad23db92e4c8e53"
        );

        // given a secret key
        let seckey = SecretKey::from_slice(&[0x46; 32]).unwrap();
        let secret = H256::from_slice(&seckey.secret_bytes());

        // the transaction signature is
        let signature = sign_transaction_hash(&tx_hash, &seckey).unwrap();
        let (recovery_id, _signature_bytes) = signature.serialize_compact();
        assert_eq!(
            recovery_id.to_i32() as u64 % 2 + TEST_CHAIN_ID * 2 + 35,
            37u64
        );

        // test signed transaction
        let signed = unsigned_tx.sign(&secret);
        assert_eq!(
            hex::encode(signed.to_bytes().as_slice()),
            "f86c098504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83"
        );
    }
}
