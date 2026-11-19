//! Small CPU-only inference runtime for Honeydew's trained drum GRU.
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
struct Tensor { scale: f32, shape: Vec<usize>, q: Vec<i8> }
#[derive(Deserialize)]
struct ModelFile { weights: HashMap<String, Tensor> }
struct Model { tensors: HashMap<String, Tensor> }

static MODEL: OnceLock<Option<Model>> = OnceLock::new();

fn decode_b64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u8;
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace()) {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62, b'/' => 63, b'=' => break, _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 255) as u8);
        }
    }
    Some(out)
}

fn load() -> Option<Model> {
    let packed = decode_b64(include_str!("../assets/ai/drums_gru.json.zlib.b64"))?;
    let json = miniz_oxide::inflate::decompress_to_vec_zlib(&packed).ok()?;
    let file: ModelFile = serde_json::from_slice(&json).ok()?;
    Some(Model { tensors: file.weights })
}

fn model() -> Option<&'static Model> { MODEL.get_or_init(load).as_ref() }

fn tensor<'a>(m: &'a Model, name: &str) -> Option<&'a Tensor> { m.tensors.get(name) }

fn row_dot(t: &Tensor, row: usize, input: &[f32]) -> f32 {
    let width = t.shape[1];
    let start = row * width;
    let sum: f32 = input.iter().enumerate().map(|(i, x)| f32::from(t.q[start + i]) * *x).sum();
    sum * t.scale
}

fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }

/// Autoregressively predicts kick, snare, closed hat and open hat for each sixteenth-note step.
pub fn generate(style: usize, steps: usize, threshold: f32) -> Option<Vec<[bool; 4]>> {
    let m = model()?;
    let wi = tensor(m, "gru.weight_ih_l0")?;
    let wh = tensor(m, "gru.weight_hh_l0")?;
    let bi = tensor(m, "gru.bias_ih_l0")?;
    let bh = tensor(m, "gru.bias_hh_l0")?;
    let wo = tensor(m, "head.weight")?;
    let bo = tensor(m, "head.bias")?;
    let hidden = 48usize;
    let style = style.min(4);
    let mut h = vec![0.0f32; hidden];
    let mut previous = [0.0f32; 4];
    let mut result = Vec::with_capacity(steps);

    for _ in 0..steps {
        let mut x = [0.0f32; 9];
        x[..4].copy_from_slice(&previous);
        x[4 + style] = 1.0;
        let mut gates = [0.0f32; 144];
        for r in 0..144 {
            gates[r] = row_dot(wi, r, &x) + f32::from(bi.q[r]) * bi.scale
                + row_dot(wh, r, &h) + f32::from(bh.q[r]) * bh.scale;
        }
        let mut next = vec![0.0f32; hidden];
        for i in 0..hidden {
            let reset = sigmoid(gates[i]);
            let update = sigmoid(gates[hidden + i]);
            let candidate_recurrent = row_dot(wh, 2 * hidden + i, &h)
                + f32::from(bh.q[2 * hidden + i]) * bh.scale;
            let candidate_input = row_dot(wi, 2 * hidden + i, &x)
                + f32::from(bi.q[2 * hidden + i]) * bi.scale;
            let candidate = (candidate_input + reset * candidate_recurrent).tanh();
            next[i] = (1.0 - update) * candidate + update * h[i];
        }
        h = next;
        let mut hits = [false; 4];
        for i in 0..4 {
            let logit = row_dot(wo, i, &h) + f32::from(bo.q[i]) * bo.scale;
            hits[i] = sigmoid(logit) >= threshold;
            previous[i] = if hits[i] { 1.0 } else { 0.0 };
        }
        result.push(hits);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trained_model_generates_each_style() {
        for style in 0..5 {
            let pattern = generate(style, 64, 0.5).expect("embedded model loads");
            assert_eq!(pattern.len(), 64);
            assert!(pattern.iter().any(|step| step.iter().any(|hit| *hit)));
        }
    }
}
