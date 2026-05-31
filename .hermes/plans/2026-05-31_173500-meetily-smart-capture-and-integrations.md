# Meetily: Smart Capture, Calendar Integration, and Centralized Pipeline

## Goal

Transform Meetily from a manual-start recording tool into an intelligent meeting assistant that:
1. Auto-detects meeting start/end and captures without user intervention
2. Subscribes to Google Calendar (ICS) to know when meetings are scheduled
3. Exposes transcriptions via CLI/MCP for centralized processing (like Pieces)
4. Shares a unified dictionary with VoiceInk and Raycast snippets

The core principle: **Meetily is a data source, not an AI endpoint.** Transcriptions flow into the user's centralized memory/processing pipeline. Reduce in-app AI to transcription only; summarization and intelligence live downstream.

## Current State

- Tauri 2.x desktop app (Rust + Next.js)
- Already has `system_detector.rs` that monitors system audio (apps using audio output)
- Has `SystemAudioEvent::SystemAudioStarted(Vec<String>)` — detects which apps produce audio
- Has notification system, tray icon with quick-record
- SQLite database with meetings, transcripts, transcript_chunks, summaries
- Backend (FastAPI) exists but is optional — most logic is in Rust
- No calendar integration currently
- No CLI/API for external consumers
- No dictionary/word-replacement system (VoiceInk has one)

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Calendar source | ICS URL subscription (not OAuth) | Simpler, works with any calendar, no auth flow |
| Meeting detection | System audio + calendar heuristic | If calendar says meeting NOW + system audio from Zoom/Teams/Meet → auto-start |
| External API | Unix socket + CLI binary | No network exposure, fast, works with MCP |
| Dictionary sync | Shared JSON file in ~/.config/dictionaries/ | All apps read/write same file, fswatch for live reload |
| Processing pipeline | Meetily exports → Hermes/mem0 imports | Meetily just writes transcripts; processing is external |

---

## Step-by-Step Plan

### Phase 1: Auto Meeting Detection (Rust)

**Files to create/modify:**
- `frontend/src-tauri/src/meeting_detector/mod.rs` (new module)
- `frontend/src-tauri/src/meeting_detector/calendar.rs` — ICS parser + scheduler
- `frontend/src-tauri/src/meeting_detector/heuristic.rs` — decision engine
- `frontend/src-tauri/src/meeting_detector/commands.rs` — Tauri commands
- `frontend/src-tauri/src/lib.rs` — register module

**Logic:**
```
MeetingDetector {
    // Inputs
    calendar_events: Vec<CalendarEvent>,  // from ICS subscription
    system_audio: SystemAudioEvent,       // existing system_detector
    known_meeting_apps: [Zoom, Teams, Meet, Webex, Slack Huddle, FaceTime]

    // Decision matrix
    should_auto_start() -> bool:
        IF calendar_event.is_active_now() AND audio_from_meeting_app():
            return true  // high confidence
        IF audio_from_meeting_app() AND duration > 30s:
            return true  // no calendar but clearly in a meeting
        return false

    should_auto_stop() -> bool:
        IF was_auto_started AND no_meeting_app_audio() for > 60s:
            return true
        IF calendar_event.ended() AND no_audio for > 30s:
            return true
}
```

**Known meeting apps (macOS bundle IDs):**
- `us.zoom.xos` / `us.zoom.videomeeting`
- `com.microsoft.teams2`
- `com.google.Chrome` (with window title heuristic for Meet)
- `com.cisco.webexmeetingsapp`
- `com.apple.FaceTime`
- `com.tinyspeck.slackmacgap` (huddle detection via audio)

### Phase 2: ICS Calendar Subscription

**Files:**
- `frontend/src-tauri/src/meeting_detector/calendar.rs`
- New migration: `migrations/YYYYMMDD_add_calendar_subscription.sql`

**Approach:**
- User provides ICS URL (Google Calendar → Settings → "Secret address in iCal format")
- Poll ICS every 5 minutes, parse with `ical` crate
- Store upcoming events in memory (next 24h window)
- Expose settings in UI: ICS URL, auto-record toggle, grace period (minutes before/after)

