#[derive(Clone, Debug)]
pub struct DecodeUtf8<I: Iterator<Item = u8>>(std::iter::Peekable<I>);

pub fn decode_utf8<I: IntoIterator<Item = u8>>(i: I) -> DecodeUtf8<I::IntoIter> {
    DecodeUtf8(i.into_iter().peekable())
}

#[derive(PartialEq, Debug)]
pub struct InvalidSequence(pub Vec<u8>);

impl<I: Iterator<Item = u8>> Iterator for DecodeUtf8<I> {
    type Item = Result<char, InvalidSequence>;
    #[inline]
    fn next(&mut self) -> Option<Result<char, InvalidSequence>> {
        let mut on_err: Vec<u8> = Vec::new();
        self.0.next().map(|b| {
            on_err.push(b);
            if b & 0x80 == 0 {
                Ok(b as char)
            } else {
                let l = (!b).leading_zeros() as usize; // number of bytes in UTF-8 representation
                if !(2..=6).contains(&l) {
                    return Err(InvalidSequence(on_err));
                };
                let mut x = (b as u32) & (0x7F >> l);
                for _ in 0..l - 1 {
                    match self.0.peek() {
                        Some(&b) if b & 0xC0 == 0x80 => {
                            on_err.push(b);
                            self.0.next();
                            x = (x << 6) | (b as u32) & 0x3F;
                        }
                        _ => return Err(InvalidSequence(on_err)),
                    }
                }
                match char::from_u32(x) {
                    Some(x) if l == x.len_utf8() => Ok(x),
                    _ => Err(InvalidSequence(on_err)),
                }
            }
        })
    }
}
