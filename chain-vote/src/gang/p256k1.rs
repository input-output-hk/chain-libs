use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;
use eccoxide::curve::sec2::p256k1::{FieldElement, Point, PointAffine, Scalar as IScalar};
use eccoxide::curve::{Sign as ISign, Sign::Positive};
use rand_core::{CryptoRng, RngCore};
use std::hash::{Hash, Hasher};
use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar(IScalar);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupElement(Point);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate(FieldElement);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sign(ISign);

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for GroupElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.to_bytes())
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Scalar {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.to_bytes())
    }
}

impl Coordinate {
    pub const BYTES_LEN: usize = FieldElement::SIZE_BYTES;

    pub fn to_bytes(&self) -> [u8; Self::BYTES_LEN] {
        self.0.to_bytes()
    }

    pub fn from_bytes(input: &[u8]) -> Option<Self> {
        if input.len() < Self::BYTES_LEN {
            None
        } else {
            Some(Coordinate(FieldElement::from_slice(
                &input[..Self::BYTES_LEN],
            )?))
        }
    }
}

impl GroupElement {
    /// Size of the byte representation of `GroupElement`.
    pub const BYTES_LEN: usize = 65;

    /// Serialized GroupElement::zero
    const BYTES_ZERO: [u8; Self::BYTES_LEN] = [0; Self::BYTES_LEN];

    /// Point from hash
    pub fn from_hash(buffer: &[u8]) -> Self {
        let mut result = [0u8; 32];
        let mut hash = Blake2b::new(32);
        hash.input(buffer);
        let mut i = 0u32;
        loop {
            hash.result(&mut result);

            if let Some(point) = Self::from_x_bytes(&result) {
                break point;
            }
            hash.input(&i.to_be_bytes());
            i += 1;
        }
    }

    fn from_x_bytes(bytes: &[u8]) -> Option<Self> {
        let x_coord = Coordinate::from_bytes(bytes)?;
        Self::decompress(&x_coord, Sign(Positive))
    }

    pub fn decompress(coord: &Coordinate, sign: Sign) -> Option<Self> {
        Some(GroupElement(Point::from_affine(&PointAffine::decompress(
            &coord.0, sign.0,
        )?)))
    }

    pub fn generator() -> Self {
        GroupElement(Point::generator())
    }

    pub fn zero() -> Self {
        GroupElement(Point::infinity())
    }

    pub fn normalize(&mut self) {
        self.0.normalize()
    }

    pub(super) fn compress(&self) -> Option<(Coordinate, Sign)> {
        self.0.to_affine().map(|p| {
            let (x, sign) = p.compress();
            (Coordinate(x.clone()), Sign(sign))
        })
    }

    pub fn to_bytes(&self) -> [u8; Self::BYTES_LEN] {
        match self.0.to_affine() {
            None => Self::BYTES_ZERO,
            Some(pa) => {
                let mut bytes = [0u8; Self::BYTES_LEN];
                let (x, y) = pa.to_coordinate();
                bytes[0] = 0x4;
                x.to_slice(&mut bytes[1..33]);
                y.to_slice(&mut bytes[33..65]);
                bytes
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes[0] == 0x4 {
            let x = FieldElement::from_slice(&bytes[1..33])?;
            let y = FieldElement::from_slice(&bytes[33..65])?;
            let p = PointAffine::from_coordinate(&x, &y)?;
            Some(GroupElement(Point::from_affine(&p)))
        } else if bytes == Self::BYTES_ZERO {
            Some(Self::zero())
        } else {
            None
        }
    }

    pub fn sum<'a, I>(i: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        let mut sum = GroupElement::zero();
        for v in i {
            sum = sum + v;
        }
        sum
    }
}

impl Scalar {
    pub const BYTES_LEN: usize = 32;

    /// additive identity
    pub fn zero() -> Self {
        Scalar(IScalar::zero())
    }

    /// multiplicative identity
    pub fn one() -> Self {
        Scalar(IScalar::one())
    }

    pub fn negate(&self) -> Self {
        Scalar(-&self.0)
    }

    /// multiplicative inverse
    pub fn inverse(&self) -> Scalar {
        Scalar(self.0.inverse())
    }

    /// Increment a
    pub fn increment(&mut self) {
        self.0 = &self.0 + IScalar::one()
    }

    pub fn to_bytes(&self) -> [u8; Self::BYTES_LEN] {
        self.0.to_bytes()
    }

    pub fn from_bytes(slice: &[u8]) -> Option<Self> {
        IScalar::from_slice(slice).map(Scalar)
    }

    pub fn random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let mut r = [0u8; 32];
        loop {
            rng.fill_bytes(&mut r[..]);

            if let Some(s) = IScalar::from_bytes(&r) {
                break (Scalar(s));
            }
        }
    }

    pub fn from_u64(v: u64) -> Self {
        Scalar(IScalar::from_u64(v))
    }

    pub fn power(&self, n: usize) -> Self {
        Self(self.0.power_u64(n as u64))
    }

    pub fn sum<I>(mut i: I) -> Option<Self>
    where
        I: Iterator<Item = Self>,
    {
        let mut sum = i.next()?;
        for v in i {
            sum = &sum + &v;
        }
        Some(sum)
    }
}

impl From<bool> for Scalar {
    fn from(b: bool) -> Self {
        if b {
            Scalar::one()
        } else {
            Scalar::zero()
        }
    }
}
/*
impl From<usize> for Scalar {
    fn from(sz: usize) -> Self {
        todo!()
    }
}
*/