**Schema addition:**
```sql
CREATE TABLE IF NOT EXISTS calendar_settings (
    id INTEGER PRIMARY KEY,
    ics_url TEXT,
    poll_interval_minutes INTEGER DEFAULT 5,
    auto_record_enabled BOOLEAN DEFAULT 0,
    pre_meeting_buffer_seconds INTEGER DEFAULT 60,
    post_meeting_buffer_seconds INTEGER DEFAULT 120
);
```

**Crate deps:** `ical` (ICS parser), already has `reqwest` for HTTP.

### Phase 3: CLI + MCP Interface for Transcript Access

**New top-level crate:** `meetily-cli/` (or add to workspace)

**Approach — Unix Domain Socket server in the Tauri app:**

The running Meetily app listens on `~/.local/share/meetily/meetily.sock` (UDS).

**CLI binary `meetily-cli`:**
```bash
meetily-cli meetings list [--since 2024-01-01] [--json]
meetily-cli meetings get <id> [--format json|markdown|plain]
meetily-cli transcript latest [--format markdown]
meetily-cli transcript stream          # live stream current transcription
meetily-cli dictionary list
meetily-cli dictionary add "originalText" "replacementText"
meetily-cli dictionary sync --from voiceink
meetily-cli status                     # recording? idle? what meeting?
```

**MCP server (for Hermes/agents):**

Expose as MCP tool definitions:
- `meetily_list_meetings` — list recent meetings with metadata
- `meetily_get_transcript` — get full transcript for a meeting ID
- `meetily_get_latest` — get most recent completed meeting transcript
- `meetily_stream_live` — SSE stream of live transcription chunks
- `meetily_status` — current recording state

This lets Hermes cron jobs poll for new transcripts and feed them into mem0/processing.

**Files:**
- `meetily-cli/Cargo.toml` (new workspace member)
- `meetily-cli/src/main.rs`
- `frontend/src-tauri/src/ipc_server/mod.rs` — UDS listener
- `frontend/src-tauri/src/ipc_server/protocol.rs` — JSON-RPC over socket
- `frontend/src-tauri/src/ipc_server/handlers.rs` — query handlers

### Phase 4: Unified Dictionary System

**Shared dictionary location:** `~/.config/unified-dictionary/dictionary.json`

**Format:**
```json
{
  "version": 2,
  "updated_at": "2026-05-31T22:00:00Z",
  "entries": [
    {
      "id": "uuid",
      "original": "kubernetes",
      "replacement": "Kubernetes",
      "case_sensitive": true,
      "regex": false,
      "source": "voiceink",
      "tags": ["tech"]
    }
  ]
}
```

**Sync strategy:**
- All apps (VoiceInk, Meetily, Raycast) read/watch this file
- Any app can write to it (last-write-wins per entry, keyed by `id`)
- `fsnotify` / `kqueue` watches for changes and hot-reloads
- VoiceInk's existing WordReplacement maps 1:1 to this format
- Raycast snippets: export script converts between Raycast JSON and unified format

**Files:**
- `frontend/src-tauri/src/dictionary/mod.rs` — shared dict reader/writer
- `frontend/src-tauri/src/dictionary/sync.rs` — file watcher + reload
- `frontend/src-tauri/src/dictionary/commands.rs` — Tauri commands
- Standalone sync script: `scripts/sync-raycast-snippets.py`

**Migration for VoiceInk:**
- Add export/import commands to VoiceInk that read/write the unified file
- VoiceInk keeps its internal DB as source-of-truth, syncs bidirectionally

### Phase 5: Transcript Export Pipeline Hook

Rather than building AI flows inside Meetily, add a **post-recording hook**:

**On recording complete:**
1. Save transcript to local DB (existing)
2. Write transcript as markdown to `~/Documents/Meetily/transcripts/YYYY-MM-DD_meeting-name.md`
3. Emit event on UDS (for listening CLI/agents)
4. Optionally: POST to configurable webhook URL

**Hermes cron job pattern:**
```
Every 5 minutes:
  meetily-cli meetings list --since "5 minutes ago" --json
  For each new meeting:
    meetily-cli transcript get <id> --format markdown
    Feed into mem0 / processing pipeline
```

Or with MCP, Hermes just has the `meetily` MCP server configured and the cron job uses those tools directly.

