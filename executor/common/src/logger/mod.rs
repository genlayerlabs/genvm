mod error;

use error::Error;
use serde::Serialize;

use std::{io::Write, str::FromStr};

use crate::calldata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => f.write_str("trace"),
            Self::Debug => f.write_str("debug"),
            Self::Info => f.write_str("info"),
            Self::Warn => f.write_str("warn"),
            Self::Error => f.write_str("error"),
        }
    }
}

impl FromStr for Level {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(()),
        }
    }
}

impl Level {
    pub const fn filter_enables(self, level: Level) -> bool {
        level as u32 >= self as u32
    }
}

impl<'d> serde::Deserialize<'d> for Level {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(|_| serde::de::Error::custom(format!("Unknown log level: {s}")))
    }
}

pub struct Logger {
    filter: Level,
    default_writer: Box<std::sync::Mutex<dyn std::io::Write + Send + Sync>>,
    disabled_buffer: String,
    disabled: Vec<(usize, usize)>,
}

impl Logger {
    pub fn enabled(&self, callsite: Callsite) -> bool {
        if !self.filter.filter_enables(callsite.level) {
            return false;
        }

        match self.disabled.binary_search_by(|(off, to)| {
            let cur = &self.disabled_buffer[*off..*to];

            cur.cmp(callsite.target)
        }) {
            Ok(_) => return false, // exact match is skipped
            Err(mut place) if place > 0 => {
                place -= 1;

                let cur_idx = self.disabled[place];
                let cur = &self.disabled_buffer[cur_idx.0..cur_idx.1];

                if cur.ends_with("::") && callsite.target.starts_with(cur) {
                    return false;
                }
            }
            _ => {}
        };

        true
    }
}

pub static __LOGGER: std::sync::OnceLock<Logger> = std::sync::OnceLock::new();

#[derive(Clone, Copy)]
pub struct Callsite {
    pub level: Level,
    pub target: &'static str,
}

pub enum Capture<'a> {
    Str(&'a str),
    Error(&'a (dyn std::error::Error + 'a)),
    Display(&'a (dyn std::fmt::Display + 'a)),
    Debug(&'a (dyn std::fmt::Debug + 'a)),
    Anyhow(&'a anyhow::Error),
    #[allow(clippy::type_complexity)]
    Serde(&'a (dyn Fn(&mut std::io::Cursor<&mut Vec<u8>>) -> Result<(), error::Error> + 'a)),
}

impl<'a> From<&'a (dyn std::error::Error + 'static)> for Capture<'a> {
    fn from(value: &'a (dyn std::error::Error + 'static)) -> Self {
        Capture::Error(value)
    }
}

impl<'a> From<&'a str> for Capture<'a> {
    fn from(value: &'a str) -> Self {
        Capture::Str(value)
    }
}

impl<'a> From<&'a anyhow::Error> for Capture<'a> {
    fn from(value: &'a anyhow::Error) -> Self {
        Capture::Anyhow(value)
    }
}

pub struct Record<'a> {
    pub callsite: Callsite,
    pub args: std::fmt::Arguments<'a>,
    pub kv: &'a [(&'static str, Capture<'a>)],
    pub file: &'static str,
    pub line: u32,
}

#[cfg(debug_assertions)]
pub const STATIC_MIN_LEVEL: Level = Level::Trace;

#[cfg(not(debug_assertions))]
pub const STATIC_MIN_LEVEL: Level = Level::Debug;

pub const fn statically_enabled(callsite: Callsite) -> bool {
    STATIC_MIN_LEVEL.filter_enables(callsite.level)
}

#[macro_export]
macro_rules! __make_capture {
    (= $value:expr) => {
        $crate::logger::Capture::Display(&$value)
    };

    (err = $value:expr) => {
        $crate::logger::Capture::Error(&$value)
    };

    (ah = $value:expr) => {
        $crate::logger::Capture::Anyhow(&$value)
    };

    (? = $value:expr) => {
        $crate::logger::Capture::Debug(&$value)
    };

    (serde = $value:expr) => {
        $crate::logger::Capture::Serde(&|v| {
            serde::Serialize::serialize(&$value, $crate::logger::Visitor(v))
        })
    };
}

