use crate::config::ConfigParam;
use chain_core::{
    mempack::{ReadBuf, ReadError},
    property::{Deserialize, Serialize, WriteError},
};

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
    fn deserialize(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        // FIXME: check canonical order?
        let len = buf.get_u16()?;
        let mut configs: Vec<ConfigParam> = Vec::with_capacity(len as usize);
        for _ in 0..len {
            configs.push(ConfigParam::deserialize(buf)?);
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
            let mut buf = ReadBuf::from(&bytes);
            let decoded = ConfigParams::deserialize(&mut buf).unwrap();

            params == decoded
        }

        fn config_params_serialize_readable(params: ConfigParams) -> bool {
            use chain_core::property::Serialize as _;
            let bytes = params.serialize_as_vec().unwrap();
            let mut reader = ReadBuf::from(&bytes);
            let decoded = ConfigParams::deserialize(&mut reader).unwrap();

            params == decoded
        }
    }
}