---

## Implementation Order & Effort

| Phase | Effort | Dependencies |
|-------|--------|-------------|
| 1. Auto-detect | M (3-4h) | Existing system_detector infra |
| 2. ICS calendar | S (2h) | Phase 1 for integration |
| 3. CLI + MCP | L (5-6h) | UDS server, protocol design |
| 4. Dictionary | M (3h) | File format, watchers |
| 5. Export hooks | S (1-2h) | Phase 3 for notification |

**Total estimate:** ~15h of implementation

## Risks & Tradeoffs

- **Auto-start false positives:** YouTube/Spotify audio from Chrome could trigger. Mitigation: require calendar match OR known meeting app bundle ID, not just any system audio.
- **ICS polling lag:** 5-minute poll means meeting could start before we know. Mitigation: pre-buffer setting (start recording N seconds before scheduled time).
- **UDS on macOS sandboxing:** If Meetily is ever App Store distributed, UDS might be blocked. For now (ad-hoc signed), this is fine.
- **Dictionary conflicts:** Two apps writing simultaneously. Mitigation: atomic writes (write to tmp, rename), and per-entry `updated_at` for conflict resolution.
- **Chrome/Meet detection:** Can't easily tell if Chrome audio is a Meet call vs YouTube. Mitigation: window title matching via accessibility API ("Meet - " prefix in tab title).

---

## Phase 6: Local Speaker Diarization (Who Said What)

**Current state:** `stt.rs` already imports a `pyannote` module (not yet implemented) and stores `speaker_embedding: Vec<f32>` per segment. Migration `20251110000001` added a `speaker` column. Infrastructure is scaffolded.

**Approach — pyannote-style local diarization via ONNX:**

Use `ort` (already a dependency) to run speaker embedding + segmentation models locally:
- **VAD model:** silero-vad (already referenced via git dep `silero-rs`)
- **Segmentation model:** pyannote/segmentation-3.0 (ONNX export, ~5MB)
- **Embedding model:** wespeaker/voxceleb_resnet34 or pyannote/embedding (ONNX, ~18MB)

**Pipeline:**
```
Audio chunks → VAD (speech segments) → Segmentation (speaker turns)
    → Embedding extraction per segment → Clustering (agglomerative)
    → Speaker labels: "Speaker 1", "Speaker 2", ...
    → Optional: user assigns names ("Speaker 1" = "John")
```

**Files to create:**
- `frontend/src-tauri/src/pyannote/mod.rs`
- `frontend/src-tauri/src/pyannote/models.rs` — model download + cache
- `frontend/src-tauri/src/pyannote/segment.rs` — speech segmentation
- `frontend/src-tauri/src/pyannote/embedding.rs` — speaker embedding extraction
- `frontend/src-tauri/src/pyannote/identify.rs` — clustering + speaker assignment
- `frontend/src-tauri/src/pyannote/commands.rs` — Tauri commands (label speakers, list speakers)

**Speaker memory:**
```sql
CREATE TABLE IF NOT EXISTS known_speakers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    embedding BLOB NOT NULL,  -- averaged embedding vector
    meeting_count INTEGER DEFAULT 1,
    last_seen_at TEXT
);
```

Across meetings, recognized embeddings are matched against `known_speakers` (cosine similarity > 0.75). New speakers get auto-assigned "Speaker N" until user labels them.

**Enriched transcript output:**
```markdown
## Meeting: Weekly Standup (2026-05-31)

**[John, 0:00-0:45]** So let's go around the room...
**[Sarah, 0:46-1:23]** I finished the API migration yesterday...
**[Unknown Speaker 3, 1:24-1:50]** Quick question about the deployment...
```

---

## Phase 7: Full Google Calendar Sync (OAuth)

Upgrade from ICS polling (Phase 2) to full Google Calendar API:

**Why upgrade beyond ICS:**
- ICS is read-only, 5-min lag, no attendee info
- OAuth gives: real-time events, attendee list (for speaker identification hints), event updates, meet links
- Can auto-extract meeting context (agenda from description, attendees → expected speakers)

**Approach:**
- Google OAuth2 PKCE flow (desktop app, no server needed)
- Store refresh token in system keychain (tauri-plugin-store or keyring crate)
- Sync events for next 7 days, poll every 60s for changes
- Extract: title, attendees (name + email), description, conferencing link

