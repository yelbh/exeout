use zstd::bulk::{compress, decompress};
use anyhow::Result;

pub fn compress_data(data: &[u8], level: i32) -> Result<Vec<u8>> {
    compress(data, level).map_err(|e| anyhow::anyhow!("Compression error: {}", e))
}

pub fn decompress_data(data: &[u8], original_size: usize) -> Result<Vec<u8>> {
    decompress(data, original_size).map_err(|e| anyhow::anyhow!("Decompression error: {}", e))
}