#[macro_export]
macro_rules! __do_log {
    ($callsite:tt, $log:tt, $($key:tt $(:$capture:tt)? $(= $value:expr)?),+; $($arg:tt)+) => ({
        let res = $log.__try_log($crate::logger::Record {
            callsite: $callsite,
            args: format_args!($($arg)+),
            kv: &[$((stringify!($key), $crate::__make_capture!($($capture)* = $($value)*))),+] as &[_],
            file: file!(),
            line: line!(),
        });
        #[cfg(debug_assertions)]
        if let Err(e) = res {
            eprintln!("Error logging: {e:#}");
        }
    });

    ($callsite:tt, $log:tt, $($arg:tt)+) => ({
        let res = $log.__try_log($crate::logger::Record {
            callsite: $callsite,
            args: format_args!($($arg)+),
            kv: &[],
            file: file!(),
            line: line!(),
        });
        #[cfg(debug_assertions)]
        if let Err(e) = res {
            eprintln!("Error logging: {e:#}");
        }
    });
}

#[macro_export]
macro_rules! log_static {
    ($level:expr, $($arg:tt)+) => {{
        const CALLSITE: $crate::logger::Callsite = $crate::logger::Callsite {
            level: $level,
            target: module_path!(),
        };
        if const { $crate::logger::statically_enabled(CALLSITE) } {
            if let Some(cur_logger) = $crate::logger::__LOGGER.get() {
                if cur_logger.enabled(CALLSITE) {
                    $crate::__do_log!(CALLSITE, cur_logger, $($arg)+)
                }
            }
        }
    }}
}

#[macro_export]
macro_rules! log_with_level {
    ($level:expr, $($arg:tt)+) => {{
        let callsite: $crate::logger::Callsite = $crate::logger::Callsite {
            level: $level,
            target: module_path!(),
        };
        if let Some(cur_logger) = $crate::logger::__LOGGER.get() {
            if cur_logger.enabled(callsite) {
                $crate::__do_log!(callsite, cur_logger, $($arg)+)
            }
        }
    }}
}

#[macro_export]
macro_rules! log_enabled {
    ($level:expr) => {{
        if let Some(cur_logger) = $crate::logger::__LOGGER.get() {
            cur_logger.enabled($crate::logger::Callsite {
                level: $level,
                target: module_path!(),
            })
        } else {
            false
        }
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)+) => ($crate::log_static!($crate::logger::Level::Error, $($arg)+))
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)+) => ($crate::log_static!($crate::logger::Level::Warn, $($arg)+))
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)+) => ($crate::log_static!($crate::logger::Level::Info, $($arg)+))
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)+) => ($crate::log_static!($crate::logger::Level::Debug, $($arg)+))
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)+) => ($crate::log_static!($crate::logger::Level::Trace, $($arg)+))
}

static LOG_CACHED_BUFFERS: std::sync::LazyLock<crossbeam::queue::ArrayQueue<Vec<u8>>> =
    std::sync::LazyLock::new(|| crossbeam::queue::ArrayQueue::new(64));

fn write_str_part_escaping(
    buf: &mut std::io::Cursor<&mut Vec<u8>>,
    s: &str,
) -> std::io::Result<()> {
    for c in s.chars() {
        match c {
            '"' => buf.write_all(b"\\\"")?,
            '\\' => buf.write_all(b"\\\\")?,
            '\n' => buf.write_all(b"\\n")?,
            '\r' => buf.write_all(b"\\r")?,
            '\t' => buf.write_all(b"\\t")?,
            c if c.is_control() => {
                let mut under_buf = [0; 2];

                let code_points = c.encode_utf16(&mut under_buf);

                for p in code_points {
                    buf.write_fmt(format_args!("\\u{p:04x}"))?;
                }
            }
            c => buf.write_all(c.encode_utf8(&mut [0; 4]).as_bytes())?,
        }
    }
    Ok(())
}

fn get_utf8_char_prefix(s: &[u8]) -> Option<&str> {
    if s.is_empty() {
        return None;
    }

    let first_byte = s[0];

    // Determine expected UTF-8 sequence length
    let expected_len = if first_byte & 0x80 == 0 {
        1 // 0xxxxxxx (ASCII)
    } else if first_byte & 0xE0 == 0xC0 {
        2 // 110xxxxx
    } else if first_byte & 0xF0 == 0xE0 {
        3 // 1110xxxx
    } else if first_byte & 0xF8 == 0xF0 {
        4 // 11110xxx
    } else {
        return None;
    };

    if s.len() < expected_len {
        return None;
    }

    std::str::from_utf8(&s[..expected_len]).ok()
}