**Files:**
- `frontend/src-tauri/src/calendar/mod.rs`
- `frontend/src-tauri/src/calendar/google_oauth.rs` — PKCE flow + token refresh
- `frontend/src-tauri/src/calendar/sync.rs` — event fetcher + cache
- `frontend/src-tauri/src/calendar/types.rs` — CalendarEvent struct
- `frontend/src-tauri/src/calendar/commands.rs` — Tauri commands

**Integration with diarization:**
Calendar attendees → pre-seed expected speakers. If meeting has 3 attendees and diarization detects 3 speakers, auto-suggest name assignment.

**Settings UI:**
- Connect Google Calendar (OAuth button)
- Select which calendars to monitor
- Toggle: auto-record all meetings / only meetings with video links / manual approval

---

## Phase 8: Full MCP Server

Beyond the basic CLI (Phase 3), implement a proper MCP server that runs alongside the app:

**Transport:** stdio (launched by MCP client) OR streamable-http on localhost

**MCP Tools:**
| Tool | Description |
|------|-------------|
| `meetily_list_meetings` | List meetings with filters (date range, speaker, keyword) |
| `meetily_get_transcript` | Full enriched transcript (with speakers, timestamps) |
| `meetily_get_summary` | Get/generate summary for a meeting |
| `meetily_search_transcripts` | FTS across all transcripts |
| `meetily_get_speakers` | List known speakers with stats |
| `meetily_label_speaker` | Assign name to a speaker ID |
| `meetily_get_calendar_events` | Upcoming meetings from synced calendar |
| `meetily_get_context` | Current meeting context (who's in it, what's on screen) |
| `meetily_stream_live` | Subscribe to live transcription chunks |
| `meetily_dictionary_manage` | CRUD on unified dictionary |

**MCP Resources:**
- `meetily://meetings/{id}/transcript` — enriched markdown
- `meetily://meetings/{id}/audio` — audio file reference
- `meetily://meetings/latest` — most recent completed meeting
- `meetily://dictionary` — current dictionary state

**Implementation:** Use `rmcp` crate (Rust MCP SDK) or hand-roll JSON-RPC over stdio.

**Files:**
- `meetily-mcp/Cargo.toml` (new workspace member)
- `meetily-mcp/src/main.rs` — MCP server entry
- `meetily-mcp/src/tools.rs` — tool implementations
- `meetily-mcp/src/resources.rs` — resource providers
- Communicates with running Meetily via UDS (Phase 3 foundation)

---

## Phase 9: Context Awareness (Screen + Active App)

Enrich transcripts with what's happening on screen during the meeting:

**Data sources:**
1. **Active window title** — via macOS Accessibility API (already have screen recording permission)
2. **Shared screen detection** — detect when Zoom/Teams shows "You are sharing your screen"
3. **Active document** — which file/URL is being discussed (from window title parsing)
4. **Periodic screenshots** — OCR key frames for slide content (opt-in, privacy-sensitive)

**Approach — lightweight metadata, not full screen recording:**

```rust
struct MeetingContext {
    timestamp: DateTime,
    active_app: String,         // "Google Chrome"
    window_title: String,       // "Q2 Planning - Google Slides"
    shared_screen: bool,        // is user sharing?
    detected_content_type: ContentType,  // Slides, Code, Document, Browser
    // Optional (if OCR enabled):
    key_text: Option<String>,   // OCR'd slide title or code snippet
}
```

**Context capture interval:** Every 30 seconds during recording, snapshot active window metadata. Full OCR only on significant changes (window title changed).

**Integration with transcript:**
```markdown
**[John, 5:23-5:45]** _[Context: Viewing "Q2 OKRs - Google Sheets"]_
As you can see in the spreadsheet, we're at 73% completion...

**[Sarah, 5:46-6:10]** _[Context: Screen shared - "Architecture Diagram.fig"]_
Let me walk you through the updated diagram...
```

