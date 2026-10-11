use randomon_unity::{
    AssetFile, Bundle, BundleNode, BundleWriteOptions, CLASS_MONO_BEHAVIOUR, Compression,
    ObjectInfo, SerializedFile, SerializedType, TypeTree, TypeTreeNode, Value, compress,
    decompress, write_object,
};

struct TreeBuilder {
    nodes: Vec<TypeTreeNode>,
    strings: Vec<u8>,
}

impl TreeBuilder {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            strings: Vec::new(),
        }
    }

    fn intern(&mut self, text: &str) -> u32 {
        let offset = self.strings.len() as u32;
        self.strings.extend_from_slice(text.as_bytes());
        self.strings.push(0);
        offset
    }

    fn node(
        mut self,
        level: u8,
        type_name: &str,
        name: &str,
        byte_size: i32,
        meta_flag: u32,
    ) -> Self {
        let type_offset = self.intern(type_name);
        let name_offset = self.intern(name);
        let index = self.nodes.len() as i32;
        self.nodes.push(TypeTreeNode {
            version: 1,
            level,
            type_flags: u8::from(type_name == "Array"),
            type_offset,
            name_offset,
            byte_size,
            index,
            meta_flag,
            ref_type_hash: 0,
            type_name: type_name.to_owned(),
            name: name.to_owned(),
        });
        self
    }

    fn build(self) -> TypeTree {
        TypeTree::new(self.nodes, self.strings)
    }
}

fn sample_tree() -> TypeTree {
    TreeBuilder::new()
        .node(0, "MonoBehaviour", "Base", -1, 0x8000)
        .node(1, "UInt8", "m_Enabled", 1, 0x4101)
        .node(1, "string", "m_Name", -1, 0x8000)
        .node(2, "Array", "Array", -1, 0x4001)
        .node(3, "int", "size", 4, 0x1)
        .node(3, "char", "data", 1, 0x1)
        .node(1, "vector", "ar", -1, 0x8000)
        .node(2, "Array", "Array", -1, 0x4000)
        .node(3, "int", "size", 4, 0)
        .node(3, "UInt16", "data", 2, 0)
        .node(1, "float", "speed", 4, 0)
        .node(1, "SInt64", "id", 8, 0)
        .build()
}

fn sample_value(name: &str, moves: &[u16]) -> Value {
    Value::Struct(randomon_unity::Fields(vec![
        ("m_Enabled".into(), Value::U8(1)),
        ("m_Name".into(), Value::String(name.into())),
        (
            "ar".into(),
            Value::Array(moves.iter().map(|&value| Value::U16(value)).collect()),
        ),
        ("speed".into(), Value::F32(1.5)),
        ("id".into(), Value::I64(-7)),
    ]))
}

fn sample_file() -> SerializedFile {
    let tree = sample_tree();
    let first = write_object(
        &tree,
        &sample_value("Table", &[1, 33, 5]),
        randomon_unity::Endian::Little,
    )
    .expect("encodes");
    let second = write_object(
        &tree,
        &sample_value("Other", &[]),
        randomon_unity::Endian::Little,
    )
    .expect("encodes");
    let kind = SerializedType {
        class_id: CLASS_MONO_BEHAVIOUR,
        is_stripped: false,
        script_type_index: 0,
        script_id: Some([3; 16]),
        old_type_hash: [9; 16],
        type_tree: Some(tree),
        dependencies: Vec::new(),
        reference_names: None,
    };
    let objects = vec![
        ObjectInfo {
            path_id: 42,
            byte_start: 0,
            type_index: 0,
            data: first,
        },
        ObjectInfo {
            path_id: -5,
            byte_start: 1,
            type_index: 0,
            data: second,
        },
    ];
    SerializedFile::from_parts(21, "2019.4.27f1", 38, vec![kind], objects)
}

fn sample_bundle(compression: Compression) -> Vec<u8> {
    let bundle = Bundle {
        format_version: 7,
        player_version: "5.x.x".into(),
        engine_version: "2019.4.27f1".into(),
        header_flags: 0x43,
        blocks: Vec::new(),
        nodes: vec![
            BundleNode {
                path: "CAB-test".into(),
                flags: 4,
                data: sample_file().rebuild(),
            },
            BundleNode {
                path: "CAB-test.resS".into(),
                flags: 0,
                data: (0..300_000u32).map(|value| (value % 251) as u8).collect(),
            },
        ],
    };
    let options = BundleWriteOptions {
        data_compression: compression,
        block_info_compression: Compression::Lz4,
        block_size: 0x20000,
    };
    bundle.to_bytes(&options).expect("bundle writes")
}

#[test]
fn serialized_file_round_trips() {
    let bytes = sample_file().rebuild();
    let parsed = SerializedFile::parse(&bytes).expect("parses");
    assert_eq!(parsed.to_bytes(), bytes);
    assert_eq!(parsed.rebuild(), bytes);
    assert_eq!(
        parsed.read_value(42).expect("reads"),
        sample_value("Table", &[1, 33, 5])
    );
    assert_eq!(
        parsed.read_value(-5).expect("reads"),
        sample_value("Other", &[])
    );
}

#[test]
fn edited_values_survive_a_bundle_round_trip() {
    for compression in [Compression::None, Compression::Lz4, Compression::Lzma] {
        let mut asset = AssetFile::parse(&sample_bundle(compression)).expect("bundle parses");
        let mut value = asset.read_value(42).expect("reads");
        value
            .field_mut("ar")
            .expect("ar")
            .as_array_mut()
            .expect("array")
            .extend([Value::U16(400), Value::U16(7)]);
        value
            .field_mut("m_Name")
            .expect("name")
            .set_string("Renamed table")
            .expect("string");
        value.set_float_field("speed", 3.1).expect("float");
        asset.write_value(42, &value).expect("writes");
        assert!(asset.is_modified());
        let rewritten = asset
            .to_bytes(&BundleWriteOptions::default())
            .expect("rewrites");
        let reparsed = AssetFile::parse(&rewritten).expect("reparses");
        assert_eq!(reparsed.read_value(42).expect("reads"), value);
        assert_eq!(
            reparsed.read_value(-5).expect("reads"),
            sample_value("Other", &[])
        );
        assert_eq!(
            reparsed.decompressed_nodes()[1],
            asset.decompressed_nodes()[1]
        );
    }
}

#[test]
fn out_of_range_values_are_rejected() {
    let mut value = Value::U16(1);
    assert!(value.set_int(70_000).is_err());
    assert!(value.set_int(65_535).is_ok());
    let tree = sample_tree();
    let wrong = Value::Struct(randomon_unity::Fields(vec![(
        "m_Enabled".into(),
        Value::U8(1),
    )]));
    assert!(write_object(&tree, &wrong, randomon_unity::Endian::Little).is_err());
}

#[test]
fn block_compression_round_trips() {
    let data: Vec<u8> = (0..50_000u32).map(|value| (value * 7 % 13) as u8).collect();
    for compression in [
        Compression::None,
        Compression::Lz4,
        Compression::Lz4Hc,
        Compression::Lzma,
    ] {
        let packed = compress(compression, &data).expect("compresses");
        assert_eq!(
            decompress(compression, &packed, data.len()).expect("decompresses"),
            data
        );
    }
}
