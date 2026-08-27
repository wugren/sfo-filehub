#[path = "../common/mod.rs"]
mod common;

use common::{assemble, make_targz, temp_dir, test_config};
use filehub_server::storage::UploadStream;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn ingest_accepts_opaque_bytes_without_decompression() {
    // 服务端不再解压校验：任意字节只要哈希匹配即可入库。
    let opaque = b"definitely not a gzip stream".to_vec();
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("opaque.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let record = state
        .files
        .ingest(
            UploadStream::from_bytes(opaque.clone()),
            Some(&common::sha256_hex(&opaque)),
        )
        .await
        .expect("opaque bytes accepted");
    assert_eq!(record.sha256, common::sha256_hex(&opaque));
}

#[tokio::test]
async fn duplex_split_writes_ingest_like_api_path() {
    let dir = temp_dir().await;
    let config = test_config(&dir, &dir.join("dup.db").to_string_lossy());
    let (state, _db) = assemble(&config).await.expect("assemble");
    let archive = make_targz("a.txt", b"a");

    for split in 1..archive.len() + 3 {
        let (reader, mut writer) = tokio::io::duplex(64 * 1024);
        let store = state.files.clone();
        let handle =
            tokio::spawn(
                async move { store.ingest(UploadStream::from_reader(reader), None).await },
            );
        for part in archive.chunks(split) {
            writer.write_all(part).await.expect("write");
        }
        drop(writer);
        let record = handle
            .await
            .expect("join")
            .unwrap_or_else(|e| panic!("split={split}: {e:?}"));
        assert_eq!(record.size, archive.len() as u64, "split={split}");
    }
}
