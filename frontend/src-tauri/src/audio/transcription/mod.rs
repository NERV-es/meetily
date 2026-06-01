// audio/transcription/mod.rs
//
// Transcription module: Provider abstraction, engine management, and worker pool.

pub mod provider;
pub mod whisper_provider;
pub mod parakeet_provider;
#[cfg(target_os = "macos")]
#[cfg(feature = "apple-speech")]
pub mod apple_speech_provider;
pub mod engine;
pub mod enhancement;
pub mod worker;

// Re-export commonly used types
pub use provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
pub use whisper_provider::WhisperProvider;
pub use parakeet_provider::ParakeetProvider;
#[cfg(target_os = "macos")]
#[cfg(feature = "apple-speech")]
pub use apple_speech_provider::AppleSpeechProvider;
pub use engine::{
    TranscriptionEngine,
    validate_transcription_model_ready,
    get_or_init_transcription_engine,
    get_or_init_whisper
};
pub use worker::{
    start_transcription_task,
    reset_speech_detected_flag,
    TranscriptUpdate
};
pub use enhancement::{
    EnhancementConfig,
    EnhancementState,
    EnhancementStateHandle,
    EnhancementRequest,
    TranscriptEnhancement,
    new_enhancement_state,
    start_enhancement_pipeline,
    get_enhancement_config,
    set_enhancement_config,
    get_enhancement_stats,
};
