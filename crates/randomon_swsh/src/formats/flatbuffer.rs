use crate::bytes::{bytes_at, i32_at, u16_at, u32_at};
use crate::error::{Result, invalid};

const CONTEXT: &str = "FlatBuffer";
const OFFSET_SIZE: usize = 4;

pub trait Scalar: Copy + Default + PartialEq + std::fmt::Debug {
    const SIZE: usize;
    fn decode(bytes: &[u8]) -> Self;
    fn encode(self, output: &mut [u8]);
}

macro_rules! numeric_scalar {
    ($($type:ty),*) => {
        $(
            impl Scalar for $type {
                const SIZE: usize = std::mem::size_of::<$type>();

                fn decode(bytes: &[u8]) -> Self {
                    let mut array = [0u8; std::mem::size_of::<$type>()];
                    array.copy_from_slice(&bytes[..Self::SIZE]);
                    <$type>::from_le_bytes(array)
                }

                fn encode(self, output: &mut [u8]) {
                    output[..Self::SIZE].copy_from_slice(&self.to_le_bytes());
                }
            }
        )*
    };
}

numeric_scalar!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl Scalar for bool {
    const SIZE: usize = 1;

    fn decode(bytes: &[u8]) -> Self {
        bytes[0] != 0
    }

    fn encode(self, output: &mut [u8]) {
        output[0] = u8::from(self);
    }
}

fn read_scalar<T: Scalar>(data: &[u8], position: usize) -> Result<T> {
    Ok(T::decode(bytes_at(data, position, T::SIZE, CONTEXT)?))
}

fn follow_offset(data: &[u8], position: usize) -> Result<usize> {
    let relative = u32_at(data, position, CONTEXT)? as usize;
    let target = position
        .checked_add(relative)
        .filter(|&target| target < data.len())
        .ok_or_else(|| {
            invalid(
                CONTEXT,
                format!("offset at {position:#x} points outside the buffer"),
            )
        })?;
    Ok(target)
}

#[derive(Debug, Clone, Copy)]
pub struct TableRef<'a> {
    data: &'a [u8],
    position: usize,
    vtable: usize,
    vtable_size: usize,
    table_size: usize,
}

impl<'a> TableRef<'a> {
    pub fn root(data: &'a [u8]) -> Result<Self> {
        let position = follow_offset(data, 0)?;
        Self::at(data, position)
    }

