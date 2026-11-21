//! Pure-Rust inference for Honeydew's small pitch-only NoteLM checkpoint.
//!
//! The checkpoint was trained as Embedding(128, 96) -> LSTM(96, 192) -> Linear(192, 128).
//! Its raw little-endian f32 layout follows the state-dict tensors in embedding, LSTM, then output-layer order. Timing, harmony,
//! velocity and instrumentation remain the responsibility of the arrangement generator.

const VOCAB: usize = 128;
const EMBED: usize = 96;
const HIDDEN: usize = 192;
const GATES: usize = HIDDEN * 4;
const WEIGHT_FLOATS: usize =
    VOCAB * EMBED + GATES * EMBED + GATES * HIDDEN + GATES + GATES
        + VOCAB * HIDDEN + VOCAB;
pub const WEIGHT_BYTES: usize = WEIGHT_FLOATS * 4;

/// Returns the model from the APK assets on Android or the repository assets folder elsewhere.
/// A missing model is non-fatal: the arrangement generator keeps its deterministic fallback.
pub fn load_model_weights() -> Option<Vec<u8>> {
    #[cfg(target_os = "android")]
    {
        return crate::android::asset_bytes("models/honeydew_note_lm.f32", WEIGHT_BYTES);
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        let bytes = std::fs::read("assets/honeydew_note_lm.f32").ok()?;
        (bytes.len() == WEIGHT_BYTES).then_some(bytes)
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

struct Weights {
    values: Vec<f32>,
}

impl Weights {
    fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != WEIGHT_BYTES {
            return None;
        }
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        if values.iter().any(|value| !value.is_finite()) {
            return None;
        }
        Some(Self { values })
    }
}

#[inline]
fn sigmoid(value: f32) -> f32 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn next_logits(
    weights: &[f32],
    pitch: u8,
    hidden: &mut [f32; HIDDEN],
    cell: &mut [f32; HIDDEN],
) -> [f32; VOCAB] {
    let mut offset = 0;
    let embedding = &weights[offset + usize::from(pitch) * EMBED..][..EMBED];
    offset += VOCAB * EMBED;
    let w_ih = &weights[offset..offset + GATES * EMBED];
    offset += GATES * EMBED;
    let w_hh = &weights[offset..offset + GATES * HIDDEN];
    offset += GATES * HIDDEN;
    let b_ih = &weights[offset..offset + GATES];
    offset += GATES;
    let b_hh = &weights[offset..offset + GATES];
    offset += GATES;
    let w_head = &weights[offset..offset + VOCAB * HIDDEN];
    offset += VOCAB * HIDDEN;
    let b_head = &weights[offset..offset + VOCAB];

    let mut gates = [0.0_f32; GATES];
    for gate in 0..GATES {
        let mut value = b_ih[gate] + b_hh[gate];
        let input_row = &w_ih[gate * EMBED..(gate + 1) * EMBED];
        let hidden_row = &w_hh[gate * HIDDEN..(gate + 1) * HIDDEN];
        for i in 0..EMBED {
            value += input_row[i] * embedding[i];
        }
        for i in 0..HIDDEN {
            value += hidden_row[i] * hidden[i];
        }
        gates[gate] = value;
    }

    // PyTorch's LSTM gate order is input, forget, cell candidate, output.
    for i in 0..HIDDEN {
        let input_gate = sigmoid(gates[i]);
        let forget_gate = sigmoid(gates[HIDDEN + i]);
        let candidate = gates[2 * HIDDEN + i].tanh();
        let output_gate = sigmoid(gates[3 * HIDDEN + i]);
        cell[i] = forget_gate * cell[i] + input_gate * candidate;
        hidden[i] = output_gate * cell[i].tanh();
    }

    let mut logits = [0.0_f32; VOCAB];
    for pitch in 0..VOCAB {
        let row = &w_head[pitch * HIDDEN..(pitch + 1) * HIDDEN];
        let mut value = b_head[pitch];
        for i in 0..HIDDEN {
            value += row[i] * hidden[i];
        }
        logits[pitch] = value;
    }
    logits
}

/// Generates scale-constrained MIDI pitches from the trained next-note model.
/// The model predicts pitch only; the caller supplies note count and handles note timing.
pub fn generate_pitch_sequence(
    bytes: &[u8],
    context: &[u8],
    count: usize,
    root: u8,
    minor: bool,
    seed: u64,
) -> Option<Vec<u8>> {
    let parsed = Weights::parse(bytes)?;
    let weights = &parsed.values;
    if context.is_empty() || context.iter().any(|pitch| usize::from(*pitch) >= VOCAB) {
        return None;
    }

    let scale: [u8; 7] = if minor {
        [0, 2, 3, 5, 7, 8, 10]
    } else {
        [0, 2, 4, 5, 7, 9, 11]
    };
    let allowed: Vec<u8> = (60_u8..=95)
        .filter(|pitch| scale.contains(&((pitch.wrapping_sub(60 + root % 12)) % 12)))
        .collect();
    if allowed.is_empty() {
        return None;
    }

    let mut hidden = [0.0_f32; HIDDEN];
    let mut cell = [0.0_f32; HIDDEN];
    let mut logits = [0.0_f32; VOCAB];
    for &pitch in context {
        logits = next_logits(weights, pitch, &mut hidden, &mut cell);
    }

    let mut rng = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let max = allowed
            .iter()
            .map(|pitch| logits[usize::from(*pitch)])
            .fold(f32::NEG_INFINITY, f32::max);
        if !max.is_finite() {
            return None;
        }
        let mut probabilities = Vec::with_capacity(allowed.len());
        let mut total = 0.0_f64;
        for &pitch in &allowed {
            let probability = (f64::from((logits[usize::from(pitch)] - max) / 0.85).exp())
                .max(0.0);
            probabilities.push(probability);
            total += probability;
        }
        if !total.is_finite() || total <= 0.0 {
            return None;
        }
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        let mut pick = (rng as f64 / u64::MAX as f64) * total;
        let mut selected = *allowed.last()?;
        for (&pitch, probability) in allowed.iter().zip(probabilities) {
            if pick <= probability {
                selected = pitch;
                break;
            }
            pick -= probability;
        }
        result.push(selected);
        logits = next_logits(weights, selected, &mut hidden, &mut cell);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_size_and_non_finite_weights() {
        assert!(Weights::parse(&[]).is_none());
        let mut bytes = vec![0_u8; WEIGHT_BYTES];
        bytes[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Weights::parse(&bytes).is_none());
    }

    #[test]
    fn weight_layout_matches_trained_checkpoint() {
        assert_eq!(WEIGHT_FLOATS, 259_712);
        assert_eq!(WEIGHT_BYTES, 1_038_848);
    }

    #[test]
    fn inference_is_repeatable_and_stays_in_the_requested_scale() {
        let weights = vec![0_u8; WEIGHT_BYTES];
        let context = [72, 76, 79, 76, 74, 77, 81, 77];
        let a = generate_pitch_sequence(&weights, &context, 24, 0, false, 1234).unwrap();
        let b = generate_pitch_sequence(&weights, &context, 24, 0, false, 1234).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 24);
        assert!(a.iter().all(|pitch| [0, 2, 4, 5, 7, 9, 11].contains(&((pitch - 60) % 12))));
    }
}
