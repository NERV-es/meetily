#!/bin/bash
# Download the WeSpeaker ResNet34-LM speaker-embedding model for diarization.
# Model: talatapp/wespeaker-voxceleb-resnet34-LM-onnx (baked fbank + masking)
#   I/O: waveform[1,160000] + mask[1,589] -> embedding[1,256]
# The Rust diarization engine loads it from:
#   $DATA_DIR/com.meetily.ai/models/diarization/wespeaker_en_voxceleb_resnet34.onnx
# where $DATA_DIR is the platform data dir (macOS: ~/Library/Application Support).
set -e

MODEL_NAME="wespeaker_en_voxceleb_resnet34.onnx"
MODEL_URL="https://huggingface.co/talatapp/wespeaker-voxceleb-resnet34-LM-onnx/resolve/main/wespeaker.onnx"

case "$(uname -s)" in
  Darwin) DEST_DIR="$HOME/Library/Application Support/com.meetily.ai/models/diarization" ;;
  Linux)  DEST_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/com.meetily.ai/models/diarization" ;;
  *)      DEST_DIR="$HOME/.local/share/com.meetily.ai/models/diarization" ;;
esac

DEST="$DEST_DIR/$MODEL_NAME"

if [ -f "$DEST" ] && [ "$(stat -f%z "$DEST" 2>/dev/null || stat -c%s "$DEST" 2>/dev/null)" -gt 1000000 ]; then
  echo "✅ Diarization model already present: $DEST"
  exit 0
fi

echo "⬇️  Downloading speaker diarization model (~26 MB)..."
mkdir -p "$DEST_DIR"
curl -fL --progress-bar -o "$DEST.tmp" "$MODEL_URL"

# Sanity check: real ONNX protobuf, not an HTML error page.
SIZE="$(stat -f%z "$DEST.tmp" 2>/dev/null || stat -c%s "$DEST.tmp" 2>/dev/null)"
if [ "$SIZE" -lt 1000000 ]; then
  echo "❌ Download failed — file is only $SIZE bytes (expected ~26 MB)."
  rm -f "$DEST.tmp"
  exit 1
fi

mv "$DEST.tmp" "$DEST"
echo "✅ Installed diarization model: $DEST ($SIZE bytes)"
