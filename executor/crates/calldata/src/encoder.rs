use crate::Address;
use crate::consts::*;

fn write_uleb<W: Writer>(w: &mut W, mut num: num_bigint::BigUint) -> Result<(), W::Error> {
    if num == num_bigint::BigUint::ZERO {
        return w.write_one(0);
    }

    loop {
        let mut cur = (num.iter_u32_digits().next().unwrap_or(0) & 0x7f) as u8;
        num >>= 7u32;
        let has_next = num != num_bigint::BigUint::ZERO;

        if has_next {
            cur |= 0x80;
        }

        w.write_one(cur)?;

        if !has_next {
            return Ok(());
        }
    }
}

fn write_uleb_u64<W: Writer>(w: &mut W, mut num: u64) -> Result<(), W::Error> {
    if num == 0 {
        return w.write_one(0);
    }

    loop {
        let mut cur = (num & 0x7f) as u8;
        num >>= 7;

        if num != 0 {
            cur |= 0x80;
        }

        w.write_one(cur)?;

        if num == 0 {
            return Ok(());
        }
    }
}

fn write_uleb_u128<W: Writer>(w: &mut W, mut num: u128) -> Result<(), W::Error> {
    if num == 0 {
        return w.write_one(0);
    }

    loop {
        let mut cur = (num & 0x7f) as u8;
        num >>= 7;

        if num != 0 {
            cur |= 0x80;
        }

        w.write_one(cur)?;

        if num == 0 {
            return Ok(());
        }
    }
}

fn write_uleb_tagged_u64<W: Writer>(w: &mut W, tag: u8, val: u64) -> Result<(), W::Error> {
    let val = val as u128;
    write_uleb_u128(w, (val << BITS_IN_TYPE as u128) + (tag as u128))
}

fn write_tagged_uleb<W: Writer>(
    w: &mut W,
    tag: u8,
    val: num_bigint::BigUint,
) -> Result<(), W::Error> {
    write_uleb(w, (val << BITS_IN_TYPE) + tag)
}

pub trait Writer {
    type Error;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error>;

    fn write_one(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.write_all(&[byte])
    }
}

pub struct StdWriter<W>(W)
where
    W: std::io::Write;

impl<W: std::io::Write> Writer for StdWriter<W> {
    type Error = std::io::Error;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(data)
    }
}

impl<W> From<W> for StdWriter<W>
where
    W: std::io::Write,
{
    fn from(writer: W) -> Self {
        Self(writer)
    }
}

impl<W> StdWriter<W>
where
    W: std::io::Write,
{
    pub fn into_inner(self) -> W {
        self.0
    }

    pub fn new(writer: W) -> Self {
        Self(writer)
    }
}

pub struct Encoder<W>(W)
where
    W: Writer;

impl<W: Writer> Encoder<W> {
    pub fn new(writer: W) -> Self {
        Self(writer)
    }

    pub fn into_inner(self) -> W {
        self.0
    }

    pub fn start_array(&mut self, len: u64) -> Result<(), W::Error> {
        write_uleb_tagged_u64(&mut self.0, TYPE_ARR, len)
    }

    pub fn start_array_big(&mut self, len: &num_bigint::BigInt) -> Result<(), W::Error> {
        write_tagged_uleb(
            &mut self.0,
            TYPE_ARR,
            len.to_biguint().expect("array length must be non-negative"),
        )
    }

    pub fn start_map(&mut self, len: u64) -> Result<(), W::Error> {
        write_uleb_tagged_u64(&mut self.0, TYPE_MAP, len)
    }

    pub fn start_map_big(&mut self, len: &num_bigint::BigInt) -> Result<(), W::Error> {
        write_tagged_uleb(
            &mut self.0,
            TYPE_MAP,
            len.to_biguint().expect("map length must be non-negative"),
        )
    }

    pub fn push_null(&mut self) -> Result<(), W::Error> {
        self.0.write_all(&[SPECIAL_NULL])
    }

    pub fn push_bool(&mut self, value: bool) -> Result<(), W::Error> {
        self.0
            .write_all(&[if value { SPECIAL_TRUE } else { SPECIAL_FALSE }])
    }

    pub fn push_i64(&mut self, value: i64) -> Result<(), W::Error> {
        if value < 0 {
            write_uleb_tagged_u64(&mut self.0, TYPE_NINT, -(value + 1) as u64)
        } else {
            write_uleb_tagged_u64(&mut self.0, TYPE_PINT, value as u64)
        }
    }

    pub fn push_u64(&mut self, value: u64) -> Result<(), W::Error> {
        write_uleb_tagged_u64(&mut self.0, TYPE_PINT, value)
    }

    pub fn push_bigint(&mut self, value: &num_bigint::BigInt) -> Result<(), W::Error> {
        if value.sign() == num_bigint::Sign::Minus {
            let mut mag = value.magnitude().clone();
            mag -= 1u32;
            write_tagged_uleb(&mut self.0, TYPE_NINT, mag)
        } else {
            write_tagged_uleb(&mut self.0, TYPE_PINT, value.magnitude().clone())
        }
    }

    pub fn push_str(&mut self, value: &str) -> Result<(), W::Error> {
        write_uleb_tagged_u64(&mut self.0, TYPE_STR, value.len() as u64)?;
        self.0.write_all(value.as_bytes())
    }

    pub fn push_bytes(&mut self, value: &[u8]) -> Result<(), W::Error> {
        write_uleb_tagged_u64(&mut self.0, TYPE_BYTES, value.len() as u64)?;
        self.0.write_all(value)
    }

    pub fn write_raw(&mut self, data: &[u8]) -> Result<(), W::Error> {
        self.0.write_all(data)
    }

    pub fn push_address(&mut self, addr: &Address) -> Result<(), W::Error> {
        self.0.write_all(&[SPECIAL_ADDR])?;
        self.0.write_all(&addr.0)
    }

    pub fn push_map_k(&mut self, key: &str) -> Result<(), W::Error> {
        write_uleb_u64(&mut self.0, key.len() as u64)?;
        self.0.write_all(key.as_bytes())
    }
}