fn write_bytes_inner(buf: &mut std::io::Cursor<&mut Vec<u8>>, s: &[u8]) -> std::io::Result<()> {
    let mut i = 0;
    while i < s.len() {
        if let Some(prefix) = get_utf8_char_prefix(&s[i..]) {
            let ch = prefix.chars().next().unwrap();
            if ch.is_control() {
                buf.write_fmt(format_args!("\\u{:04x}", s[i]))?;

                i += 1;
            } else {
                write_str_part_escaping(buf, prefix)?;
            }

            i += prefix.len();
        } else {
            buf.write_fmt(format_args!("\\u{:04x}", s[i]))?;

            i += 1;
        }
    }

    Ok(())
}

fn write_bytes(buf: &mut std::io::Cursor<&mut Vec<u8>>, s: &[u8]) -> std::io::Result<()> {
    buf.write_all(b"\"$Bytes(")?;

    if s.len() > 128 {
        write_bytes_inner(buf, &s[..64])?;
        buf.write_all(b"...")?;
        write_bytes_inner(buf, &s[s.len() - 64..])?;
    } else {
        write_bytes_inner(buf, s)?;
    }

    buf.write_all(b")\"")?;

    Ok(())
}

fn write_quoted_str_escaping(
    buf: &mut std::io::Cursor<&mut Vec<u8>>,
    s: &str,
) -> std::io::Result<()> {
    buf.write_all(b"\"")?;
    write_str_escaping(buf, s)?;
    buf.write_all(b"\"")?;

    Ok(())
}

fn write_str_escaping(buf: &mut std::io::Cursor<&mut Vec<u8>>, s: &str) -> std::io::Result<()> {
    if s.starts_with("$") {
        // If the string starts with a dollar sign, we escape it to avoid confusion with variables.
        buf.write_all(b"$")?;
    }

    write_str_part_escaping(buf, s)
}

fn write_comma(buf: &mut std::io::Cursor<&mut Vec<u8>>) -> std::io::Result<()> {
    buf.write_all(b",")
}

fn write_k_v_str_fast(
    buf: &mut std::io::Cursor<&mut Vec<u8>>,
    k: &str,
    v: &str,
) -> std::io::Result<()> {
    buf.write_all(b"\"")?;
    buf.write_all(k.as_bytes())?;
    buf.write_all(b"\":\"")?;

    write_str_escaping(buf, v)?;

    buf.write_all(b"\"")?;

    Ok(())
}

pub struct Visitor<'a, 'w>(pub &'a mut std::io::Cursor<&'w mut Vec<u8>>);

pub struct SerializeVec<'a, 'w> {
    cur: &'a mut std::io::Cursor<&'w mut Vec<u8>>,
    put_comma: bool,
    close_curly: bool,
}

pub struct SerializeMap<'a, 'w> {
    cur: &'a mut std::io::Cursor<&'w mut Vec<u8>>,
    put_comma: bool,
    close_curly: bool,
}

impl Visitor<'_, '_> {
    fn serialize_with_special<T>(&mut self, value: &T) -> Result<(), error::Error>
    where
        T: ?Sized + Serialize,
    {
        let full_name = std::any::type_name_of_val(value);

        match full_name {
            "genvm_common::calldata::types::Value" => {
                let casted = unsafe {
                    (std::ptr::from_ref(value) as *const calldata::Value)
                        .as_ref()
                        .unwrap()
                };
                match casted {
                    calldata::Value::Number(num) => {
                        self.serialize_with_special(num)?;
                    }
                    calldata::Value::Address(addr) => {
                        self.serialize_with_special(addr)?;
                    }
                    _ => value.serialize(Visitor(self.0))?,
                }
                Ok(())
            }
            "genvm_common::calldata::types::Address" => {
                let casted = unsafe {
                    (std::ptr::from_ref(value) as *const calldata::Address)
                        .as_ref()
                        .unwrap()
                };
                self.0
                    .write_fmt(format_args!("\"$Address({})\"", hex::encode(casted.raw())))?;

                Ok(())
            }
            "primitive_types::U256" => {
                let casted = unsafe {
                    (std::ptr::from_ref(value) as *const primitive_types::U256)
                        .as_ref()
                        .unwrap()
                };
                self.0.write_fmt(format_args!("{}", casted))?;

                Ok(())
            }
            "num_bigint::bigint::BigInt" => {
                let casted = unsafe {
                    (std::ptr::from_ref(value) as *const num_bigint::BigInt)
                        .as_ref()
                        .unwrap()
                };
                self.0.write_fmt(format_args!("{}", casted))?;

                Ok(())
            }
            _ => value.serialize(Visitor(self.0)),
        }
    }
}

