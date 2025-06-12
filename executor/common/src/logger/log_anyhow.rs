use serde::ser::{SerializeSeq, SerializeStruct};

pub struct LogError<'a>(pub &'a anyhow::Error);

struct ChainSerialize<'a>(&'a anyhow::Error);

impl<'a> serde::Serialize for ChainSerialize<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for i in self.0.chain() {
            seq.serialize_element(&i.to_string())?;
        }

        seq.end()
    }
}

struct SplitStrSerialize<'a>(&'a str);

impl<'a> serde::Serialize for SplitStrSerialize<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for i in self.0.split('\n') {
            seq.serialize_element(i)?;
        }

        seq.end()
    }
}

impl serde::Serialize for LogError<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("LogError", 1)?;
        state.serialize_field("causes", &ChainSerialize(self.0))?;

        if let std::backtrace::BacktraceStatus::Captured = self.0.backtrace().status() {
            let trace_str = self.0.backtrace().to_string();

            state.serialize_field("trace", &SplitStrSerialize(&trace_str))?;
        }

        state.end()
    }
}
