use randomon_core::{OutputFile, write_directory_output};

fn file(path: &str, bytes: &[u8]) -> OutputFile {
    OutputFile {
        path: path.to_owned(),
        bytes: bytes.to_vec(),
    }
}

#[test]
fn writes_layered_fs_and_purges_the_title_folder() {
    let root =
        std::env::temp_dir().join(format!("randomon-directory-output-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("TITLE/romfs/stale")).unwrap();
    std::fs::write(root.join("TITLE/romfs/stale/old.bin"), b"old").unwrap();
    std::fs::write(root.join("keep.txt"), b"keep").unwrap();
    let files = [
        file("TITLE/romfs/", b""),
        file("TITLE/code.bin", b"code"),
        file("TITLE/romfs/a/0/1", b"garc"),
    ];
    write_directory_output(&root, &files).unwrap();
    assert!(!root.join("TITLE/romfs/stale").exists());
    assert_eq!(std::fs::read(root.join("TITLE/code.bin")).unwrap(), b"code");
    assert_eq!(
        std::fs::read(root.join("TITLE/romfs/a/0/1")).unwrap(),
        b"garc"
    );
    assert!(root.join("keep.txt").exists());
    std::fs::remove_dir_all(&root).unwrap();
}
