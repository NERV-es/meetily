# CLAUDE.md — Meetily Agent Reference

## Overview
Privacy-first AI meeting assistant. Local capture → transcribe → summarize.
- Frontend: Tauri 2.x (Rust) + Next.js 14 + React 18 | `/frontend`
- Backend: FastAPI + SQLite (aiosqlite) | `/backend`
- Audio: Rust (cpal, whisper-rs) + Whisper.cpp (local, GPU-accel)
- LLM: Ollama | Claude | Groq | OpenRouter

## Dev Commands

### Frontend
```bash
./clean_run.sh [debug]      # macOS: clean build+run
./clean_build.sh            # prod build
clean_run_windows.bat / clean_build_windows.bat
pnpm install && pnpm run dev          # Next.js (port 3118)
pnpm run tauri:dev / tauri:build
pnpm run tauri:dev:metal|cuda|vulkan|cpu   # GPU variants
```

### Backend
```bash
./build_whisper.sh small    # build Whisper + model
./clean_start_backend.sh    # start API (port 5167)
# Windows: build_whisper.cmd small | clean_start_backend.cmd
./run-docker.sh start --interactive   # Docker
```

Whisper models: `tiny` `base` `small` `medium` `large-v1/v2/v3` `large-v3-turbo` (+ `.en` variants)

### Endpoints
- Whisper: :8178 | Backend API: :5167 (/docs, /redoc) | Frontend dev: :3118

## Architecture

### System (3-tier)
```
Tauri Frontend: Next.js UI ←→ Rust (Audio+IPC) ←→ Whisper (local STT)
                      ↓ HTTP
FastAPI Backend: SQLite ←→ Meeting Manager ←→ LLM (Ollama/etc)
```

### Audio Pipeline (2 parallel paths)
```
Mic + System Audio → AudioPipelineManager (pipeline.rs)
  ├→ Recording Path (pre-mixed) → RecordingSaver.save()
  └→ Transcription Path (VAD-filtered) → WhisperEngine.transcribe()
```
- Recording: RMS-based ducking, clipping prevention
- Transcription: VAD filters speech only → ~70% Whisper load reduction
- 48kHz required; resampling at capture. Ring buffer (VecDeque), 50ms windows.

### Audio Module Structure (`frontend/src-tauri/src/audio/`)
```
devices/discovery.rs        # list_audio_devices, permissions
devices/microphone.rs / speakers.rs
devices/configuration.rs    # AudioDevice types
devices/platform/windows.rs(WASAPI) | macos.rs(ScreenCaptureKit) | linux.rs(ALSA/PA)
capture/microphone.rs | system.rs | core_audio.rs
pipeline.rs                 # AudioMixerRingBuffer, ProfessionalAudioMixer, AudioPipelineManager
recording_manager.rs | recording_commands.rs | recording_saver.rs
```
Issue routing: device detect → `devices/discovery.rs`/`platform/*` | capture → `capture/*` | mix/VAD → `pipeline.rs` | workflow → `recording_manager.rs`

## Tauri IPC

### Frontend → Rust
```typescript
await invoke('start_recording', { mic_device_name, system_device_name, meeting_name });
```
```rust
#[tauri::command]
async fn start_recording<R: Runtime>(app: AppHandle<R>, mic_device_name: Option<String>, ...) -> Result<(), String>
```

### Rust → Frontend
```rust
app.emit("transcript-update", TranscriptUpdate { text, timestamp })?;
```
```typescript
await listen<TranscriptUpdate>('transcript-update', (e) => setTranscripts(p => [...p, e.payload]));
```

### Add Tauri Command
1. `#[tauri::command] async fn my_cmd(arg: String) -> Result<String, String>`
2. Register: `.invoke_handler(tauri::generate_handler![..., my_cmd])`
3. Call: `await invoke<string>('my_cmd', { arg: 'value' })`

## Key Patterns

