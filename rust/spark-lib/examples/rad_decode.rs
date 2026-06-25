// Decode a .rad through the fork's RadDecoder and report a content hash + decode
// time. Used to prove gz/zstd decode EQUIVALENCE (identical hash for a twin pair)
// and to benchmark decode speed of the codec. The hash folds every decoded
// property value (post-dequantization), so an identical hash ⇒ byte-identical splats.
//
//   cargo run -p spark-lib --release --example rad_decode -- <a.rad> [b.rad ...]
use std::collections::BTreeMap;
use std::io::Read;
use std::time::Instant;

use spark_lib::decoder::{ChunkReceiver, SplatInit, SplatProps, SplatReceiver};
use spark_lib::rad::RadDecoder;

struct HashRecv {
    h: u64,
    decoded: usize,
    calls: BTreeMap<&'static str, usize>,
}
impl HashRecv {
    fn new() -> Self {
        Self { h: 0xcbf29ce484222325, decoded: 0, calls: BTreeMap::new() }
    }
    fn mix(&mut self, b: &[u8]) {
        for &x in b {
            self.h = (self.h ^ x as u64).wrapping_mul(0x100000001b3); // FNV-1a 64
        }
    }
    fn mix_f32(&mut self, v: &[f32]) {
        for &f in v {
            self.mix(&f.to_bits().to_le_bytes());
        }
    }
    fn tag(&mut self, t: &'static str) {
        *self.calls.entry(t).or_insert(0) += 1;
    }
}
impl SplatReceiver for HashRecv {
    fn init_splats(&mut self, _init: &SplatInit) -> anyhow::Result<()> {
        Ok(())
    }
    fn set_batch(&mut self, _b: usize, _c: usize, _p: &SplatProps) {
        self.tag("batch");
    }
    fn set_center(&mut self, _b: usize, c: usize, v: &[f32]) {
        self.tag("center");
        self.decoded += c;
        self.mix_f32(v);
    }
    fn set_opacity(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("opacity");
        self.mix_f32(v);
    }
    fn set_rgb(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("rgb");
        self.mix_f32(v);
    }
    fn set_rgba(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("rgba");
        self.mix_f32(v);
    }
    fn set_scale(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("scale");
        self.mix_f32(v);
    }
    fn set_quat(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("quat");
        self.mix_f32(v);
    }
    fn set_sh(&mut self, _b: usize, _c: usize, a: &[f32], d: &[f32], e: &[f32]) {
        self.tag("sh");
        self.mix_f32(a);
        self.mix_f32(d);
        self.mix_f32(e);
    }
    fn set_sh1(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("sh1");
        self.mix_f32(v);
    }
    fn set_sh2(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("sh2");
        self.mix_f32(v);
    }
    fn set_sh3(&mut self, _b: usize, _c: usize, v: &[f32]) {
        self.tag("sh3");
        self.mix_f32(v);
    }
    fn set_child_count(&mut self, _b: usize, _c: usize, v: &[u16]) {
        self.tag("child_count");
        for &x in v {
            self.mix(&x.to_le_bytes());
        }
    }
    fn set_child_start(&mut self, _b: usize, _c: usize, v: &[usize]) {
        self.tag("child_start");
        for &x in v {
            self.mix(&(x as u64).to_le_bytes());
        }
    }
}

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let mut dec = RadDecoder::new(HashRecv::new());
        let mut f = std::io::BufReader::new(std::fs::File::open(&path)?);
        let mut buf = vec![0u8; 8 << 20];
        let mut total = 0usize;
        let t = Instant::now();
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            dec.push(&buf[..n])?;
            total += n;
        }
        dec.finish()?;
        let el = t.elapsed();
        let r = dec.into_splats();
        let name = path.rsplit('/').next().unwrap_or(&path);
        println!(
            "{:<34} decoded={:>9} hash={:016x} time={:>8.1}ms {:>6.1}MB/s calls={:?}",
            name,
            r.decoded,
            r.h,
            el.as_secs_f64() * 1000.0,
            (total as f64 / 1e6) / el.as_secs_f64(),
            r.calls
        );
    }
    Ok(())
}
