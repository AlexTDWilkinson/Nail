use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use flate2::write::{GzEncoder, GzDecoder};
use flate2::Compression;
use std::io::Write;

/// Compress a string using gzip
pub fn gzip_compress(data: String) -> Result<String, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes())
        .map_err(|e| format!("compress_gzip: could not compress the data: {}", e))?;

    let compressed = encoder.finish()
        .map_err(|e| format!("compress_gzip: could not finish compression: {}", e))?;
    
    // Convert to base64 for safe string representation
    Ok(STANDARD.encode(compressed))
}

/// Decompress a gzipped string
pub fn gzip_decompress(data: String) -> Result<String, String> {
    // Decode from base64
    let compressed = STANDARD.decode(&data)
        .map_err(|e| format!("compress_gunzip: the input is not valid base64: {}", e))?;

    let mut decoder = GzDecoder::new(Vec::new());
    decoder.write_all(&compressed)
        .map_err(|e| format!("compress_gunzip: could not decompress the data: {}", e))?;

    let decompressed = decoder.finish()
        .map_err(|e| format!("compress_gunzip: could not finish decompression: {}", e))?;

    String::from_utf8(decompressed)
        .map_err(|e| format!("compress_gunzip: the decompressed data is not valid UTF-8: {}", e))
}

/// Zstd-compress a string and return it base64-encoded. Level 3 - zstd's own
/// default, fast with a good ratio.
#[cfg(feature = "compress")]
pub fn zstd_compress(data: String) -> Result<String, String> {
    let compressed = zstd::stream::encode_all(data.as_bytes(), 3).map_err(|e| format!("compress_zstd: could not compress the data: {}", e))?;
    return Ok(STANDARD.encode(compressed));
}

/// Decompress a base64-encoded zstd string back to the original text.
#[cfg(feature = "compress")]
pub fn zstd_decompress(data: String) -> Result<String, String> {
    let compressed = STANDARD.decode(&data).map_err(|e| format!("compress_unzstd: the input is not valid base64: {}", e))?;
    let decompressed = zstd::stream::decode_all(&compressed[..]).map_err(|e| format!("compress_unzstd: could not decompress the data: {}", e))?;
    return String::from_utf8(decompressed).map_err(|e| format!("compress_unzstd: the decompressed data is not valid UTF-8: {}", e));
}

/// Brotli-compress a string and return it base64-encoded. Quality 5 - the
/// balanced setting web servers use for on-the-fly compression.
#[cfg(feature = "compress")]
pub fn brotli_compress(data: String) -> Result<String, String> {
    let params = brotli::enc::BrotliEncoderParams { quality: 5, ..Default::default() };
    let mut compressed = Vec::new();
    brotli::BrotliCompress(&mut data.as_bytes(), &mut compressed, &params).map_err(|e| format!("compress_brotli: could not compress the data: {}", e))?;
    return Ok(STANDARD.encode(compressed));
}

/// Decompress a base64-encoded brotli string back to the original text.
#[cfg(feature = "compress")]
pub fn brotli_decompress(data: String) -> Result<String, String> {
    let compressed = STANDARD.decode(&data).map_err(|e| format!("compress_unbrotli: the input is not valid base64: {}", e))?;
    let mut decompressed = Vec::new();
    brotli::BrotliDecompress(&mut &compressed[..], &mut decompressed).map_err(|e| format!("compress_unbrotli: could not decompress the data: {}", e))?;
    return String::from_utf8(decompressed).map_err(|e| format!("compress_unbrotli: the decompressed data is not valid UTF-8: {}", e));
}

#[cfg(all(test, feature = "compress"))]
mod modern_compression_tests {
    use super::*;

    #[test]
    fn zstd_round_trips() {
        let text = "the same phrase over and over ".repeat(50);
        let packed = zstd_compress(text.clone()).unwrap();
        assert!(packed.len() < text.len());
        assert_eq!(zstd_decompress(packed).unwrap(), text);
        assert!(zstd_decompress("AAAA".to_string()).unwrap_err().contains("could not decompress"));
    }

    #[test]
    fn brotli_round_trips() {
        let text = "the same phrase over and over ".repeat(50);
        let packed = brotli_compress(text.clone()).unwrap();
        assert!(packed.len() < text.len());
        assert_eq!(brotli_decompress(packed).unwrap(), text);
        assert!(brotli_decompress("not base64!".to_string()).unwrap_err().contains("not valid base64"));
    }
}