### Thread Safety
```rust
pub struct RecordingState {
    is_recording: Arc<AtomicBool>,
    audio_sender: Arc<RwLock<Option<mpsc::UnboundedSender<AudioChunk>>>>,
}
// Arc<RwLock<T>> for shared async state; Arc<AtomicBool> for flags
```

### Perf Logging (zero-cost in release)
```rust
#[cfg(debug_assertions)] macro_rules! perf_debug { ($($arg:tt)*) => { log::debug!($($arg)*) }; }
#[cfg(not(debug_assertions))] macro_rules! perf_debug { ($($arg:tt)*) => {}; }
// Use perf_debug!()/perf_trace!() on hot paths
```

### Error Handling + Logging
- Rust: `anyhow::Result` | Frontend: try-catch + user-friendly msg
- Backend log: `2025-01-03 12:34:56 - INFO - [file.py:123 - func()] - msg`

### Frontend State
- `SidebarProvider.tsx` → global state (meetings, recording status, WebSocket)
- Flow: Tauri cmd → Rust state → emit event → React listener → context → components

## Whisper Models
- Dev: `frontend/models/` or `backend/whisper-server-package/models/`
- Prod macOS: `~/Library/Application Support/Meetily/models/`
- Prod Windows: `%APPDATA%\Meetily\models\`
- Load once + cached; model change → restart. GPU auto-detect, fallback CPU.
- Cargo: `--features cuda` | `--features vulkan`
- Speed: GPU 5-10x. Dev: `base`/`small` | Prod: `medium`/`large-v3`

## Backend API
```python
@app.post("/api/my-endpoint")
async def my_endpoint(request: MyRequest) -> MyResponse:
    db = DatabaseManager()   # all DB via DatabaseManager (db.py), aiosqlite
    return await db.some_operation()
```

## Platform Notes
| | macOS | Windows | Linux |
|---|---|---|---|
| System audio | ScreenCaptureKit (13+) + BlackHole | WASAPI loopback | ALSA/PulseAudio |
| GPU | Metal+CoreML (auto) | CUDA/Vulkan | CUDA/Vulkan |
| Deps | mic+screen recording perms | VS Build Tools + C++ | cmake, llvm, libomp |
| Quirks | ScreenCaptureKit needs macOS 13+ | WASAPI excl. mode can conflict | - |

## Debugging
```bash
RUST_LOG=debug ./clean_run.sh
RUST_LOG=app_lib::audio=debug ./clean_run.sh   # audio only
$env:RUST_LOG="debug"; ./clean_run_windows.bat  # Windows PS
# DevTools: Cmd+Shift+I (mac) | Ctrl+Shift+I (win)
# API explorer: http://localhost:5167/docs
```
Audio pipeline emits: buffer sizes, mixing window count, VAD rate, dropped chunks.

## Gotchas
1. 48kHz required — resampling at capture
2. Models load once; change → restart or manual unload
3. Frontend runs standalone (local Whisper); backend needed for persistence + LLM
4. CORS: `"*"` in dev — restrict for prod
5. Always use Tauri path APIs — never hardcode paths
6. macOS: request mic + screen recording perms early

## Conventions
- Device names: "microphone" + "system" (not input/output)
- Git: `main` stable | `fix/*` | `enhance/*`

## Key Files
```
frontend/src-tauri/src/lib.rs                          # Tauri entry, command registration
frontend/src-tauri/src/audio/pipeline.rs               # mixing + VAD
frontend/src-tauri/src/audio/recording_manager.rs      # recording orchestration
frontend/src-tauri/src/audio/recording_saver.rs        # file writing
frontend/src-tauri/src/whisper_engine/whisper_engine.rs# model mgmt + transcription
frontend/src/app/page.tsx                              # main recording UI
frontend/src/components/Sidebar/SidebarProvider.tsx    # global state
backend/app/main.py                                    # FastAPI endpoints
backend/app/db.py                                      # DatabaseManager
```
