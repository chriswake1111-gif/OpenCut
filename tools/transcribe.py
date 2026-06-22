import os
import json
import sys
import argparse
from imageio_ffmpeg import get_ffmpeg_exe
import subprocess

def main():
    parser = argparse.ArgumentParser(description="Local ASR using Whisper-Base")
    parser.add_argument("--input", required=True, help="Input video or audio file path")
    parser.add_argument("--output", required=True, help="Output JSON path for segments")
    args = parser.parse_args()

    # 1. Find snapshot path
    snapshots_dir = r"C:\Users\User\.cache\huggingface\hub\models--openai--whisper-base\snapshots"
    if not os.path.exists(snapshots_dir):
        print(f"Error: Snapshots directory does not exist: {snapshots_dir}", file=sys.stderr)
        sys.exit(1)
        
    subdirs = [os.path.join(snapshots_dir, d) for d in os.listdir(snapshots_dir) if os.path.isdir(os.path.join(snapshots_dir, d))]
    if not subdirs:
        print("Error: No snapshots folder found inside whisper-base cache folder.", file=sys.stderr)
        sys.exit(1)
        
    model_path = subdirs[0]
    print(f"Loading Whisper model from: {model_path}")
    
    # 2. Extract audio to 16kHz mono WAV using imageio-ffmpeg
    temp_wav = args.output + ".temp.wav"
    ffmpeg_exe = get_ffmpeg_exe()
    
    print(f"Extracting audio to: {temp_wav}")
    cmd = [
        ffmpeg_exe, "-y", 
        "-i", args.input, 
        "-ar", "16000", 
        "-ac", "1", 
        "-f", "wav", 
        temp_wav
    ]
    ret = subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if ret.returncode != 0:
        print("Warning: Failed to extract audio. The file might have no audio track. Writing empty JSON segments.", file=sys.stderr)
        with open(args.output, "w", encoding="utf-8") as f:
            json.dump([], f)
        sys.exit(0)
        
    # 3. Load model and transcribe
    try:
        import torch
        from transformers import pipeline
        
        device = "cuda" if torch.cuda.is_available() else "cpu"
        print(f"Using device: {device}")
        
        pipe = pipeline(
            "automatic-speech-recognition",
            model=model_path,
            chunk_length_s=30,
            device=device,
            return_timestamps=True
        )
        
        print("Transcribing...")
        result = pipe(temp_wav, generate_kwargs={"task": "transcribe"})
        
        chunks = result.get("chunks", [])
        segments = []
        for chunk in chunks:
            timestamp = chunk.get("timestamp")
            if timestamp is not None and len(timestamp) == 2:
                start, end = timestamp
                if start is None: start = 0.0
                if end is None: end = start + 2.0
                segments.append({
                    "start": float(start),
                    "end": float(end),
                    "text": str(chunk.get("text", "")).strip()
                })
        
        with open(args.output, "w", encoding="utf-8") as f:
            json.dump(segments, f, ensure_ascii=False, indent=2)
            
        print(f"Successfully transcribed. Output saved to: {args.output}")
        
    except Exception as e:
        print(f"Error during transcription: {e}", file=sys.stderr)
        sys.exit(1)
    finally:
        if os.path.exists(temp_wav):
            try:
                os.remove(temp_wav)
            except:
                pass

if __name__ == "__main__":
    main()