impl serde::ser::SerializeSeq for SerializeVec<'_, '_> {
    type Ok = ();

    type Error = error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.put_comma {
            self.cur.write_all(b",")?;
        } else {
            self.cur.write_all(b"[")?;
            self.put_comma = true;
        }

        Visitor(self.cur).serialize_with_special(value)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.put_comma {
            self.cur.write_all(b"[")?;
        }
        self.cur.write_all(b"]")?;

        if self.close_curly {
            self.cur.write_all(b"}")?;
        }
        Ok(())
    }
}

impl serde::ser::SerializeTupleStruct for SerializeVec<'_, '_> {
    type Ok = ();
    type Error = error::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeTupleVariant for SerializeVec<'_, '_> {
    type Ok = ();
    type Error = error::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeMap for SerializeMap<'_, '_> {
    type Ok = ();
    type Error = error::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.put_comma {
            self.cur.write_all(b",")?;
        } else {
            self.cur.write_all(b"{")?;
            self.put_comma = true;
        }

        let key = serde_json::to_string(key).map_err(|e| error::Error(e.into()))?;
        if key.starts_with("\"") && key.ends_with("\"") {
            self.cur.write_all(key.as_bytes())?;
        } else {
            write_quoted_str_escaping(self.cur, key.as_str())?;
        }

        self.cur.write_all(b":")?;

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        Visitor(self.cur).serialize_with_special(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if !self.put_comma {
            self.cur.write_all(b"{")?;
        }
        self.cur.write_all(b"}")?;
        if self.close_curly {
            self.cur.write_all(b"}")?;
        }
        Ok(())
    }
}

impl serde::ser::SerializeTuple for SerializeVec<'_, '_> {
    type Ok = ();

    type Error = error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        serde::ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeSeq::end(self)
    }
}

impl serde::ser::SerializeStruct for SerializeMap<'_, '_> {
    type Ok = ();

    type Error = error::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.put_comma {
            self.cur.write_all(b",")?;
        } else {
            self.cur.write_all(b"{")?;
            self.put_comma = true;
        }
        write_quoted_str_escaping(self.cur, key)?;
        self.cur.write_all(b":")?;
        Visitor(self.cur).serialize_with_special(value)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.cur.write_all(b"}")?;
        if self.close_curly {
            self.cur.write_all(b"}")?;
        }
        Ok(())
    }
}

impl serde::ser::SerializeStructVariant for SerializeMap<'_, '_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        serde::ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        serde::ser::SerializeMap::end(self)
    }
}

