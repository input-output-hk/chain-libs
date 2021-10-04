use chain_core::mempack::{ReadBuf, Readable};
use chain_core::property::{Deserialize, Serialize};
use quickcheck::Arbitrary;
use std::fmt::Debug;

/// test that any arbitrary given object can serialize and deserialize
/// back into itself (i.e. it is a bijection,  or a one to one match
/// between the serialized bytes and the object)
pub fn serialization_bijection<T>(t: T)
where
    T: Arbitrary + Serialize + Deserialize + Eq + Debug,
{
    let vec = t.serialize_as_vec().unwrap();
    let decoded_t = T::deserialize(&vec[..]).unwrap();
    assert_eq!(decoded_t, t);
}

/// test that any arbitrary given object can serialize and deserialize
/// back into itself (i.e. it is a bijection,  or a one to one match
/// between the serialized bytes and the object)
pub fn serialization_bijection_r<T>(t: T)
where
    T: Arbitrary + Serialize + Readable + Eq + Debug,
{
    let vec = t.serialize_as_vec().unwrap();
    let mut buf = ReadBuf::from(&vec);
    let decoded_t = T::read(&mut buf).unwrap();
    assert!(buf.expect_end().is_ok() && decoded_t == t);
}
