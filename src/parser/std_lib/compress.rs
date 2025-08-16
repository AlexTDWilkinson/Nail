use flate2::write::{GzEncoder, GzDecoder};
use flate2::Compression;
use std::io::Write;

/// Compress a string using gzip
pub async fn gzip_compress(data: String) -> Result<String, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes())
        .map_err(|e| format!("Failed to compress data: {}", e))?;
    
    let compressed = encoder.finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;
    
    // Convert to base64 for safe string representation
    Ok(base64::encode(compressed))
}

/// Decompress a gzipped string
pub async fn gzip_decompress(data: String) -> Result<String, String> {
    // Decode from base64
    let compressed = base64::decode(&data)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;
    
    let mut decoder = GzDecoder::new(Vec::new());
    decoder.write_all(&compressed)
        .map_err(|e| format!("Failed to decompress data: {}", e))?;
    
    let decompressed = decoder.finish()
        .map_err(|e| format!("Failed to finish decompression: {}", e))?;
    
    String::from_utf8(decompressed)
        .map_err(|e| format!("Decompressed data is not valid UTF-8: {}", e))
}