    pub fn at(data: &'a [u8], position: usize) -> Result<Self> {
        let relative = i64::from(i32_at(data, position, CONTEXT)?);
        let vtable = i64::try_from(position)
            .ok()
            .and_then(|position| position.checked_sub(relative))
            .and_then(|vtable| usize::try_from(vtable).ok())
            .ok_or_else(|| {
                invalid(
                    CONTEXT,
                    format!("table at {position:#x} has an invalid vtable offset"),
                )
            })?;
        let vtable_size = usize::from(u16_at(data, vtable, CONTEXT)?);
        if vtable_size < 4 || !vtable_size.is_multiple_of(2) {
            return Err(invalid(
                CONTEXT,
                format!("vtable at {vtable:#x} has size {vtable_size}"),
            ));
        }
        bytes_at(data, vtable, vtable_size, CONTEXT)?;
        let table_size = usize::from(u16_at(data, vtable + 2, CONTEXT)?);
        if table_size < OFFSET_SIZE {
            return Err(invalid(
                CONTEXT,
                format!("table at {position:#x} has size {table_size}"),
            ));
        }
        bytes_at(data, position, table_size, CONTEXT)?;
        Ok(TableRef {
            data,
            position,
            vtable,
            vtable_size,
            table_size,
        })
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn field_slots(&self) -> usize {
        (self.vtable_size - 4) / 2
    }

    pub fn table_size(&self) -> usize {
        self.table_size
    }

    pub fn field_position(&self, index: usize) -> Result<Option<usize>> {
        if index >= self.field_slots() {
            return Ok(None);
        }
        let relative = usize::from(u16_at(self.data, self.vtable + 4 + index * 2, CONTEXT)?);
        if relative == 0 {
            return Ok(None);
        }
        if relative >= self.table_size {
            return Err(invalid(
                CONTEXT,
                format!(
                    "field {index} of table at {:#x} lies outside the table",
                    self.position
                ),
            ));
        }
        Ok(Some(self.position + relative))
    }

    pub fn has_field(&self, index: usize) -> Result<bool> {
        Ok(self.field_position(index)?.is_some())
    }

    pub fn present_fields(&self) -> Result<Vec<usize>> {
        let mut present = Vec::new();
        for index in 0..self.field_slots() {
            if self.has_field(index)? {
                present.push(index);
            }
        }
        Ok(present)
    }

    pub fn ensure_known_fields(&self, known: usize, context: &'static str) -> Result<()> {
        match self
            .present_fields()?
            .into_iter()
            .find(|&index| index >= known)
        {
            Some(index) => Err(invalid(
                context,
                format!(
                    "table at {:#x} has field {index}, but only {known} fields are known",
                    self.position
                ),
            )),
            None => Ok(()),
        }
    }

    pub fn fields_with_nonzero_slack(
        &self,
        size_of: impl Fn(usize) -> Option<usize>,
    ) -> Result<Vec<usize>> {
        let mut positions = Vec::new();
        for index in self.present_fields()? {
            if let Some(position) = self.field_position(index)? {
                positions.push((position, index));
            }
        }
        positions.sort_unstable();
        let table_end = self.position + self.table_size;
        let mut suspicious = Vec::new();
        for (slot, &(position, index)) in positions.iter().enumerate() {
            let next = positions.get(slot + 1).map_or(table_end, |(next, _)| *next);
            let size = size_of(index).unwrap_or(OFFSET_SIZE);
            let slack = bytes_at(self.data, position, next.saturating_sub(position), CONTEXT)?;
            if size > slack.len() || slack[size..].iter().any(|&byte| byte != 0) {
                suspicious.push(index);
            }
        }
        Ok(suspicious)
    }

    pub fn scalar<T: Scalar>(&self, index: usize) -> Result<T> {
        match self.field_position(index)? {
            Some(position) => read_scalar(self.data, position),
            None => Ok(T::default()),
        }
    }

    pub fn table(&self, index: usize) -> Result<Option<TableRef<'a>>> {
        match self.field_position(index)? {
            Some(position) => Ok(Some(TableRef::at(
                self.data,
                follow_offset(self.data, position)?,
            )?)),
            None => Ok(None),
        }
    }

