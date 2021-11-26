use crate::config::ConfigParam;
use chain_core::property::{Deserialize, ReadError, Serialize, WriteError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigParams(pub(crate) Vec<ConfigParam>);

impl ConfigParams {
    pub fn new() -> Self {
        ConfigParams(Vec::new())
    }

    pub fn push(&mut self, config: ConfigParam) {
        self.0.push(config)
    }

    pub fn iter(&self) -> std::slice::Iter<ConfigParam> {
        self.0.iter()
    }
}

impl Serialize for ConfigParams {
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), WriteError> {
        // FIXME: put params in canonical order (e.g. sorted by tag)?
        use chain_core::packer::*;
        Codec::new(&mut writer).put_u16(self.0.len() as u16)?;
        for config in &self.0 {
            config.serialize(&mut writer)?
        }
        Ok(())
    }
}

impl Deserialize for ConfigParams {
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, ReadError> {
        use chain_core::packer::Codec;

        // FIXME: check canonical order?
        let mut codec = Codec::new(reader);
        let len = codec.get_u16()?;
        let mut configs: Vec<ConfigParam> = Vec::with_capacity(len as usize);
        for _ in 0..len {
            configs.push(ConfigParam::deserialize(&mut codec)?);
        }
        Ok(ConfigParams(configs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck! {
        fn config_params_serialize(params: ConfigParams) -> bool {
            use chain_core::property::{Serialize as _,};
            let bytes = params.serialize_as_vec().unwrap();
            let decoded = ConfigParams::deserialize(bytes.as_slice()).unwrap();

            params == decoded
        }

        fn config_params_serialize_readable(params: ConfigParams) -> bool {
            use chain_core::property::Serialize as _;
            let bytes = params.serialize_as_vec().unwrap();
            let decoded = ConfigParams::deserialize(bytes.as_slice()).unwrap();

            params == decoded
        }
    }
}
