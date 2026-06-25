// Codec decode speed (native) for the decoders that compile into the wasm:
// miniz_oxide (gz/DEFLATE), ruzstd (pure-Rust zstd), libzstd (the `zstd` crate).
// Reads <base>.raw/.gz/.zst (from codec_prep.py) and times decode-only.
//   cargo run -p spark-lib --release --example codec_bench -- <dir>/fullchunk <dir>/sh3 ...
use std::hint::black_box;
use std::io::Read;
use std::time::Instant;

fn inflate_gz(b: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec(b).expect("gz")
}
fn decode_ruzstd(b: &[u8]) -> Vec<u8> {
    let mut d = ruzstd::decoding::StreamingDecoder::new(b).expect("ruzstd init");
    let mut o = Vec::new();
    d.read_to_end(&mut o).expect("ruzstd read");
    o
}
fn decode_libzstd(b: &[u8]) -> Vec<u8> {
    zstd::stream::decode_all(b).expect("libzstd")
}

fn bench(f: impl Fn() -> Vec<u8>, raw_len: usize, iters: usize) -> f64 {
    let t = Instant::now();
    for _ in 0..iters {
        black_box(f());
    }
    (raw_len as f64 * iters as f64 / 1e6) / t.elapsed().as_secs_f64()
}

fn main() {
    println!(
        "{:<10}{:>8}{:>12}{:>12}{:>12}{:>13}",
        "blob", "raw KB", "miniz", "ruzstd", "libzstd", "lzstd/ruzs"
    );
    for base in std::env::args().skip(1) {
        let raw = std::fs::read(format!("{base}.raw")).unwrap();
        let gz = std::fs::read(format!("{base}.gz")).unwrap();
        let zs = std::fs::read(format!("{base}.zst")).unwrap();
        assert_eq!(inflate_gz(&gz), raw, "gz mismatch");
        assert_eq!(decode_ruzstd(&zs), raw, "ruzstd mismatch");
        assert_eq!(decode_libzstd(&zs), raw, "libzstd mismatch");
        let it = (300_000_000 / raw.len().max(1)).max(20);
        black_box(decode_libzstd(&zs));
        let gm = bench(|| inflate_gz(&gz), raw.len(), it);
        let rz = bench(|| decode_ruzstd(&zs), raw.len(), it);
        let lz = bench(|| decode_libzstd(&zs), raw.len(), it);
        let name = base.rsplit('/').next().unwrap_or(&base);
        println!(
            "{:<10}{:>8}{:>10.0}MB{:>10.0}MB{:>10.0}MB{:>11.2}x",
            name, raw.len() / 1024, gm, rz, lz, lz / rz
        );
    }
}
