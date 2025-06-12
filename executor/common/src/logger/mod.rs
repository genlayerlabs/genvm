mod error;
mod log_anyhow;

pub use log_anyhow::LogError;

use error::Error;
use serde::Serialize;

use std::io::Write;

pub struct Logger {
    filter: log::LevelFilter,
    default_writer: Box<std::sync::Mutex<dyn std::io::Write + Send + Sync>>,
    disabled_buffer: String,
    disabled: Vec<(usize, usize)>,
}

thread_local! {
    static LOG_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(Vec::new());
}

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
                buf.write_fmt(format_args!("\\x{:02x}", s[i]))?;

                i += 1;
            } else {
                write_str_part_escaping(buf, prefix)?;
            }

            i += prefix.len();
        } else {
            buf.write_fmt(format_args!("\\x{:02x}", s[i]))?;

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

struct Visitor<'a, 'w>(&'a mut std::io::Cursor<&'w mut Vec<u8>>);

struct SerializeVec<'a, 'w> {
    cur: &'a mut std::io::Cursor<&'w mut Vec<u8>>,
    put_comma: bool,
    close_curly: bool,
}

struct SerializeMap<'a, 'w> {
    cur: &'a mut std::io::Cursor<&'w mut Vec<u8>>,
    put_comma: bool,
    close_curly: bool,
}

impl<'a, 'w> Visitor<'a, 'w> {
    fn serialize_with_special<T>(&mut self, value: &T) -> Result<(), error::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(Visitor(self.0))
    }
}

impl<'a, 'w> serde::ser::SerializeSeq for SerializeVec<'a, 'w> {
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

impl<'a, 'w> serde::ser::SerializeTupleStruct for SerializeVec<'a, 'w> {
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

impl<'a, 'w> serde::ser::SerializeTupleVariant for SerializeVec<'a, 'w> {
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

impl<'a, 'w> serde::ser::SerializeMap for SerializeMap<'a, 'w> {
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
            self.cur.write_all(b"\"")?;
            write_str_part_escaping(self.cur, &key[1..key.len() - 1])?;
            self.cur.write_all(b"\"")?;
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

impl<'a, 'w> serde::ser::SerializeTuple for SerializeVec<'a, 'w> {
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

impl<'a, 'w> serde::ser::SerializeStruct for SerializeMap<'a, 'w> {
    type Ok = ();

    type Error = error::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        if self.put_comma {
            self.cur.write_all(b",")?;
        } else {
            self.put_comma = true;
        }
        self.cur.write_all(b"\"")?;
        write_str_part_escaping(self.cur, key)?;
        self.cur.write_all(b"\":")?;
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

impl<'a, 'w> serde::ser::SerializeStructVariant for SerializeMap<'a, 'w> {
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

    fn serialize_str(mut self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.0.write_all(b"\"")?;
        write_str_escaping(&mut self.0, v)?;
        self.0.write_all(b"\"")?;

        Ok(())
    }

