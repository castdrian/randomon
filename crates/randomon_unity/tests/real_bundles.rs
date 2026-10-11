use std::path::{Path, PathBuf};

use randomon_unity::{AssetFile, BundleWriteOptions, read_object, write_object};

const ROMFS_ENV: &str = "RANDOMON_BDSP_ROMFS";

const BUNDLES: &[&str] = &[
    "StreamingAssets/AssetAssistant/Dpr/masterdatas",
    "StreamingAssets/AssetAssistant/Dpr/ev_script",
    "StreamingAssets/AssetAssistant/Dpr/scriptableobjects/gamesettings",
    "StreamingAssets/AssetAssistant/Pml/personal_masterdatas",
    "StreamingAssets/AssetAssistant/UnderGround/data/ugdata",
    "StreamingAssets/AssetAssistant/Message/english",
    "globalgamemanagers",
];

fn data_dir() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var(ROMFS_ENV).ok()?);
    let data = root.join("Data");
    Some(if data.is_dir() { data } else { root })
}

fn check_bundle(path: &Path) {
    let original = std::fs::read(path).expect("bundle readable");
    let asset = AssetFile::parse(&original).expect("bundle parses");
    let nodes = asset.decompressed_nodes();
    for file in asset.serialized_files() {
        let rebuilt = file.rebuild();
        let original = file.to_bytes();
        let first_difference = rebuilt.iter().zip(&original).position(|(a, b)| a != b);
        assert!(
            rebuilt == original,
            "relayout is lossless for {}: lengths {} vs {}, first difference {first_difference:?}",
            path.display(),
            rebuilt.len(),
            original.len()
        );
        for object in &file.objects {
            let Some(tree) = file.type_tree(object) else {
                continue;
            };
            let value = read_object(tree, &object.data, file.endian).unwrap_or_else(|error| {
                panic!("{} object {}: {error}", path.display(), object.path_id)
            });
            let encoded = write_object(tree, &value, file.endian).expect("encodes");
            assert!(
                encoded == object.data,
                "{} object {} round trips",
                path.display(),
                object.path_id
            );
        }
    }
    let rewritten = asset
        .to_bytes(&BundleWriteOptions::default())
        .expect("writes");
    let reparsed = AssetFile::parse(&rewritten).expect("rewritten bundle parses");
    assert!(
        reparsed.decompressed_nodes() == nodes,
        "{} decompressed content is identical",
        path.display()
    );
}

#[test]
fn real_bundles_round_trip() {
    let Some(data) = data_dir() else {
        eprintln!("{ROMFS_ENV} is not set; skipping real bundle round trips");
        return;
    };
    for bundle in BUNDLES {
        check_bundle(&data.join(bundle));
    }
}
