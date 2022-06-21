//! Wrappers that handle custom serialization

use std::fmt;

use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy)]
pub enum Error {
    /// only b'0'-b'9' and b'a'-b'f' is allowed
    InvalidChar,
    /// Should only happen in development with wrong constants
    InvalidConsts,
    /// String length was wrong
    InvalidLen,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

use Error::*;

/// Fast fixed size hex string conversion. This is from `u64_array_bigints` but with fixed size and no reversal except at nibble level
fn from_hex_str_fast<const N: usize, const N2: usize>(src: &[u8]) -> Result<[u64; N], Error> {
    // Using SWAR techniques to process 8 hex chars at a time.
    let swar = |x: u64| -> Result<u32, Error> {
        // this seems like a lot, but the overflow branches here are rarely taken and
        // most operations can be done in parallel

        const MSBS: u64 = 0x8080_8080_8080_8080;
        // get the msb out of the way so that later carries cannot propogate between
        // byte boundaries
        if (x & MSBS) != 0 {
            return Err(InvalidChar);
        }
        // add -(b'f' + 1)u7 (ASCII z no 8th bit)
        let gt_f = x.wrapping_add(0x1919_1919_1919_1919) & MSBS;
        if gt_f != 0 {
            // overflow in at least one of the bytes, there was a char above b'f'
            return Err(InvalidChar);
        }
        // add -b'0'u7
        let ge_0 = x.wrapping_add(0x5050_5050_5050_5050) & MSBS;
        let lt_0 = ge_0 ^ MSBS;
        if lt_0 != 0 {
            return Err(InvalidChar);
        }

        // now all bytes are in the range b'0'..=b'f', but need to remove two more
        // ranges

        // add -b'a'u7
        let ge_a = x.wrapping_add(0x1f1f_1f1f_1f1f_1f1f) & MSBS;
        let lt_a = ge_a ^ MSBS;
        // add -(b'9' + 1)u7
        let ge_9 = x.wrapping_add(0x4646_4646_4646_4646) & MSBS;
        if (ge_9 & lt_a) != 0 {
            return Err(InvalidChar);
        }

        let ge_9_mask = (ge_9 >> 7).wrapping_mul(0xff);

        // add -(b'a')u7 + 10u7, and mask off carries
        let alphas =
            (x & ge_9_mask).wrapping_add(0x2929_2929_2929_2929 & ge_9_mask) & 0x0f0f_0f0f_0f0f_0f0f;
        // add -(b'0')u7 and mask off carries
        let nums = (x & !ge_9_mask).wrapping_add(0x5050_5050_5050_5050 & !ge_9_mask)
            & 0x0f0f_0f0f_0f0f_0f0f;
        let mut y = alphas | nums;

        // gather
        let mut y0 = y & 0x000f_000f_000f_000f;
        let mut y1 = y & 0x0f00_0f00_0f00_0f00;
        y = (y0 << 4) | (y1 >> 8);
        y0 = y & 0x0000_00ff_0000_00ff;
        y1 = y & 0x00ff_0000_00ff_0000;
        y = y0 | (y1 >> 8);
        y0 = y & 0x0000_0000_0000_ffff;
        y1 = y & 0x0000_ffff_0000_0000;
        y = y0 | (y1 >> 16);

        Ok(y as u32)
    };

    // avoid overflow possibility
    if ((N2 >> 1) != N) || ((N2 & 1) != 0) {
        return Err(InvalidConsts);
    }
    if ((src.len() >> 3) != N2) || ((src.len() & 0b111) != 0) {
        return Err(InvalidLen);
    }
    // if `src` has zero bytes they will overwrite and cause overflow errors in
    // `swar`
    let mut buf = [0x3030_3030_3030_3030u64; N2];
    let bytes_buf: &mut [u8] = bytemuck::try_cast_slice_mut(&mut buf).unwrap();
    bytes_buf.copy_from_slice(src);
    let mut res = [0u64; N];
    for i in 0..buf.len() {
        // fix for big endian, is a no-op on little endian architectures
        buf[i] = u64::from_le(buf[i]);
        let x = match swar(buf[i]) {
            Ok(x) => x,
            Err(e) => return Err(e),
        };
        res[(i / 2)] |= (x as u64) << if (i % 2) == 0 { 0 } else { 32 };
    }
    Ok(res)
}

/// Also from `u64_array_bigints`, fixed size and no reversal except at nibble level. Errors can only occur if `N*2 != N2` or `src.len()*8 != NN`.
fn to_hex_string_fast<const N: usize, const N2: usize>(src: &[u8]) -> Result<String, Error> {
    let swar = |x: u64| -> u64 {
        // Using SWAR techniques to process one u32 at a time.
        // First, scatter `x` evenly into groups of 4 bits.
        // 0x0000_0000_abcd_efgh
        // 0xefgh_0000_abcd_0000
        // 0x00gh_00ef_00cd_00ab
        // 0x0h0g_0f0e_0d0c_0b0a
        let mut x0: u64 = x & 0xffff;
        let mut x1: u64 = x & 0xffff_0000;
        let mut x: u64 = x0 | (x1 << 16);
        x0 = x & 0x0000_00ff_0000_00ff;
        x1 = x & 0x0000_ff00_0000_ff00;
        x = x0 | (x1 << 8);
        x0 = x & 0x000f_000f_000f_000f;
        x1 = x & 0x00f0_00f0_00f0_00f0;
        x = (x0 << 8) | (x1 >> 4);

        // because ASCII does not have letters immediately following numbers, we need to
        // differentiate between them to be able to add different offsets.

        // the two's complement of `10u4 == 0b1010u4` is `0b0110u4 == 0x6u4`.
        // get the carries, if there is a carry the 4 bits were above '9'
        let c = (x.wrapping_add(0x0606_0606_0606_0606) & 0x1010_1010_1010_1010) >> 4;

        // conditionally offset to b'a' or b'0'
        let offsets = c.wrapping_mul(0x57) | (c ^ 0x0101_0101_0101_0101).wrapping_mul(0x30);

        x.wrapping_add(offsets)
    };

    if ((N2 >> 1) != N) || ((N2 & 1) != 0) {
        return Err(InvalidConsts);
    }
    if ((src.len() >> 3) != N) || ((src.len() & 0b111) != 0) {
        return Err(InvalidLen);
    }
    let mut buf = [0u64; N];
    let bytes_buf: &mut [u8] = bytemuck::try_cast_slice_mut(&mut buf).unwrap();
    bytes_buf.copy_from_slice(src);
    let mut buf2 = [0u64; N2];
    for i in 0..buf.len() {
        // find first nonzero leading digit
        let x = buf[i];
        let lo = x as u32 as u64;
        let hi = (x >> 32) as u32 as u64;
        // The `from_le` fixes big endian architectures.
        buf2[(i << 1)] = u64::from_le(swar(lo));
        buf2[(i << 1) + 1] = u64::from_le(swar(hi));
    }
    let v: &[u8] = bytemuck::try_cast_slice(&buf2).unwrap();
    // TODO it might be possible to allocate as Vec<u64> and then raw_parts cast to Vec<u8>?
    let mut s = vec![0u8; N2 * 8];
    s.copy_from_slice(v);
    #[cfg(all(debug_assertions, not(miri)))]
    {
        Ok(String::from_utf8(s).unwrap())
    }
    #[cfg(any(not(debug_assertions), miri))]
    {
        // Safety: `to_hex_string_buffer` only set `buf[index..]` to b'0'-b'9' and
        // b'a'-b'f', and `s` is defaulted to have all b'0'.
        unsafe { Ok(String::from_utf8_unchecked(s)) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    //#[serde(deserialize_with = "de_signature")]
    pub signature: ed25519_dalek::Signature,
}

impl Signature {}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Signature", 1)?;
        s.serialize_field(
            "signature",
            &to_hex_string_fast::<8, 16>(&self.signature.to_bytes()).unwrap(),
        )?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Signature {
            signature: String,
        }
        let tmp = Signature::deserialize(deserializer)?;
        let array: [u64; 8] =
            from_hex_str_fast::<8, 16>(tmp.signature.as_bytes()).map_err(de::Error::custom)?;
        let signature = ed25519_dalek::Signature::from_bytes(bytemuck::bytes_of(&array))
            .map_err(de::Error::custom)?;
        Ok(Self { signature })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use old_rand_core::{CryptoRng as OldCryptoRng, RngCore as OldRngCore};
    use rand_xoshiro::{
        rand_core::{RngCore, SeedableRng},
        Xoshiro128StarStar,
    };

    /// Use only for deterministic testing not for production
    struct TestRng(Xoshiro128StarStar);
    impl OldCryptoRng for TestRng {}
    impl OldRngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            RngCore::next_u32(&mut self.0)
        }
        fn next_u64(&mut self) -> u64 {
            RngCore::next_u64(&mut self.0)
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            RngCore::fill_bytes(&mut self.0, dest)
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), old_rand_core::Error> {
            Ok(RngCore::try_fill_bytes(&mut self.0, dest).unwrap())
        }
    }
    impl TestRng {
        pub fn seed_from_u64(x: u64) -> Self {
            Self(Xoshiro128StarStar::seed_from_u64(x))
        }
    }

    #[test]
    fn serialization() {
        use ed25519_dalek::Signer;
        let mut rng = TestRng::seed_from_u64(0);
        let keypair = ed25519_dalek::Keypair::generate(&mut rng);
        let signature = keypair.sign(b"hello");
        let signature0 = Signature {
            signature: signature,
        };
        let s = serde_json::to_string(&signature0).unwrap();
        assert_eq!(s, "{\"signature\":\"6a8405500e2773d2c2c17edfec2f94d9200927f91e8db37d61244e8a5a9bd347e2935183a07e8f38dceef2b0663462ab2d9c1db402eac777c5abdbed29ba2d0f\"}");
        let signature1: Signature = serde_json::from_str(&s).unwrap();
        assert_eq!(signature0, signature1);
    }
}
