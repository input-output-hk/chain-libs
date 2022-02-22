//! EVM precompiles based on the aurora-engine project.
#![allow(dead_code)]
pub mod blake2;
pub mod bn128;
pub mod hash;
pub mod identity;
pub mod modexp;
pub mod native;
mod prelude;
pub mod secp256k1;
#[cfg(test)]
mod utils;

use self::blake2::Blake2F;
use self::bn128::{Bn128Add, Bn128Mul, Bn128Pair};
use self::hash::{RIPEMD160, SHA256};
use self::identity::Identity;
use self::modexp::ModExp;
use self::native::{ExitToEthereum, ExitToNear};
use self::secp256k1::ECRecover;
use evm::backend::Log;
use evm::executor::stack;
use evm::{Context, ExitError, ExitSucceed};
use sha3::{Digest, Keccak256};

#[derive(Debug, Default)]
pub struct PrecompileOutput {
    pub cost: u64,
    pub output: prelude::Vec<u8>,
    pub logs: prelude::Vec<Log>,
}

impl PrecompileOutput {
    pub fn without_logs(cost: u64, output: prelude::Vec<u8>) -> Self {
        Self {
            cost,
            output,
            logs: prelude::Vec::new(),
        }
    }
}

impl From<PrecompileOutput> for stack::PrecompileOutput {
    fn from(output: PrecompileOutput) -> Self {
        stack::PrecompileOutput {
            exit_status: ExitSucceed::Returned,
            cost: output.cost,
            output: output.output,
            logs: output.logs,
        }
    }
}

type EvmPrecompileResult = Result<stack::PrecompileOutput, ExitError>;

/// A precompiled function for use in the EVM.
pub trait Precompile {
    /// The required gas in order to run the precompile function.
    fn required_gas(input: &[u8]) -> Result<u64, ExitError>;

    /// Runs the precompile function.
    fn run(
        input: &[u8],
        target_gas: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> EvmPrecompileResult;
}

type PrecompileFn = fn(&[u8], Option<u64>, &Context, bool) -> EvmPrecompileResult;

pub struct Precompiles(pub prelude::BTreeMap<prelude::Address, PrecompileFn>);

impl Precompiles {
    #[allow(dead_code)]
    pub fn new_homestead() -> Self {
        let addresses = prelude::vec![
            ECRecover::ADDRESS,
            SHA256::ADDRESS,
            RIPEMD160::ADDRESS,
            ExitToNear::ADDRESS,
            ExitToEthereum::ADDRESS,
        ];
        let fun: prelude::Vec<PrecompileFn> = prelude::vec![
            ECRecover::run,
            SHA256::run,
            RIPEMD160::run,
            ExitToNear::run,
            ExitToEthereum::run,
        ];
        let map = addresses.into_iter().zip(fun).collect();

        Precompiles(map)
    }

    #[allow(dead_code)]
    pub fn new_byzantium() -> Self {
        let addresses = prelude::vec![
            ECRecover::ADDRESS,
            SHA256::ADDRESS,
            RIPEMD160::ADDRESS,
            Identity::ADDRESS,
            ModExp::ADDRESS,
            Bn128Add::ADDRESS,
            Bn128Mul::ADDRESS,
            Bn128Pair::ADDRESS,
            ExitToNear::ADDRESS,
            ExitToEthereum::ADDRESS,
        ];
        let fun: prelude::Vec<PrecompileFn> = prelude::vec![
            ECRecover::run,
            SHA256::run,
            RIPEMD160::run,
            Identity::run,
            ModExp::run,
            Bn128Add::run,
            Bn128Mul::run,
            Bn128Pair::run,
            ExitToNear::run,
            ExitToEthereum::run,
        ];
        let mut map = prelude::BTreeMap::new();
        for (address, fun) in addresses.into_iter().zip(fun) {
            map.insert(address, fun);
        }

        Precompiles(map)
    }

    pub fn new_istanbul() -> Self {
        let addresses = prelude::vec![
            ECRecover::ADDRESS,
            SHA256::ADDRESS,
            RIPEMD160::ADDRESS,
            Identity::ADDRESS,
            ModExp::ADDRESS,
            Bn128Add::ADDRESS,
            Bn128Mul::ADDRESS,
            Bn128Pair::ADDRESS,
            Blake2F::ADDRESS,
            ExitToNear::ADDRESS,
            ExitToEthereum::ADDRESS,
        ];
        let fun: prelude::Vec<PrecompileFn> = prelude::vec![
            ECRecover::run,
            SHA256::run,
            RIPEMD160::run,
            Identity::run,
            ModExp::run,
            Bn128Add::run,
            Bn128Mul::run,
            Bn128Pair::run,
            Blake2F::run,
            ExitToNear::run,
            ExitToEthereum::run,
        ];
        let mut map = prelude::BTreeMap::new();
        for (address, fun) in addresses.into_iter().zip(fun) {
            map.insert(address, fun);
        }

        Precompiles(map)
    }