    pub fn vector(&self, index: usize) -> Result<Option<VectorRef<'a>>> {
        match self.field_position(index)? {
            Some(position) => {
                let start = follow_offset(self.data, position)?;
                let length = u32_at(self.data, start, CONTEXT)? as usize;
                Ok(Some(VectorRef {
                    data: self.data,
                    start: start + OFFSET_SIZE,
                    length,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn tables(&self, index: usize) -> Result<Vec<TableRef<'a>>> {
        match self.vector(index)? {
            Some(vector) => vector.tables(),
            None => Ok(Vec::new()),
        }
    }

    pub fn scalars<T: Scalar>(&self, index: usize) -> Result<Vec<T>> {
        match self.vector(index)? {
            Some(vector) => vector.scalars(),
            None => Ok(Vec::new()),
        }
    }

    pub fn string(&self, index: usize) -> Result<Option<String>> {
        let Some(vector) = self.vector(index)? else {
            return Ok(None);
        };
        let bytes = bytes_at(self.data, vector.start, vector.length, CONTEXT)?;
        String::from_utf8(bytes.to_vec()).map(Some).map_err(|_| {
            invalid(
                CONTEXT,
                format!("string at {:#x} is not UTF-8", vector.start),
            )
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VectorRef<'a> {
    data: &'a [u8],
    start: usize,
    length: usize,
}

impl<'a> VectorRef<'a> {
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn element_position(&self, index: usize, element_size: usize) -> Result<usize> {
        if index >= self.length {
            return Err(invalid(
                CONTEXT,
                format!("vector index {index} of {}", self.length),
            ));
        }
        let position = index
            .checked_mul(element_size)
            .and_then(|offset| offset.checked_add(self.start))
            .ok_or_else(|| invalid(CONTEXT, "vector element offset overflows"))?;
        bytes_at(self.data, position, element_size, CONTEXT)?;
        Ok(position)
    }

    pub fn tables(&self) -> Result<Vec<TableRef<'a>>> {
        self.ensure_fits(OFFSET_SIZE)?;
        (0..self.length)
            .map(|index| {
                let slot = self.start + index * OFFSET_SIZE;
                TableRef::at(self.data, follow_offset(self.data, slot)?)
            })
            .collect()
    }

    pub fn scalars<T: Scalar>(&self) -> Result<Vec<T>> {
        self.ensure_fits(T::SIZE)?;
        (0..self.length)
            .map(|index| read_scalar(self.data, self.start + index * T::SIZE))
            .collect()
    }

    fn ensure_fits(&self, element_size: usize) -> Result<()> {
        let total = self
            .length
            .checked_mul(element_size)
            .ok_or_else(|| invalid(CONTEXT, "vector length overflows"))?;
        bytes_at(self.data, self.start, total, CONTEXT)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum FieldValue {
    Scalar { size: usize, bytes: Vec<u8> },
    String(String),
    Table(TableBuilder),
    Tables(Vec<TableBuilder>),
    Scalars { size: usize, bytes: Vec<u8> },
}

impl FieldValue {
    fn inline_size(&self) -> usize {
        match self {
            FieldValue::Scalar { size, .. } => *size,
            _ => OFFSET_SIZE,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableBuilder {
    fields: Vec<(usize, FieldValue)>,
}

impl TableBuilder {
    pub fn new() -> Self {
        TableBuilder::default()
    }

    fn set(&mut self, index: usize, value: FieldValue) -> &mut Self {
        self.fields.retain(|(existing, _)| *existing != index);
        self.fields.push((index, value));
        self
    }

    pub fn scalar<T: Scalar>(&mut self, index: usize, value: T) -> &mut Self {
        if value == T::default() {
            self.fields.retain(|(existing, _)| *existing != index);
            return self;
        }
        let mut bytes = vec![0u8; T::SIZE];
        value.encode(&mut bytes);
        self.set(
            index,
            FieldValue::Scalar {
                size: T::SIZE,
                bytes,
            },
        )
    }

    pub fn string(&mut self, index: usize, value: &str) -> &mut Self {
        self.set(index, FieldValue::String(value.to_string()))
    }

    pub fn table(&mut self, index: usize, value: TableBuilder) -> &mut Self {
        self.set(index, FieldValue::Table(value))
    }

    pub fn tables(&mut self, index: usize, values: Vec<TableBuilder>) -> &mut Self {
        self.set(index, FieldValue::Tables(values))
    }

    pub fn scalars<T: Scalar>(&mut self, index: usize, values: &[T]) -> &mut Self {
        let mut bytes = vec![0u8; values.len() * T::SIZE];
        for (index, value) in values.iter().enumerate() {
            value.encode(&mut bytes[index * T::SIZE..]);
        }
        self.set(
            index,
            FieldValue::Scalars {
                size: T::SIZE,
                bytes,
            },
        )
    }

    pub fn finish(&self) -> Vec<u8> {
        let mut writer = Writer {
            buffer: vec![0u8; OFFSET_SIZE],
        };
        let root = writer.write_table(self);
        writer.patch_offset(0, root);
        writer.buffer
    }
}

struct Writer {
    buffer: Vec<u8>,
}

struct TableLayout {
    offsets: Vec<(usize, usize)>,
    size: usize,
    alignment: usize,
    slots: usize,
}

impl TableLayout {
    fn new(table: &TableBuilder) -> Self {
        let mut ordered: Vec<&(usize, FieldValue)> = table.fields.iter().collect();
        ordered.sort_by_key(|(index, value)| (std::cmp::Reverse(value.inline_size()), *index));
        let mut cursor = OFFSET_SIZE;
        let mut alignment = OFFSET_SIZE;
        let mut offsets = Vec::with_capacity(ordered.len());
        for (index, value) in ordered {
            let size = value.inline_size();
            cursor = cursor.next_multiple_of(size);
            offsets.push((*index, cursor));
            cursor += size;
            alignment = alignment.max(size);
        }
        let slots = table
            .fields
            .iter()
            .map(|(index, _)| index + 1)
            .max()
            .unwrap_or(0);
        TableLayout {
            offsets,
            size: cursor,
            alignment,
            slots,
        }
    }

    fn offset_of(&self, index: usize) -> usize {
        self.offsets
            .iter()
            .find(|(field, _)| *field == index)
            .map_or(0, |(_, offset)| *offset)
    }
}

impl Writer {
    fn pad_to(&mut self, alignment: usize) {
        let length = self.buffer.len().next_multiple_of(alignment);
        self.buffer.resize(length, 0);
    }

    fn push_u16(&mut self, value: usize) {
        self.buffer.extend_from_slice(&(value as u16).to_le_bytes());
    }

    fn push_u32(&mut self, value: usize) {
        self.buffer.extend_from_slice(&(value as u32).to_le_bytes());
    }

    fn patch_offset(&mut self, at: usize, target: usize) {
        let relative = (target - at) as u32;
        self.buffer[at..at + OFFSET_SIZE].copy_from_slice(&relative.to_le_bytes());
    }

    fn write_table(&mut self, table: &TableBuilder) -> usize {
        let layout = TableLayout::new(table);
        self.pad_to(2);
        let vtable = self.buffer.len();
        self.push_u16(4 + layout.slots * 2);
        self.push_u16(layout.size);
        for index in 0..layout.slots {
            self.push_u16(layout.offset_of(index));
        }
        self.pad_to(layout.alignment);
        let position = self.buffer.len();
        self.buffer.resize(position + layout.size, 0);
        let soffset = (position - vtable) as i32;
        self.buffer[position..position + OFFSET_SIZE].copy_from_slice(&soffset.to_le_bytes());
        for (index, value) in &table.fields {
            if let FieldValue::Scalar { bytes, .. } = value {
                let at = position + layout.offset_of(*index);
                self.buffer[at..at + bytes.len()].copy_from_slice(bytes);
            }
        }
        let mut references: Vec<&(usize, FieldValue)> = table
            .fields
            .iter()
            .filter(|(_, value)| !matches!(value, FieldValue::Scalar { .. }))
            .collect();
        references.sort_by_key(|(index, _)| *index);
        for (index, value) in references {
            let child = self.write_child(value);
            self.patch_offset(position + layout.offset_of(*index), child);
        }
        position
    }

    fn write_child(&mut self, value: &FieldValue) -> usize {
        match value {
            FieldValue::Scalar { .. } => 0,
            FieldValue::String(text) => {
                self.pad_to(OFFSET_SIZE);
                let position = self.buffer.len();
                self.push_u32(text.len());
                self.buffer.extend_from_slice(text.as_bytes());
                self.buffer.push(0);
                position
            }
            FieldValue::Table(table) => self.write_table(table),
            FieldValue::Tables(tables) => {
                self.pad_to(OFFSET_SIZE);
                let position = self.buffer.len();
                self.push_u32(tables.len());
                let slots = self.buffer.len();
                self.buffer.resize(slots + tables.len() * OFFSET_SIZE, 0);
                for (index, table) in tables.iter().enumerate() {
                    let child = self.write_table(table);
                    self.patch_offset(slots + index * OFFSET_SIZE, child);
                }
                position
            }
            FieldValue::Scalars { size, bytes } => {
                let alignment = (*size).max(OFFSET_SIZE);
                while !(self.buffer.len() + OFFSET_SIZE).is_multiple_of(alignment) {
                    self.buffer.push(0);
                }
                let position = self.buffer.len();
                self.push_u32(bytes.len() / size);
                self.buffer.extend_from_slice(bytes);
                position
            }
        }
    }
}

pub type FieldMap = fn(usize) -> Option<usize>;

pub fn identity_field_map(index: usize) -> Option<usize> {
    Some(index)
}

macro_rules! scalar_table {
    ($(#[$attribute:meta])* pub struct $name:ident { $($index:literal => $field:ident: $type:ty,)* }) => {
        $(#[$attribute])*
        #[derive(Debug, Clone, Copy, Default, PartialEq)]
        pub struct $name {
            $(pub $field: $type,)*
        }

        impl $name {
            pub const FIELD_SIZES: &'static [(usize, usize)] = &[
                $(($index, <$type as $crate::formats::flatbuffer::Scalar>::SIZE),)*
            ];

            pub fn field_size(index: usize) -> Option<usize> {
                Self::FIELD_SIZES
                    .iter()
                    .find(|(field, _)| *field == index)
                    .map(|(_, size)| *size)
            }

            pub fn read(table: &$crate::formats::flatbuffer::TableRef<'_>) -> $crate::error::Result<Self> {
                Self::read_mapped(table, $crate::formats::flatbuffer::identity_field_map)
            }

            pub fn read_mapped(
                table: &$crate::formats::flatbuffer::TableRef<'_>,
                map: $crate::formats::flatbuffer::FieldMap,
            ) -> $crate::error::Result<Self> {
                let known = [$($index),*]
                    .into_iter()
                    .filter_map(map)
                    .map(|index| index + 1)
                    .max()
                    .unwrap_or(0);
                table.ensure_known_fields(known, stringify!($name))?;
                Ok(Self {
                    $($field: match map($index) {
                        Some(index) => table.scalar(index)?,
                        None => <$type>::default(),
                    },)*
                })
            }

            pub fn build(&self) -> $crate::formats::flatbuffer::TableBuilder {
                let mut builder = $crate::formats::flatbuffer::TableBuilder::new();
                $(builder.scalar($index, self.$field);)*
                builder
            }

            pub fn build_mapped(
                &self,
                map: $crate::formats::flatbuffer::FieldMap,
            ) -> $crate::error::Result<$crate::formats::flatbuffer::TableBuilder> {
                let mut builder = $crate::formats::flatbuffer::TableBuilder::new();
                $(
                    match map($index) {
                        Some(index) => {
                            builder.scalar(index, self.$field);
                        }
                        None if self.$field != <$type>::default() => {
                            return Err($crate::error::invalid(
                                stringify!($name),
                                concat!("field ", stringify!($field), " does not exist in this game version"),
                            ));
                        }
                        None => {}
                    }
                )*
                Ok(builder)
            }
        }
    };
}

pub(crate) use scalar_table;

pub fn parse_table_list<T>(
    data: &[u8],
    context: &'static str,
    read: impl Fn(&TableRef<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    let root = TableRef::root(data)?;
    root.ensure_known_fields(1, context)?;
    root.tables(0)?.iter().map(read).collect()
}

pub fn build_table_list(tables: Vec<TableBuilder>) -> Vec<u8> {
    let mut root = TableBuilder::new();
    root.tables(0, tables);
    root.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    scalar_table! {
        pub struct Sample {
            0 => small: u8,
            1 => wide: u64,
            2 => signed: i16,
            3 => flag: bool,
            4 => ratio: f32,
        }
    }

    fn sample() -> Sample {
        Sample {
            small: 7,
            wide: 0x1122_3344_5566_7788,
            signed: -3,
            flag: true,
            ratio: 1.5,
        }
    }

    #[test]
    fn scalar_table_round_trips() {
        let bytes = sample().build().finish();
        let table = TableRef::root(&bytes).expect("root");
        assert_eq!(Sample::read(&table), Ok(sample()));
        assert_eq!(Sample::field_size(1), Some(8));
        assert_eq!(Sample::field_size(9), None);
        assert_eq!(
            table.fields_with_nonzero_slack(Sample::field_size),
            Ok(Vec::new())
        );
        let wide = table.field_position(1).expect("field").expect("present");
        assert_eq!(wide % 8, 0);
    }

    #[test]
    fn default_scalars_are_omitted() {
        let bytes = Sample::default().build().finish();
        let table = TableRef::root(&bytes).expect("root");
        assert_eq!(table.present_fields(), Ok(Vec::new()));
        assert_eq!(Sample::read(&table), Ok(Sample::default()));
    }

    #[test]
    fn nested_tables_vectors_and_strings_round_trip() {
        let mut child = TableBuilder::new();
        child.scalar(0, 42u32);
        let mut other = TableBuilder::new();
        other.scalar(1, 9u8);
        let mut root = TableBuilder::new();
        root.scalar(0, 5u8)
            .table(1, child.clone())
            .tables(2, vec![child, other, TableBuilder::new()])
            .scalars(3, &[1u64, 2, 3])
            .scalars(4, &[10i32, -20])
            .string(5, "hello")
            .tables(6, Vec::new());
        let bytes = root.finish();
        let table = TableRef::root(&bytes).expect("root");
        assert_eq!(table.scalar::<u8>(0), Ok(5));
        let nested = table.table(1).expect("table").expect("present");
        assert_eq!(nested.scalar::<u32>(0), Ok(42));
        let list = table.tables(2).expect("tables");
        assert_eq!(list.len(), 3);
        assert_eq!(list[1].scalar::<u8>(1), Ok(9));
        assert_eq!(list[2].present_fields(), Ok(Vec::new()));
        assert_eq!(table.scalars::<u64>(3), Ok(vec![1, 2, 3]));
        let wide = table.vector(3).expect("vector").expect("present");
        assert_eq!(wide.start() % 8, 0);
        assert_eq!(table.scalars::<i32>(4), Ok(vec![10, -20]));
        assert_eq!(table.string(5), Ok(Some("hello".to_string())));
        assert_eq!(table.tables(6).map(|tables| tables.len()), Ok(0));
        assert_eq!(table.tables(9).map(|tables| tables.len()), Ok(0));
        assert_eq!(table.string(9), Ok(None));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut builder = sample().build();
        builder.scalar(8, 1u8);
        let bytes = builder.finish();
        let table = TableRef::root(&bytes).expect("root");
        assert!(Sample::read(&table).is_err());
    }

    #[test]
    fn mapped_fields_skip_missing_slots() {
        fn skip_second(index: usize) -> Option<usize> {
            match index {
                1 => None,
                0 => Some(0),
                other => Some(other - 1),
            }
        }
        let value = Sample {
            wide: 0,
            ..sample()
        };
        let bytes = value.build_mapped(skip_second).expect("build").finish();
        let table = TableRef::root(&bytes).expect("root");
        assert_eq!(table.scalar::<i16>(1), Ok(-3));
        assert_eq!(Sample::read_mapped(&table, skip_second), Ok(value));
        assert!(sample().build_mapped(skip_second).is_err());
    }

    #[test]
    fn malformed_buffers_are_errors() {
        assert!(TableRef::root(&[]).is_err());
        assert!(TableRef::root(&[0xFF, 0xFF, 0, 0]).is_err());
        assert!(TableRef::root(&[4, 0, 0, 0, 0x40, 0, 0, 0]).is_err());
        let mut bytes = sample().build().finish();
        let root = TableRef::root(&bytes).expect("root").position();
        bytes[root] = 0x7F;
        assert!(TableRef::root(&bytes).is_err());
        let mut vector = TableBuilder::new();
        vector.scalars(0, &[1u32, 2, 3]);
        let mut bytes = vector.finish();
        let length = bytes.len() - 16;
        bytes[length] = 0xFF;
        let table = TableRef::root(&bytes).expect("root");
        assert!(table.scalars::<u32>(0).is_err());
    }
}
