pub const BITS_IN_TYPE: usize = 3;

pub const TYPE_SPECIAL: u8 = 0;
pub const TYPE_PINT: u8 = 1;
pub const TYPE_NINT: u8 = 2;
pub const TYPE_BYTES: u8 = 3;
pub const TYPE_STR: u8 = 4;
pub const TYPE_ARR: u8 = 5;
pub const TYPE_MAP: u8 = 6;

pub const SPECIAL_NULL: u8 = (0 << BITS_IN_TYPE) | TYPE_SPECIAL;
pub const SPECIAL_FALSE: u8 = (1 << BITS_IN_TYPE) | TYPE_SPECIAL;
pub const SPECIAL_TRUE: u8 = (2 << BITS_IN_TYPE) | TYPE_SPECIAL;
pub const SPECIAL_ADDR: u8 = (3 << BITS_IN_TYPE) | TYPE_SPECIAL;
