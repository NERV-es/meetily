// Speaker embedding extraction using ONNX Runtime (WeSpeaker ResNet34-LM)
// Model: talatapp/wespeaker-voxceleb-resnet34-LM-onnx (baked fbank + masking)
// I/O contract:
//   input  waveform f32 [1, 160000]  — raw 16kHz mono PCM, fixed 10s window
//   input  mask     f32 [1, 589]     — pyannote-3.0 frame mask (1 = active)
//   output embedding f32 [1, 256]    — L2-normalizable speaker embedding

use log::info;
use ort::inputs;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Fixed waveform length the model expects (10s @ 16kHz).
const WAVEFORM_LEN: usize = 160_000;
/// Fixed mask length (pyannote-3.0 frame count).
const MASK_LEN: usize = 589;
/// Embedding dimensionality produced by the model.
pub const EMBEDDING_DIM: usize = 256;

/// WeSpeaker-based speaker embedding extractor
pub struct EmbeddingExtractor {
    session: Arc<Mutex<Session>>,
    sample_rate: u32,
}

impl EmbeddingExtractor {
    /// Load the ONNX model
    pub fn new(model_path: &Path) -> Result<Self, String> {
        let session = Session::builder()
            .map_err(|e| format!("Failed to create ONNX session builder: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| format!("Failed to load model {}: {}", model_path.display(), e))?;

        info!("🎤 Speaker embedding model loaded from {}", model_path.display());

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            sample_rate: 16000,
        })
    }

    /// Extract speaker embedding from audio samples.
    /// Audio must be 16kHz mono f32. Shorter clips are zero-padded, longer
    /// clips are truncated to the model's fixed 10s window.
    pub async fn extract_embedding(&self, audio: &[f32]) -> Result<Vec<f32>, String> {
        if audio.len() < (self.sample_rate as usize / 2) {
            return Err("Audio too short for embedding (need at least 0.5s)".to_string());
        }

        let mut session = self.session.lock().await;

        // Pad/truncate the waveform to the fixed [1, 160000] window the model expects.
        let mut waveform = vec![0.0f32; WAVEFORM_LEN];
        let copy_len = audio.len().min(WAVEFORM_LEN);
        waveform[..copy_len].copy_from_slice(&audio[..copy_len]);

        // Mask is [1, 589], all-active for a single-speaker slot. We mark frames
        // active in proportion to how much of the window actually contains audio,
        // so trailing zero-pad doesn't drag the per-utterance CMN.
        let active_frames =
            (((copy_len as f64 / WAVEFORM_LEN as f64) * MASK_LEN as f64).ceil() as usize)
                .clamp(1, MASK_LEN);
        let mut mask = vec![0.0f32; MASK_LEN];
        for m in mask.iter_mut().take(active_frames) {
            *m = 1.0;
        }

        let waveform_array = ndarray::Array2::from_shape_vec((1, WAVEFORM_LEN), waveform)
            .map_err(|e| format!("Failed to create waveform array: {}", e))?;
        let mask_array = ndarray::Array2::from_shape_vec((1, MASK_LEN), mask)
            .map_err(|e| format!("Failed to create mask array: {}", e))?;

        let model_inputs = inputs![
            "waveform" => TensorRef::from_array_view(waveform_array.view())
                .map_err(|e| format!("waveform TensorRef error: {}", e))?,
            "mask" => TensorRef::from_array_view(mask_array.view())
                .map_err(|e| format!("mask TensorRef error: {}", e))?,
        ];

        let outputs = session
            .run(model_inputs)
            .map_err(|e| format!("Inference error: {}", e))?;

        // Output is [1, 256] — extract as Vec<f32>
        let (_, output) = outputs
            .iter()
            .next()
            .ok_or("No output from model")?;

        let embedding: Vec<f32> = output
            .try_extract_array::<f32>()
            .map_err(|e| format!("Failed to extract embedding: {}", e))?
            .iter()
            .cloned()
            .collect();

        // L2 normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            Ok(embedding.iter().map(|x| x / norm).collect())
        } else {
            Ok(embedding)
        }
    }

    /// Compute cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }
}
