use chain_core::mempack::{Deserialize, ReadBuf};
use chain_core::property::Serialize;
use quickcheck::{Arbitrary, TestResult};

/// test that any arbitrary given object can serialize and deserialize
/// back into itself (i.e. it is a bijection,  or a one to one match
/// between the serialized bytes and the object)
pub fn serialization_bijection<T>(t: T) -> TestResult
where
    T: Arbitrary + Serialize + Deserialize + Eq,
{
    let vec = match t.serialize_as_vec() {
        Err(error) => return TestResult::error(format!("serialization: {}", error)),
        Ok(v) => v,
    };
    let mut buf = ReadBuf::from(&vec);
    let decoded_t = match T::deserialize(&mut buf) {
        Err(error) => {
            return TestResult::error(format!("deserialization: {:?}\n{}", error, buf.debug()))
        }
        Ok(v) => v,
    };
    TestResult::from_bool(buf.expect_end().is_ok() && decoded_t == t)
}