macro_rules! lref {
    ($lty: ident, $class: ident, $rty: ident, $out: ident, $f: ident) => {
        impl<'a> $class<$rty> for &'a $lty {
            type Output = $out;

            fn $f(self, other: $rty) -> Self::Output {
                self.$f(&other)
            }
        }
    };
}

macro_rules! rref {
    ($lty: ident, $class: ident, $rty: ident, $out: ident, $f: ident) => {
        impl<'b> $class<&'b $rty> for $lty {
            type Output = $out;

            fn $f(self, other: &'b $rty) -> Self::Output {
                (&self).$f(other)
            }
        }
    };
}

macro_rules! nref {
    ($lty: ident, $class: ident, $rty: ident, $out: ident, $f: ident) => {
        impl $class<$rty> for $lty {
            type Output = $out;

            fn $f(self, other: $rty) -> Self::Output {
                (&self).$f(&other)
            }
        }
    };
}

//////////
// FE + FE
//////////

impl<'a, 'b> Add<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    fn add(self, other: &'b Scalar) -> Scalar {
        Scalar(&self.0 + &other.0)
    }
}

lref!(Scalar, Add, Scalar, Scalar, add);
rref!(Scalar, Add, Scalar, Scalar, add);
nref!(Scalar, Add, Scalar, Scalar, add);

//////////
// FE - FE
//////////

impl<'a, 'b> Sub<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    fn sub(self, other: &'b Scalar) -> Scalar {
        Scalar(&self.0 - &other.0)
    }
}

lref!(Scalar, Sub, Scalar, Scalar, sub);
rref!(Scalar, Sub, Scalar, Scalar, sub);
nref!(Scalar, Sub, Scalar, Scalar, sub);

//////////
// FE * FE
//////////

impl<'a, 'b> Mul<&'b Scalar> for &'a Scalar {
    type Output = Scalar;

    fn mul(self, other: &'b Scalar) -> Scalar {
        Scalar(&self.0 * &other.0)
    }
}

lref!(Scalar, Mul, Scalar, Scalar, mul);
rref!(Scalar, Mul, Scalar, Scalar, mul);
nref!(Scalar, Mul, Scalar, Scalar, mul);

//////////
// FE * GE
//////////

impl<'a, 'b> Mul<&'b GroupElement> for &'a Scalar {
    type Output = GroupElement;

    fn mul(self, other: &'b GroupElement) -> GroupElement {
        GroupElement(&other.0 * &self.0)
    }
}

impl<'a, 'b> Mul<&'b Scalar> for &'a GroupElement {
    type Output = GroupElement;

    fn mul(self, other: &'b Scalar) -> GroupElement {
        GroupElement(&other.0 * &self.0)
    }
}

lref!(Scalar, Mul, GroupElement, GroupElement, mul);
rref!(Scalar, Mul, GroupElement, GroupElement, mul);
nref!(Scalar, Mul, GroupElement, GroupElement, mul);

lref!(GroupElement, Mul, Scalar, GroupElement, mul);
rref!(GroupElement, Mul, Scalar, GroupElement, mul);
nref!(GroupElement, Mul, Scalar, GroupElement, mul);

//////////
// u64 * GE
//////////

impl<'a> Mul<&'a GroupElement> for u64 {
    type Output = GroupElement;

    fn mul(self, other: &'a GroupElement) -> GroupElement {
        GroupElement(&other.0 * self)
    }
}

impl<'a> Mul<u64> for &'a GroupElement {
    type Output = GroupElement;

    fn mul(self, other: u64) -> GroupElement {
        GroupElement(other * &self.0)
    }
}

//////////
// GE + GE
//////////

impl<'a, 'b> Add<&'b GroupElement> for &'a GroupElement {
    type Output = GroupElement;

    fn add(self, other: &'b GroupElement) -> GroupElement {
        GroupElement(&self.0 + &other.0)
    }
}

lref!(GroupElement, Add, GroupElement, GroupElement, add);
rref!(GroupElement, Add, GroupElement, GroupElement, add);
nref!(GroupElement, Add, GroupElement, GroupElement, add);

//////////
// GE - GE
//////////

impl<'a, 'b> Sub<&'b GroupElement> for &'a GroupElement {
    type Output = GroupElement;

    fn sub(self, other: &'b GroupElement) -> GroupElement {
        GroupElement(&self.0 + (-&other.0))
    }
}

lref!(GroupElement, Sub, GroupElement, GroupElement, sub);
rref!(GroupElement, Sub, GroupElement, GroupElement, sub);
nref!(GroupElement, Sub, GroupElement, GroupElement, sub);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn from_hash() {
        let element = GroupElement::from_hash(&mut [1u8]);

        let element2 = GroupElement::from_bytes(&[
            4, 238, 21, 90, 206, 156, 64, 41, 32, 116, 203, 106, 255, 140, 156, 205, 210, 115, 200,
            22, 72, 255, 17, 73, 239, 54, 188, 234, 110, 187, 138, 62, 37, 118, 220, 23, 189, 207,
            152, 182, 86, 240, 37, 233, 17, 111, 86, 215, 174, 77, 96, 212, 83, 190, 44, 148, 208,
            206, 163, 225, 224, 89, 1, 63, 152,
        ])
        .expect("This point is on the curve");

        assert_eq!(element, element2)
    }
}