    fn serialize_bytes(mut self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        write_bytes(&mut self.0, v)?;

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

    fn serialize_unit_struct(mut self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.0.write_all(b"\"")?;
        write_str_escaping(&mut self.0, name)?;
        self.0.write_all(b"\"")?;

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
        mut self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.0.write_all(b"{\"")?;
        write_str_part_escaping(&mut self.0, name)?;
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

impl<'a, 'kv, 'w> log::kv::VisitValue<'kv> for Visitor<'a, 'w> {
    fn visit_any(&mut self, value: log::kv::Value) -> Result<(), log::kv::Error> {
        Visitor(&mut self.0)
            .serialize_with_special(&value)
            .map_err(log::kv::Error::boxed)
    }

    fn visit_null(&mut self) -> Result<(), log::kv::Error> {
        self.0.write_all(b"null")?;
        Ok(())
    }

    fn visit_bool(&mut self, v: bool) -> Result<(), log::kv::Error> {
        if v {
            self.0.write_all(b"true")?;
        } else {
            self.0.write_all(b"false")?;
        }

        Ok(())
    }

    fn visit_u64(&mut self, value: u64) -> Result<(), log::kv::Error> {
        self.0.write_all(value.to_string().as_bytes())?;
        Ok(())
    }

    fn visit_f64(&mut self, value: f64) -> Result<(), log::kv::Error> {
        if value.is_nan() {
            self.0.write_all(b"\"$nan\"")?;
        } else if value.is_infinite() {
            if value.is_sign_positive() {
                self.0.write_all(b"\"$+inf\"")?;
            } else {
                self.0.write_all(b"\"$-inf\"")?;
            }
        } else {
            self.0.write_all(value.to_string().as_bytes())?;
        }

        Ok(())
    }

    fn visit_i64(&mut self, value: i64) -> Result<(), log::kv::Error> {
        self.0.write_all(value.to_string().as_bytes())?;
        Ok(())
    }

    fn visit_str(&mut self, value: &str) -> Result<(), log::kv::Error> {
        self.0.write_all(b"\"")?;
        write_str_escaping(&mut self.0, value)?;
        self.0.write_all(b"\"")?;
        Ok(())
    }

    fn visit_error(
        &mut self,
        err: &(dyn std::error::Error + 'static),
    ) -> Result<(), log::kv::Error> {
        self.0.write_all(b"{")?;
        write_k_v_str_fast(self.0, "message", &format!("{err:#}"))?;
        if let Some(source) = err.source() {
            self.0.write_all(b",\"source\":")?;
            self.visit_error(source)?;
        }
        self.0.write_all(b"}")?;

        Ok(())
    }

    fn visit_borrowed_error(
        &mut self,
        err: &'kv (dyn std::error::Error + 'static),
    ) -> Result<(), log::kv::Error> {
        self.0.write_all(b"{")?;
        write_k_v_str_fast(self.0, "message", &format!("{err:#}"))?;
        if let Some(source) = err.source() {
            self.0.write_all(b",\"source\":")?;
            self.visit_error(source)?;
        }
        self.0.write_all(b"}")?;

        Ok(())
    }
}

impl<'a, 'w> log::kv::VisitSource<'w> for Visitor<'a, 'w> {
    #[inline]
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'w>,
        value: log::kv::Value<'w>,
    ) -> Result<(), log::kv::Error> {
        self.0.write_all(b",\"")?;
        write_str_escaping(&mut self.0, key.as_str())?;
        self.0.write_all(b"\":")?;

        value.visit(self)?;

        Ok(())
    }
}

impl Logger {
    fn try_log(&self, record: &log::Record) -> std::result::Result<(), Error> {
        LOG_BUFFER.with_borrow_mut(|buf| {
            buf.clear();
            let mut writer = std::io::Cursor::new(buf);
            writer.write_all(b"{")?;

            writer.write_all(format!("\"level\":\"{}\",", record.level()).as_bytes())?;
            write_k_v_str_fast(&mut writer, "target", record.target())?;

            write_comma(&mut writer)?;

            if let Some(msg) = record.args().as_str() {
                write_k_v_str_fast(&mut writer, "message", msg)?;
            } else {
                write_k_v_str_fast(&mut writer, "message", &record.args().to_string())?;
            }

            let mut visitor = Visitor(&mut writer);
            record.key_values().visit(&mut visitor)?;

            if let Some(file) = record.file() {
                write_comma(&mut writer)?;

                if let Some(line) = record.line() {
                    writer.write_all(b"\"file\":\"")?;
                    write_str_escaping(&mut writer, file)?;
                    writer.write_all(b":")?;
                    writer.write_all(line.to_string().as_bytes())?;
                    writer.write_all(b"\"")?;
                } else {
                    write_k_v_str_fast(&mut writer, "file", file)?;
                }
            }

            if let Some(module) = record.module_path() {
                write_comma(&mut writer)?;
                write_k_v_str_fast(&mut writer, "module", module)?;
            }

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

            let buf = writer.into_inner();

            let mut writer = self.default_writer.lock().unwrap();

            writer.write_all(&buf)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            buf.clear();

            Ok(())
        })
    }
}

impl log::Log for Logger {
    #[inline(always)]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if self.filter < metadata.level() {
            return false;
        }

        match self.disabled.binary_search_by(|(off, to)| {
            let cur = &self.disabled_buffer[*off..*to];

            cur.cmp(metadata.target())
        }) {
            Ok(_) => return false, // exact match is skipped
            Err(mut place) if place > 0 => {
                place -= 1;

                let cur_idx = self.disabled[place];
                let cur = &self.disabled_buffer[cur_idx.0..cur_idx.1];

                if cur.ends_with("::") && metadata.target().starts_with(cur) {
                    return false;
                }
            }
            _ => {}
        };

        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Err(e) = self.try_log(record) {
            eprintln!("logging failed with error: {e:#}");
        }
    }

    fn flush(&self) {}
}

pub fn initialize<W>(filter: log::LevelFilter, disabled: &str, writer: W)
where
    W: std::io::Write + Send + Sync + 'static,
{
    let default_writer = Box::new(std::sync::Mutex::new(writer));

    let disabled_src: Vec<&str> = disabled.split(",").collect();
    let mut disabled = Vec::with_capacity(disabled_src.len());

    for x in disabled_src {
        if x.ends_with("*") {
            disabled.push(x[..x.len() - 1].to_owned());
            if !x.ends_with("::*") {
                let mut my = x[..x.len() - 1].to_owned();
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

    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(filter);

    std::panic::set_hook(Box::new(log_panic));
}

fn log_panic(info: &std::panic::PanicHookInfo<'_>) {
    use std::backtrace::Backtrace;
    use std::thread;

    let mut record = log::Record::builder();
    let thread = thread::current();
    let thread_name = thread.name().unwrap_or("unnamed");
    let backtrace = Backtrace::force_capture();

    let key_values = [
        ("backtrace", log::kv::Value::from_debug(&backtrace)),
        ("thread_name", log::kv::Value::from(thread_name)),
    ];
    let key_values = key_values.as_slice();

    let _ = record
        .level(log::Level::Error)
        .target("panic")
        .key_values(&key_values);

    if let Some(location) = info.location() {
        let _ = record
            .file(Some(location.file()))
            .line(Some(location.line()));
    };

    log::logger().log(
        &record
            .args(format_args!("thread '{thread_name}' {info}"))
            .build(),
    );
}
