#[path = "../common/mod.rs"]
mod common;

use common::{assemble, file_record, make_targz, sha256_hex, temp_dir, test_config};
use filehub_server::storage::UploadStream;
use std::collections::HashSet;

#[tokio::test]
async fn ingest_discard_and_orphan_gc() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");

    let archive = make_targz("hello.txt", b"hello filehub");
    let expected = sha256_hex(&archive);
    let record = state
        .files
        .ingest(UploadStream::from_bytes(archive.clone()), Some(&expected))
        .await
        .expect("ingest");
    assert_eq!(record.sha256, expected);
    assert_eq!(record.size as usize, archive.len());
    assert!(!record.file_id.0.is_empty());

    let mut reader = state.files.open_read(&record.file_id).await.expect("open");
    use tokio::io::AsyncReadExt;
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).await.expect("read");
    assert_eq!(bytes, archive);

    // 非 tar.gz 不再做解压/格式校验：任意字节按不透明流入库，
    // 完整性仅由 sha256（inject 层必填）与 `expected_sha256` 承担。
    let bad = vec![0u8; 16];
    let opaque = state
        .files
        .ingest(
            UploadStream::from_bytes(bad.clone()),
            Some(&sha256_hex(&bad)),
        )
        .await
        .expect("opaque bytes accepted");
    assert_eq!(opaque.sha256, sha256_hex(&bad));
    // sha 不一致拒绝
    let zeros = "0".repeat(64);
    assert!(
        state
            .files
            .ingest(UploadStream::from_bytes(archive.clone()), Some(&zeros))
            .await
            .is_err()
    );

    // 无引用时 discard 成功，随后 gc 不再处理
    state.files.discard(&record.file_id).await.expect("discard");
    let removed = state.files.gc_orphans(&HashSet::new()).await.expect("gc");
    assert!(!removed.iter().any(|id| *id == record.file_id));
    assert!(state.files.open_read(&record.file_id).await.is_err());
}

#[tokio::test]
async fn gc_keeps_referenced_files() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("test.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");

    let a = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("a", b"a")), None)
        .await
        .expect("a");
    let b = state
        .files
        .ingest(UploadStream::from_bytes(make_targz("b", b"b")), None)
        .await
        .expect("b");
    let keep: HashSet<_> = [a.file_id.clone()].into_iter().collect();
    let removed = state.files.gc_orphans(&keep).await.expect("gc");
    assert_eq!(removed, vec![b.file_id.clone()]);
    assert!(state.files.open_read(&a.file_id).await.is_ok());
    assert!(state.files.open_read(&b.file_id).await.is_err());
}

#[test]
fn file_record_helper_is_stable() {
    let r = file_record("f1", "abc", 42);
    assert_eq!(r.file_id.0, "f1");
}
