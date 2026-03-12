#[derive(Debug)]
pub enum Error {
    InvalidType {
        unexpected: String,
        expected: String,
    },
    InvalidValue {
        unexpected: String,
        expected: String,
    },
    InvalidLength {
        len: usize,
        expected: String,
    },
    DuplicateField(&'static str),
    MissingField(&'static str),
    UnknownField {
        field: String,
        expected: &'static [&'static str],
    },
    UnknownVariant {
        variant: String,
        expected: &'static [&'static str],
    },
    OnlyStringKeysSupported,
    NumberTooBig,
    FloatHasFractionalPart,
    FloatOutOfRange,
    CharsNotSupported,
    UnexpectedAddress {
        target_type: &'static str,
    },
    Custom(String),
    /// Error with field path context
    AtField {
        field: String,
        source: Box<Error>,
    },
    /// Error with array index context
    AtIndex {
        index: usize,
        source: Box<Error>,
    },
}

impl Error {
    /// Wrap this error with field context
    pub fn at_field(self, field: impl Into<String>) -> Self {
        Error::AtField {
            field: field.into(),
            source: Box::new(self),
        }
    }

    /// Wrap this error with array index context
    pub fn at_index(self, index: usize) -> Self {
        Error::AtIndex {
            index,
            source: Box::new(self),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidType {
                unexpected,
                expected,
            } => {
                write!(f, "invalid type: {unexpected}, expected {expected}")
            }
            Error::InvalidValue {
                unexpected,
                expected,
            } => {
                write!(f, "invalid value: {unexpected}, expected {expected}")
            }
            Error::InvalidLength { len, expected } => {
                write!(f, "invalid length {len}, expected {expected}")
            }
            Error::DuplicateField(field) => write!(f, "duplicate field `{field}`"),
            Error::MissingField(field) => write!(f, "missing field `{field}`"),
            Error::UnknownField { field, expected } => {
                write!(f, "unknown field `{field}`, expected one of {expected:?}")
            }
            Error::UnknownVariant { variant, expected } => {
                write!(
                    f,
                    "unknown variant `{variant}`, expected one of {expected:?}"
                )
            }
            Error::OnlyStringKeysSupported => write!(f, "only string keys are supported"),
            Error::NumberTooBig => write!(f, "number is too big"),
            Error::FloatHasFractionalPart => write!(f, "float has fractional part"),
            Error::FloatOutOfRange => write!(f, "float out of range for serialization"),
            Error::CharsNotSupported => write!(f, "chars are not supported"),
            Error::UnexpectedAddress { target_type } => {
                write!(f, "unexpected address for {target_type}")
            }
            Error::Custom(msg) => write!(f, "{msg}"),
            Error::AtField { field, source } => {
                write!(f, "at field `{field}`: {source}")
            }
            Error::AtIndex { index, source } => {
                write!(f, "at index [{index}]: {source}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Custom(msg.to_string())
    }

    fn invalid_type(unexpected: serde::de::Unexpected, exp: &dyn serde::de::Expected) -> Self {
        Error::InvalidType {
            unexpected: format!("{unexpected:?}"),
            expected: exp.to_string(),
        }
    }

    fn invalid_value(unexpected: serde::de::Unexpected, exp: &dyn serde::de::Expected) -> Self {
        Error::InvalidValue {
            unexpected: format!("{unexpected:?}"),
            expected: exp.to_string(),
        }
    }

    fn invalid_length(len: usize, exp: &dyn serde::de::Expected) -> Self {
        Error::InvalidLength {
            len,
            expected: exp.to_string(),
        }
    }

    fn duplicate_field(field: &'static str) -> Self {
        Error::DuplicateField(field)
    }

    fn missing_field(field: &'static str) -> Self {
        Error::MissingField(field)
    }

    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        Error::UnknownField {
            field: field.to_owned(),
            expected,
        }
    }

    fn unknown_variant(variant: &str, expected: &'static [&'static str]) -> Self {
        Error::UnknownVariant {
            variant: variant.to_owned(),
            expected,
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Error::Custom(msg.to_string())
    }
}
