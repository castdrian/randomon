use crate::error::{Result, UnityError};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Struct(Fields),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fields(pub Vec<(String, Value)>);

impl Fields {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.0
            .iter_mut()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut Value)> {
        self.0
            .iter_mut()
            .map(|(name, value)| (name.as_str(), value))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn out_of_range(value: impl ToString, kind: &'static str) -> UnityError {
    UnityError::OutOfRange {
        value: value.to_string(),
        kind,
    }
}

impl Value {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::I8(_) => "SInt8",
            Self::U8(_) => "UInt8",
            Self::I16(_) => "SInt16",
            Self::U16(_) => "UInt16",
            Self::I32(_) => "SInt32",
            Self::U32(_) => "UInt32",
            Self::I64(_) => "SInt64",
            Self::U64(_) => "UInt64",
            Self::F32(_) => "float",
            Self::F64(_) => "double",
            Self::String(_) => "string",
            Self::Bytes(_) => "TypelessData",
            Self::Array(_) => "Array",
            Self::Map(_) => "map",
            Self::Struct(_) => "struct",
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Bool(_)
                | Self::I8(_)
                | Self::U8(_)
                | Self::I16(_)
                | Self::U16(_)
                | Self::I32(_)
                | Self::U32(_)
                | Self::I64(_)
                | Self::U64(_)
        )
    }

    pub fn as_i128(&self) -> Option<i128> {
        Some(match *self {
            Self::Bool(value) => i128::from(value),
            Self::I8(value) => i128::from(value),
            Self::U8(value) => i128::from(value),
            Self::I16(value) => i128::from(value),
            Self::U16(value) => i128::from(value),
            Self::I32(value) => i128::from(value),
            Self::U32(value) => i128::from(value),
            Self::I64(value) => i128::from(value),
            Self::U64(value) => i128::from(value),
            _ => return None,
        })
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_i128().and_then(|value| i64::try_from(value).ok())
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Self::F32(value) => Some(f64::from(value)),
            Self::F64(value) => Some(value),
            _ => self.as_i128().map(|value| value as f64),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_struct(&self) -> Option<&Fields> {
        match self {
            Self::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<&mut Fields> {
        match self {
            Self::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn field(&self, name: &str) -> Result<&Value> {
        self.as_struct()
            .and_then(|fields| fields.get(name))
            .ok_or_else(|| UnityError::FieldNotFound(name.to_owned()))
    }

    pub fn field_mut(&mut self, name: &str) -> Result<&mut Value> {
        self.as_struct_mut()
            .and_then(|fields| fields.get_mut(name))
            .ok_or_else(|| UnityError::FieldNotFound(name.to_owned()))
    }

    pub fn item(&self, index: usize) -> Result<&Value> {
        let items = self
            .as_array()
            .ok_or_else(|| UnityError::ValueMismatch(format!("{} is not an array", self.kind())))?;
        items.get(index).ok_or(UnityError::IndexOutOfRange {
            index,
            length: items.len(),
        })
    }

    pub fn item_mut(&mut self, index: usize) -> Result<&mut Value> {
        let kind = self.kind();
        let items = self
            .as_array_mut()
            .ok_or_else(|| UnityError::ValueMismatch(format!("{kind} is not an array")))?;
        let length = items.len();
        items
            .get_mut(index)
            .ok_or(UnityError::IndexOutOfRange { index, length })
    }

    pub fn int(&self) -> Result<i64> {
        self.as_i64()
            .ok_or_else(|| UnityError::ValueMismatch(format!("{} is not an integer", self.kind())))
    }

    pub fn int_field(&self, name: &str) -> Result<i64> {
        self.field(name)?.int()
    }

    pub fn set_int(&mut self, value: i128) -> Result<()> {
        *self = match self {
            Self::Bool(_) => Self::Bool(match value {
                0 => false,
                1 => true,
                _ => return Err(out_of_range(value, "bool")),
            }),
            Self::I8(_) => Self::I8(i8::try_from(value).map_err(|_| out_of_range(value, "SInt8"))?),
            Self::U8(_) => Self::U8(u8::try_from(value).map_err(|_| out_of_range(value, "UInt8"))?),
            Self::I16(_) => {
                Self::I16(i16::try_from(value).map_err(|_| out_of_range(value, "SInt16"))?)
            }
            Self::U16(_) => {
                Self::U16(u16::try_from(value).map_err(|_| out_of_range(value, "UInt16"))?)
            }
            Self::I32(_) => {
                Self::I32(i32::try_from(value).map_err(|_| out_of_range(value, "SInt32"))?)
            }
            Self::U32(_) => {
                Self::U32(u32::try_from(value).map_err(|_| out_of_range(value, "UInt32"))?)
            }
            Self::I64(_) => {
                Self::I64(i64::try_from(value).map_err(|_| out_of_range(value, "SInt64"))?)
            }
            Self::U64(_) => {
                Self::U64(u64::try_from(value).map_err(|_| out_of_range(value, "UInt64"))?)
            }
            Self::F32(_) => Self::F32(value as f32),
            Self::F64(_) => Self::F64(value as f64),
            other => {
                return Err(UnityError::ValueMismatch(format!(
                    "cannot store an integer in {}",
                    other.kind()
                )));
            }
        };
        Ok(())
    }

    pub fn set_int_field(&mut self, name: &str, value: i128) -> Result<()> {
        self.field_mut(name)?.set_int(value)
    }

    pub fn set_float(&mut self, value: f64) -> Result<()> {
        *self = match self {
            Self::F32(_) => Self::F32(value as f32),
            Self::F64(_) => Self::F64(value),
            other => {
                return Err(UnityError::ValueMismatch(format!(
                    "cannot store a float in {}",
                    other.kind()
                )));
            }
        };
        Ok(())
    }

    pub fn set_float_field(&mut self, name: &str, value: f64) -> Result<()> {
        self.field_mut(name)?.set_float(value)
    }

    pub fn set_string(&mut self, text: &str) -> Result<()> {
        match self {
            Self::String(existing) => {
                text.clone_into(existing);
                Ok(())
            }
            other => Err(UnityError::ValueMismatch(format!(
                "cannot store a string in {}",
                other.kind()
            ))),
        }
    }
}