impl<'a, 'w> serde::Serializer for Visitor<'a, 'w> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = SerializeVec<'a, 'w>;
    type SerializeTuple = SerializeVec<'a, 'w>;
    type SerializeTupleStruct = SerializeVec<'a, 'w>;
    type SerializeTupleVariant = SerializeVec<'a, 'w>;
    type SerializeMap = SerializeMap<'a, 'w>;
    type SerializeStruct = SerializeMap<'a, 'w>;
    type SerializeStructVariant = SerializeMap<'a, 'w>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        if v {
            self.0.write_all(b"true")?;
        } else {
            self.0.write_all(b"false")?;
        }

        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.0.write_all(v.to_string().as_bytes())?;

        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.0.write_all(v.to_string().as_bytes())?;

        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if v.is_nan() {
            self.0.write_all(b"\"$nan\"")?;
        } else if v.is_infinite() {
            if v.is_sign_positive() {
                self.0.write_all(b"\"$+inf\"")?;
            } else {
                self.0.write_all(b"\"$-inf\"")?;
            }
        } else {
            self.0.write_all(v.to_string().as_bytes())?;
        }

        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = [0; 4];
        self.serialize_str(v.encode_utf8(&mut buf))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        write_quoted_str_escaping(self.0, v)?;

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_bytes(self.0, v)?;

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(mut self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.serialize_with_special(value)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.write_all(b"null")?;

        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        write_quoted_str_escaping(self.0, name)?;

        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(self.0, name)?;
        self.0.write_all(b"\":")?;
        Visitor(self.0).serialize_with_special(value)?;
        self.0.write_all(b"}")?;

        Ok(())
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(self.0, variant)?;
        self.0.write_all(b"\":")?;
        Visitor(self.0).serialize_with_special(value)?;
        self.0.write_all(b"}")?;
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeVec {
            cur: self.0,
            put_comma: false,
            close_curly: false,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeVec {
            cur: self.0,
            put_comma: false,
            close_curly: false,
        })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(self.0, name)?;
        self.0.write_all(b"\":")?;

        Ok(SerializeVec {
            cur: self.0,
            put_comma: false,
            close_curly: true,
        })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(self.0, variant)?;
        self.0.write_all(b"\":")?;

        Ok(SerializeVec {
            cur: self.0,
            put_comma: false,
            close_curly: true,
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap {
            cur: self.0,
            put_comma: false,
            close_curly: false,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeMap {
            cur: self.0,
            put_comma: false,
            close_curly: false,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(self.0, variant)?;
        self.0.write_all(b"\":")?;

        Ok(SerializeMap {
            cur: self.0,
            put_comma: false,
            close_curly: true,
        })
    }
}

impl<'a> Visitor<'a, '_> {
    fn dump_error(&mut self, err: &(dyn std::error::Error + 'a)) -> Result<(), error::Error> {
        self.0.write_all(b"{")?;
        write_k_v_str_fast(self.0, "message", &format!("{err:#}"))?;
        if let Some(source) = err.source() {
            self.0.write_all(b",\"source\":")?;
            self.dump_error(source)?;
        }
        self.0.write_all(b"}")?;

        Ok(())
    }

    fn dump_anyhow(&mut self, err: &anyhow::Error) -> Result<(), error::Error> {
        self.0.write_all(b"{\"causes\":[")?;

        let mut first = true;

        for c in err.chain() {
            if !first {
                self.0.write_all(b",")?;
            } else {
                first = false;
            }

            write_quoted_str_escaping(self.0, &format!("{c:#}"))?;
        }
        self.0.write_all(b"]")?;

        if let std::backtrace::BacktraceStatus::Captured = err.backtrace().status() {
            let trace_str = err.backtrace().to_string();

            self.0.write_all(b",\"backtrace\":[")?;

            let mut first = true;
            for line in trace_str.lines() {
                if !first {
                    self.0.write_all(b",")?;
                } else {
                    first = false;
                }

                write_quoted_str_escaping(self.0, line)?;
            }

            self.0.write_all(b"]")?;
        }

        self.0.write_all(b"}")?;

        Ok(())
    }
}

impl Logger {
    fn log_inner(&self, buf: &mut Vec<u8>, record: Record<'_>) -> std::result::Result<(), Error> {
        buf.clear();
        let mut writer = std::io::Cursor::new(buf);
        writer.write_all(b"{")?;

        writer.write_all(format!("\"level\":\"{}\",", record.callsite.level).as_bytes())?;
        write_k_v_str_fast(&mut writer, "target", record.callsite.target)?;

        write_comma(&mut writer)?;

        if let Some(msg) = record.args.as_str() {
            write_k_v_str_fast(&mut writer, "message", msg)?;
        } else {
            write_k_v_str_fast(&mut writer, "message", &record.args.to_string())?;
        }

        let mut visitor = Visitor(&mut writer);
        for (k, v) in record.kv {
            write_comma(visitor.0)?;
            write_quoted_str_escaping(visitor.0, k)?;
            visitor.0.write_all(b":")?;

            match v {
                Capture::Error(e) => visitor.dump_error(*e)?,
                Capture::Anyhow(e) => visitor.dump_anyhow(e)?,
                Capture::Str(s) => write_quoted_str_escaping(visitor.0, s)?,
                Capture::Display(d) => write_quoted_str_escaping(visitor.0, &d.to_string())?,
                Capture::Debug(d) => {
                    write_quoted_str_escaping(visitor.0, &format!("{d:?}"))?;
                }
                Capture::Serde(serde_fn) => {
                    serde_fn(visitor.0)?;
                }
            }
        }

        write_comma(&mut writer)?;

        writer.write_all(b"\"file\":\"")?;
        write_str_escaping(&mut writer, record.file)?;
        writer.write_all(b":")?;
        writer.write_all(record.line.to_string().as_bytes())?;
        writer.write_all(b"\"")?;

        write_comma(&mut writer)?;
        write_k_v_str_fast(
            &mut writer,
            "ts",
            &std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string(),
        )?;

        writer.write_all(b"}")?;
        writer.flush()?;

        Ok(())
    }

    pub fn __try_log(&self, record: Record<'_>) -> std::result::Result<(), Error> {
        let mut buf = LOG_CACHED_BUFFERS.pop().unwrap_or_default();

        buf.clear();

        let res = self.log_inner(&mut buf, record);

        let mut writer = self.default_writer.lock().unwrap();

        writer.write_all(&buf)?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        std::mem::drop(writer);

        buf.clear();
        buf.shrink_to(4 * 1024);

        let _ = LOG_CACHED_BUFFERS.push(buf);

        res
    }
}

pub fn initialize<W>(filter: Level, disabled: &str, writer: W)
where
    W: std::io::Write + Send + Sync + 'static,
{
    let default_writer = Box::new(std::sync::Mutex::new(writer));

    let disabled_src: Vec<&str> = disabled.split(",").collect();
    let mut disabled = Vec::with_capacity(disabled_src.len());

    for x in disabled_src {
        if let Some(stripped) = x.strip_suffix("*") {
            disabled.push(stripped.to_owned());
            if !x.ends_with("::*") {
                let mut my = stripped.to_owned();
                my.push_str("::");
                disabled.push(my);
            }
        } else {
            disabled.push(x.to_owned());
        }
    }
    disabled.sort();

    let mut all_buffer = String::new();
    for x in &mut disabled {
        all_buffer.push_str(x);
    }

    let mut new_disabled = Vec::with_capacity(disabled.len());
    let mut off = 0;
    for x in disabled {
        let len = x.len();
        new_disabled.push((off, off + x.len()));
        off += len;
    }

    let logger = Logger {
        filter,
        default_writer,
        disabled: new_disabled,
        disabled_buffer: all_buffer,
    };

    if let Err(logger) = __LOGGER.set(logger) {
        if let Ok(mut lock) = logger.default_writer.lock() {
            let _ = lock.write_all(r#"{"error":"Logger already initialized"}"#.as_bytes());
        }
    } else {
        let old_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            log_panic(info);
            old_hook(info);
        }));
    }
}

fn log_panic(info: &std::panic::PanicHookInfo<'_>) {
    use std::backtrace::Backtrace;
    use std::thread;

    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let backtrace = Backtrace::force_capture();

    let key_values = [
        ("backtrace", Capture::Debug(&backtrace)),
        ("thread_name", Capture::Str(thread_name)),
    ];
    let key_values = key_values.as_slice();

    if let Some(logger) = __LOGGER.get() {
        let _ = logger.__try_log(Record {
            callsite: Callsite {
                level: Level::Error,
                target: "panic",
            },
            args: format_args!("thread '{thread_name}' panicked {info}"),
            kv: key_values,
            file: file!(),
            line: line!(),
        });
    }
}

#[cfg(test)]
mod tests {

    #[derive(serde::Serialize)]
    struct SerializableNoCopy(i32);

    #[test]
    fn compiles() {
        log_error!(x = 11; "just string");
        log_error!(x:? = 11; "just string");
        log_error!(x:serde = 11; "just string");
        let _bar = {
            let ser = SerializableNoCopy(11);
            log_error!(x:serde = ser; "just string");
            log_error!(x:serde = ser; "just string");
            log_error!(x:serde = ser.0; "just string");
            ser
        };
        log_error!(x:serde = serde_json::json!({"foo": "bar"}); "just string");
    }
}