**Files:**
- `frontend/src-tauri/src/context/mod.rs`
- `frontend/src-tauri/src/context/window_monitor.rs` — AX API window title polling
- `frontend/src-tauri/src/context/screen_ocr.rs` — optional OCR via Vision.framework
- `frontend/src-tauri/src/context/enrichment.rs` — attach context to transcript chunks

**macOS APIs:**
- `CGWindowListCopyWindowInfo` — active window info
- `AXUIElementCopyAttributeValue` — window titles
- `Vision.framework VNRecognizeTextRequest` — OCR (already available via cidre crate)

**Privacy controls:**
- Context capture: ON by default (just window titles, no OCR)
- OCR capture: OFF by default, opt-in per-meeting or global setting
- Exclude list: apps whose titles are never captured (e.g., 1Password, Messages)

---

## Phase 10: Enriched Transcript Output Format

The final output combines all phases into a rich, machine-parseable transcript:

```markdown
---
meeting_id: "abc-123"
title: "Weekly Engineering Standup"
date: 2026-05-31T10:00:00-05:00
duration: "32:15"
calendar_event: "Google Calendar: Weekly Standup"
attendees: ["John Smith", "Sarah Chen", "Unknown Speaker 3"]
speakers_identified: 3
context_captures: 12
dictionary_corrections: 8
---

# Weekly Engineering Standup
_May 31, 2026 • 32 minutes • 3 participants_

## Participants
- **John Smith** (host) — 45% speaking time
- **Sarah Chen** — 38% speaking time
- **Speaker 3** (unidentified) — 17% speaking time

## Transcript

**[John, 0:00]** _[Context: Zoom main window]_
Good morning everyone. Let's do a quick round...

**[Sarah, 0:23]** _[Context: Viewing "sprint-board - Linear"]_
I wrapped up the Kubernetes migration yesterday. All pods are healthy.

**[Speaker 3, 1:05]** _[Context: Screen shared - "Grafana Dashboard"]_
Quick flag — I'm seeing elevated error rates on the payment service since last night's deploy.

## Key Moments
- 1:05 — Error rate alert discussed (Speaker 3 shared Grafana)
- 5:30 — Architecture decision: moving to event-driven (Sarah shared diagram)
- 12:00 — Action item: John to review PR #482 by EOD

## Context Timeline
| Time | Active Content |
|------|---------------|
| 0:00-1:00 | Zoom gallery view |
| 1:05-3:20 | Grafana Dashboard (shared) |
| 5:30-8:00 | Architecture Diagram.fig (shared) |
| 8:00-32:15 | Zoom gallery view |
```

This format is designed for downstream processing — Hermes/mem0 can parse the YAML frontmatter, extract action items, identify topics, and build knowledge graphs without needing to re-process audio.

---

## Updated Implementation Order

| Phase | Effort | Priority |
|-------|--------|----------|
| 1. Auto-detect meetings | M (3-4h) | P0 — core UX |
| 2. ICS calendar (simple) | S (2h) | P0 — enables auto-detect |
| 3. CLI + UDS server | L (5-6h) | P0 — enables pipeline |
| 4. Dictionary sync | M (3h) | P1 — cross-app value |
| 5. Export hooks | S (1-2h) | P0 — pipeline trigger |
| 6. Speaker diarization | XL (8-10h) | P1 — major differentiator |
| 7. Full Google Calendar | M (4h) | P2 — upgrade from ICS |
| 8. Full MCP server | L (6h) | P1 — agent integration |
| 9. Context awareness | L (6-8h) | P2 — enrichment |
| 10. Rich transcript format | M (3h) | P1 — ties it all together |

**Total estimate:** ~45-50h of implementation

## Open Questions

1. Should the MCP server be a separate process or embedded in the Tauri app? (Embedded = simpler but dies with app; separate = more robust but another process to manage) → **Recommendation: separate binary in workspace, communicates via UDS**
2. Dictionary sync with Raycast: push or pull? → **Periodic export script, Raycast has no live sync API**
3. Speaker diarization model size: wespeaker (18MB, good) vs pyannote embedding (70MB, better)? → **Start with wespeaker, upgrade later**
4. OCR privacy: should screen captures ever be stored, or only ephemeral? → **Ephemeral only, extract text then discard image**
5. Google Calendar OAuth: store in keychain or encrypted file? → **Keychain via security framework / keyring crate**
