use crate::error::{Result, UnityError};
use crate::stream::{ByteReader, ByteWriter, Endian};
use crate::typetree::TypeTree;
use crate::value::{Fields, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeShape {
    Primitive,
    String,
    TypelessData,
    Map,
    Vector,
    Class,
}

fn shape(tree: &TypeTree, index: usize) -> NodeShape {
    let node = &tree.nodes[index];
    match node.type_name.as_str() {
        "SInt8" | "UInt8" | "char" | "short" | "SInt16" | "UInt16" | "unsigned short" | "int"
        | "SInt32" | "UInt32" | "unsigned int" | "Type*" | "long long" | "SInt64" | "UInt64"
        | "unsigned long long" | "FileSize" | "float" | "double" | "bool" => NodeShape::Primitive,
        "string" => NodeShape::String,
        "map" => NodeShape::Map,
        "TypelessData" => NodeShape::TypelessData,
        _ if tree.nodes.get(index + 1).is_some_and(|next| {
            next.type_name == "Array" && index + 1 < tree.subtree_end(index)
        }) =>
        {
            NodeShape::Vector
        }
        _ => NodeShape::Class,
    }
}

fn next_aligns(tree: &TypeTree, index: usize) -> bool {
    tree.nodes.get(index + 1).is_some_and(|node| node.aligns())
}

fn element_index(tree: &TypeTree, vector_index: usize) -> Result<usize> {
    let array_index = vector_index + 1;
    tree.children(array_index).nth(1).ok_or_else(|| {
        UnityError::InvalidTypeTree(format!(
            "array {} has no element node",
            tree.nodes[vector_index].name
        ))
    })
}

fn map_pair_indices(tree: &TypeTree, map_index: usize) -> Result<(usize, usize)> {
    let pair = element_index(tree, map_index)?;
    let mut children = tree.children(pair);
    match (children.next(), children.next()) {
        (Some(first), Some(second)) => Ok((first, second)),
        _ => Err(UnityError::InvalidTypeTree(
            "map pair needs two children".into(),
        )),
    }
}

fn read_count(reader: &mut ByteReader<'_>) -> Result<usize> {
    let offset = reader.position();
    let count = reader.i32()?;
    usize::try_from(count).map_err(|_| UnityError::UnexpectedEof { offset, needed: 0 })
}

pub fn read_object(tree: &TypeTree, data: &[u8], endian: Endian) -> Result<Value> {
    if tree.nodes.is_empty() {
        return Err(UnityError::InvalidTypeTree("empty type tree".into()));
    }
    let mut reader = ByteReader::new(data, endian);
    let value = read_class(tree, 0, &mut reader)?;
    if reader.position() != data.len() {
        return Err(UnityError::SizeMismatch {
            read: reader.position(),
            expected: data.len(),
        });
    }
    Ok(value)
}

fn read_class(tree: &TypeTree, index: usize, reader: &mut ByteReader<'_>) -> Result<Value> {
    let fields = tree
        .children(index)
        .map(|child| {
            Ok((
                tree.nodes[child].name.clone(),
                read_value(tree, child, reader)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::Struct(Fields(fields)))
}

fn read_primitive(type_name: &str, reader: &mut ByteReader<'_>) -> Result<Value> {
    Ok(match type_name {
        "SInt8" => Value::I8(reader.i8()?),
        "UInt8" | "char" => Value::U8(reader.u8()?),
        "short" | "SInt16" => Value::I16(reader.i16()?),
        "UInt16" | "unsigned short" => Value::U16(reader.u16()?),
        "int" | "SInt32" => Value::I32(reader.i32()?),
        "UInt32" | "unsigned int" | "Type*" => Value::U32(reader.u32()?),
        "long long" | "SInt64" => Value::I64(reader.i64()?),
        "UInt64" | "unsigned long long" | "FileSize" => Value::U64(reader.u64()?),
        "float" => Value::F32(reader.f32()?),
        "double" => Value::F64(reader.f64()?),
        "bool" => Value::Bool(reader.u8()? != 0),
        other => {
            return Err(UnityError::InvalidTypeTree(format!(
                "not a primitive: {other}"
            )));
        }
    })
}

fn read_value(tree: &TypeTree, index: usize, reader: &mut ByteReader<'_>) -> Result<Value> {
    let node = &tree.nodes[index];
    let mut align = node.aligns();
    let value = match shape(tree, index) {
        NodeShape::Primitive => read_primitive(&node.type_name, reader)?,
        NodeShape::String => {
            let length = read_count(reader)?;
            let start = reader.position();
            let bytes = reader.take(length)?;
            align |= length > 0 || next_aligns(tree, index);
            Value::String(
                String::from_utf8(bytes.to_vec()).map_err(|_| UnityError::InvalidString(start))?,
            )
        }
        NodeShape::TypelessData => {
            let length = read_count(reader)?;
            Value::Bytes(reader.take(length)?.to_vec())
        }
        NodeShape::Map => {
            align |= next_aligns(tree, index);
            let (first, second) = map_pair_indices(tree, index)?;
            let length = read_count(reader)?;
            let entries = (0..length)
                .map(|_| {
                    Ok((
                        read_value(tree, first, reader)?,
                        read_value(tree, second, reader)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            Value::Map(entries)
        }
        NodeShape::Vector => {
            align |= next_aligns(tree, index);
            let element = element_index(tree, index)?;
            let length = read_count(reader)?;
            let items = (0..length)
                .map(|_| read_value(tree, element, reader))
                .collect::<Result<Vec<_>>>()?;
            Value::Array(items)
        }
        NodeShape::Class => read_class(tree, index, reader)?,
    };
    if align {
        reader.align(4);
    }
    Ok(value)
}

pub fn write_object(tree: &TypeTree, value: &Value, endian: Endian) -> Result<Vec<u8>> {
    if tree.nodes.is_empty() {
        return Err(UnityError::InvalidTypeTree("empty type tree".into()));
    }
    let mut writer = ByteWriter::new(endian);
    write_class(tree, 0, value, &mut writer)?;
    Ok(writer.into_bytes())
}

fn mismatch(tree: &TypeTree, index: usize, value: &Value) -> UnityError {
    let node = &tree.nodes[index];
    UnityError::ValueMismatch(format!(
        "{} {} cannot hold {}",
        node.type_name,
        node.name,
        value.kind()
    ))
}

fn write_class(
    tree: &TypeTree,
    index: usize,
    value: &Value,
    writer: &mut ByteWriter,
) -> Result<()> {
    let fields = value
        .as_struct()
        .ok_or_else(|| mismatch(tree, index, value))?;
    for child in tree.children(index) {
        let name = &tree.nodes[child].name;
        let field = fields
            .get(name)
            .ok_or_else(|| UnityError::FieldNotFound(name.clone()))?;
        write_value(tree, child, field, writer)?;
    }
    Ok(())
}

fn write_primitive(
    tree: &TypeTree,
    index: usize,
    value: &Value,
    writer: &mut ByteWriter,
) -> Result<()> {
    let type_name = tree.nodes[index].type_name.as_str();
    let error = || mismatch(tree, index, value);
    if matches!(type_name, "float" | "double") {
        let number = value.as_f64().ok_or_else(error)?;
        if type_name == "float" {
            writer.f32(number as f32);
        } else {
            writer.f64(number);
        }
        return Ok(());
    }
    let number = value.as_i128().ok_or_else(error)?;
    let out_of_range = || UnityError::OutOfRange {
        value: number.to_string(),
        kind: match type_name {
            "SInt8" => "SInt8",
            "UInt8" | "char" => "UInt8",
            "short" | "SInt16" => "SInt16",
            "UInt16" | "unsigned short" => "UInt16",
            "int" | "SInt32" => "SInt32",
            "UInt32" | "unsigned int" | "Type*" => "UInt32",
            "long long" | "SInt64" => "SInt64",
            "bool" => "bool",
            _ => "UInt64",
        },
    };
    match type_name {
        "SInt8" => writer.i8(i8::try_from(number).map_err(|_| out_of_range())?),
        "UInt8" | "char" => writer.u8(u8::try_from(number).map_err(|_| out_of_range())?),
        "short" | "SInt16" => writer.i16(i16::try_from(number).map_err(|_| out_of_range())?),
        "UInt16" | "unsigned short" => {
            writer.u16(u16::try_from(number).map_err(|_| out_of_range())?)
        }
        "int" | "SInt32" => writer.i32(i32::try_from(number).map_err(|_| out_of_range())?),
        "UInt32" | "unsigned int" | "Type*" => {
            writer.u32(u32::try_from(number).map_err(|_| out_of_range())?)
        }
        "long long" | "SInt64" => writer.i64(i64::try_from(number).map_err(|_| out_of_range())?),
        "bool" => writer.u8(u8::from(number != 0)),
        _ => writer.u64(u64::try_from(number).map_err(|_| out_of_range())?),
    }
    Ok(())
}

fn write_value(
    tree: &TypeTree,
    index: usize,
    value: &Value,
    writer: &mut ByteWriter,
) -> Result<()> {
    let mut align = tree.nodes[index].aligns();
    match shape(tree, index) {
        NodeShape::Primitive => write_primitive(tree, index, value, writer)?,
        NodeShape::String => {
            let text = value.as_str().ok_or_else(|| mismatch(tree, index, value))?;
            writer.i32(text.len() as i32);
            writer.bytes(text.as_bytes());
            align = true;
        }
        NodeShape::TypelessData => {
            let Value::Bytes(bytes) = value else {
                return Err(mismatch(tree, index, value));
            };
            writer.i32(bytes.len() as i32);
            writer.bytes(bytes);
        }
        NodeShape::Map => {
            align |= next_aligns(tree, index);
            let Value::Map(entries) = value else {
                return Err(mismatch(tree, index, value));
            };
            let (first, second) = map_pair_indices(tree, index)?;
            writer.i32(entries.len() as i32);
            for (key, entry) in entries {
                write_value(tree, first, key, writer)?;
                write_value(tree, second, entry, writer)?;
            }
        }
        NodeShape::Vector => {
            align |= next_aligns(tree, index);
            let items = value
                .as_array()
                .ok_or_else(|| mismatch(tree, index, value))?;
            let element = element_index(tree, index)?;
            writer.i32(items.len() as i32);
            for item in items {
                write_value(tree, element, item, writer)?;
            }
        }
        NodeShape::Class => write_class(tree, index, value, writer)?,
    }
    if align {
        writer.align(4);
    }
    Ok(())
}