    #[allow(dead_code)]
    pub fn new_berlin() -> Self {
        Self::new_istanbul()
    }
}

impl stack::PrecompileSet for Precompiles {
    fn execute(
        &self,
        address: prelude::Address,
        input: &[u8],
        gas_limit: Option<u64>,
        context: &Context,
        is_static: bool,
    ) -> Option<Result<stack::PrecompileOutput, stack::PrecompileFailure>> {
        if let Some(precompile) = self.0.get(&address) {
            match precompile(input, gas_limit, context, is_static)
                .map_err(|exit_status| stack::PrecompileFailure::Error { exit_status })
            {
                Ok(output) => Some(Ok(output)),
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }
    fn is_precompile(&self, address: prelude::Address) -> bool {
        self.0.contains_key(&address)
    }
}

/// const fn for making an address by concatenating the bytes from two given numbers,
/// Note that 32 + 128 = 160 = 20 bytes (the length of an address). This function is used
/// as a convenience for specifying the addresses of the various precompiles.
pub const fn make_address(x: u32, y: u128) -> prelude::Address {
    let x_bytes = x.to_be_bytes();
    let y_bytes = y.to_be_bytes();
    prelude::H160([
        x_bytes[0],
        x_bytes[1],
        x_bytes[2],
        x_bytes[3],
        y_bytes[0],
        y_bytes[1],
        y_bytes[2],
        y_bytes[3],
        y_bytes[4],
        y_bytes[5],
        y_bytes[6],
        y_bytes[7],
        y_bytes[8],
        y_bytes[9],
        y_bytes[10],
        y_bytes[11],
        y_bytes[12],
        y_bytes[13],
        y_bytes[14],
        y_bytes[15],
    ])
}

const fn make_h256(x: u128, y: u128) -> prelude::H256 {
    let x_bytes = x.to_be_bytes();
    let y_bytes = y.to_be_bytes();
    prelude::H256([
        x_bytes[0],
        x_bytes[1],
        x_bytes[2],
        x_bytes[3],
        x_bytes[4],
        x_bytes[5],
        x_bytes[6],
        x_bytes[7],
        x_bytes[8],
        x_bytes[9],
        x_bytes[10],
        x_bytes[11],
        x_bytes[12],
        x_bytes[13],
        x_bytes[14],
        x_bytes[15],
        y_bytes[0],
        y_bytes[1],
        y_bytes[2],
        y_bytes[3],
        y_bytes[4],
        y_bytes[5],
        y_bytes[6],
        y_bytes[7],
        y_bytes[8],
        y_bytes[9],
        y_bytes[10],
        y_bytes[11],
        y_bytes[12],
        y_bytes[13],
        y_bytes[14],
        y_bytes[15],
    ])
}

#[inline]
pub fn keccak(data: &[u8]) -> prelude::H256 {
    prelude::H256::from_slice(Keccak256::digest(data).as_slice())
}

#[cfg(test)]
mod tests {
    use super::prelude;
    use rand::Rng;

    #[test]
    fn test_precompile_addresses() {
        assert_eq!(super::secp256k1::ECRecover::ADDRESS, u8_to_address(1));
        assert_eq!(super::hash::SHA256::ADDRESS, u8_to_address(2));
        assert_eq!(super::hash::RIPEMD160::ADDRESS, u8_to_address(3));
        assert_eq!(super::identity::Identity::ADDRESS, u8_to_address(4));
        assert_eq!(super::ModExp::ADDRESS, u8_to_address(5));
        assert_eq!(super::Bn128Add::ADDRESS, u8_to_address(6));
        assert_eq!(super::Bn128Mul::ADDRESS, u8_to_address(7));
        assert_eq!(super::Bn128Pair::ADDRESS, u8_to_address(8));
        assert_eq!(super::blake2::Blake2F::ADDRESS, u8_to_address(9));
    }

    #[test]
    fn test_make_address() {
        for i in 0..u8::MAX {
            assert_eq!(super::make_address(0, i as u128), u8_to_address(i));
        }

        let mut rng = rand::thread_rng();
        for _ in 0..u8::MAX {
            let address = rng.gen::<[u8; 20]>().into();
            let (x, y) = split_address(address);
            assert_eq!(address, super::make_address(x, y))
        }
    }

    fn u8_to_address(x: u8) -> prelude::Address {
        let mut bytes = [0u8; 20];
        bytes[19] = x;
        bytes.into()
    }

    // Inverse function of `super::make_address`.
    fn split_address(a: prelude::Address) -> (u32, u128) {
        let mut x_bytes = [0u8; 4];
        let mut y_bytes = [0u8; 16];

        x_bytes.copy_from_slice(&a[0..4]);
        y_bytes.copy_from_slice(&a[4..20]);

        (u32::from_be_bytes(x_bytes), u128::from_be_bytes(y_bytes))
    }
}
