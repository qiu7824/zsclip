import argparse
import base64
import json
import os
import sys
import urllib.error
import urllib.request
import uuid
from pathlib import Path


ENDPOINT = "https://openspeech.bytedance.com/api/v3/tts/unidirectional"


def load_segments(path: Path, default_speaker: str):
    if path.suffix.lower() == ".json":
        captions = json.loads(path.read_text(encoding="utf-8"))
        segments = []
        for item in captions:
            text = item.get("text", "").strip()
            if not text:
                continue
            segments.append({
                "speaker": str(item.get("speaker") or default_speaker).strip(),
                "text": text,
            })
        return segments
    text = path.read_text(encoding="utf-8").strip()
    return [{"speaker": default_speaker, "text": text}] if text else []


def parse_chunked_json(raw: str):
    decoder = json.JSONDecoder()
    index = 0
    while index < len(raw):
        while index < len(raw) and raw[index] in " \r\n\t":
            index += 1
        if index >= len(raw):
            break
        obj, index = decoder.raw_decode(raw, index)
        yield obj


def build_auth_headers(api_key: str, app_id: str, access_key: str):
    if api_key.strip():
        return {"X-Api-Key": api_key.strip()}
    if app_id.strip() and access_key.strip():
        return {
            "X-Api-App-Id": app_id.strip(),
            "X-Api-Access-Key": access_key.strip(),
        }
    return {}


def synthesize(
    auth_headers: dict,
    resource_id: str,
    speaker: str,
    text: str,
    speech_rate: int,
    explicit_language: str,
):
    request_id = str(uuid.uuid4())
    req_params = {
        "text": text,
        "speaker": speaker,
        "audio_params": {
            "format": "mp3",
            "sample_rate": 24000,
            "speech_rate": speech_rate,
            "loudness_rate": 0,
        },
    }
    if explicit_language.strip():
        req_params["additions"] = json.dumps(
            {"explicit_language": explicit_language.strip()},
            ensure_ascii=False,
        )
    payload = {
        "user": {"uid": "zsclip-video"},
        "req_params": req_params,
    }
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        ENDPOINT,
        data=body,
        method="POST",
        headers={
            "Content-Type": "application/json",
            "X-Api-Resource-Id": resource_id,
            "X-Api-Request-Id": request_id,
            **auth_headers,
        },
    )

    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            raw_bytes = response.read()
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", errors="replace")
        log_id = exc.headers.get("X-Tt-Logid") or exc.headers.get("X-Tt-LogId") or ""
        raise RuntimeError(f"HTTP {exc.code}: x_tt_logid={log_id} {detail[:1000]}") from exc

    raw = raw_bytes.decode("utf-8", errors="replace")
    chunks = []
    final = None
    errors = []
    for obj in parse_chunked_json(raw):
        code = obj.get("code")
        if isinstance(obj.get("data"), str):
            chunks.append(obj["data"])
        if code == 20000000:
            final = obj
        elif code not in (None, 0, 20000000):
            errors.append(obj)

    if not chunks:
        if errors:
            raise RuntimeError(json.dumps(errors[-1], ensure_ascii=False)[:1000])
        if final:
            raise RuntimeError(
                "TTS returned OK but no audio data. "
                "The speaker is queryable, but this resource did not synthesize audio."
            )
        raise RuntimeError(raw[:1000] if raw else "empty TTS response")

    audio = b"".join(base64.b64decode(chunk) for chunk in chunks)
    return {"audio": audio, "bytes": len(audio), "chunks": len(chunks), "final": final}


def resolve_speaker_id(label: str, male_speaker: str, female_speaker: str, fallback: str) -> str:
    normalized = label.strip().lower()
    if normalized in {"female", "woman", "girl", "女", "女声"}:
        return female_speaker
    if normalized in {"male", "man", "boy", "男", "男声"}:
        return male_speaker
    if label.startswith("S_"):
        return label
    return fallback


def resolve_resource_id(
    label: str,
    male_resource_id: str,
    female_resource_id: str,
    speaker_resource_id: str,
    fallback: str,
) -> str:
    normalized = label.strip().lower()
    if normalized in {"female", "woman", "girl", "女", "女声"} and female_resource_id.strip():
        return female_resource_id.strip()
    if normalized in {"male", "man", "boy", "男", "男声"} and male_resource_id.strip():
        return male_resource_id.strip()
    if speaker_resource_id.strip():
        return speaker_resource_id.strip()
    return fallback


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate ZSClip voiceover with Volcengine TTS V3.")
    parser.add_argument("--api-key", default=os.environ.get("VOLCENGINE_TTS_API_KEY", ""))
    parser.add_argument("--app-id", default=os.environ.get("VOLCENGINE_TTS_APP_ID", ""))
    parser.add_argument("--access-key", default=os.environ.get("VOLCENGINE_TTS_ACCESS_KEY", ""))
    parser.add_argument("--speaker", default="S_WgFVfXhO1")
    parser.add_argument("--male-speaker", default="S_WgFVfXhO1")
    parser.add_argument("--female-speaker", default="zh_female_linxiao_uranus_bigtts")
    parser.add_argument("--resource-ids", default="seed-icl-2.0,seed-icl-1.0,seed-tts-2.0,seed-tts-1.0")
    parser.add_argument("--speaker-resource-id", default="")
    parser.add_argument("--male-resource-id", default="seed-icl-2.0")
    parser.add_argument("--female-resource-id", default="seed-tts-2.0")
    parser.add_argument("--speech-rate", type=int, default=18)
    parser.add_argument("--explicit-language", default="zh-cn")
    parser.add_argument("--text-file", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--segments-dir", default="")
    args = parser.parse_args()

    auth_headers = build_auth_headers(args.api_key, args.app_id, args.access_key)
    if not auth_headers:
        print(
            "Missing Volcengine TTS auth. Set VOLCENGINE_TTS_API_KEY, or set "
            "VOLCENGINE_TTS_APP_ID and VOLCENGINE_TTS_ACCESS_KEY.",
            file=sys.stderr,
        )
        return 2

    segments = load_segments(Path(args.text_file), "male")
    if not segments:
        print("Voiceover text is empty.", file=sys.stderr)
        return 2

    failures = []
    for resource_id in [part.strip() for part in args.resource_ids.split(",") if part.strip()]:
        try:
            pieces = []
            segment_meta = []
            segments_dir = Path(args.segments_dir) if args.segments_dir.strip() else Path(args.output).with_suffix("").parent / f"{Path(args.output).stem}-segments"
            segments_dir.mkdir(parents=True, exist_ok=True)
            for old_file in segments_dir.glob("segment-*.mp3"):
                old_file.unlink()
            for index, segment in enumerate(segments, start=1):
                speaker_id = resolve_speaker_id(
                    segment["speaker"],
                    male_speaker=args.male_speaker,
                    female_speaker=args.female_speaker,
                    fallback=args.speaker,
                )
                segment_resource_id = resolve_resource_id(
                    segment["speaker"],
                    male_resource_id=args.male_resource_id,
                    female_resource_id=args.female_resource_id,
                    speaker_resource_id=args.speaker_resource_id,
                    fallback=resource_id,
                )
                result = synthesize(
                    auth_headers=auth_headers,
                    resource_id=segment_resource_id,
                    speaker=speaker_id,
                    text=segment["text"],
                    speech_rate=args.speech_rate,
                    explicit_language=args.explicit_language,
                )
                pieces.append(result["audio"])
                segment_path = segments_dir / f"segment-{index:02d}-{segment['speaker']}.mp3"
                segment_path.write_bytes(result["audio"])
                segment_meta.append({
                    "index": index,
                    "speaker": segment["speaker"],
                    "speaker_id": speaker_id,
                    "resource_id": segment_resource_id,
                    "path": str(segment_path.resolve()),
                    "bytes": result["bytes"],
                    "chunks": result["chunks"],
                })

            concat_list = segments_dir / "concat-list.txt"
            concat_list.write_text(
                "\n".join(
                    "file '" + item["path"].replace("\\", "/").replace("'", "'\\''") + "'"
                    for item in segment_meta
                ) + "\n",
                encoding="utf-8",
            )

            output = Path(args.output)
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(b"".join(pieces))
            print(json.dumps({
                "ok": True,
                "resource_id": resource_id,
                "male_speaker": args.male_speaker,
                "female_speaker": args.female_speaker,
                "male_resource_id": args.male_resource_id,
                "female_resource_id": args.female_resource_id,
                "speech_rate": args.speech_rate,
                "explicit_language": args.explicit_language,
                "output": str(output.resolve()),
                "segments_dir": str(segments_dir.resolve()),
                "concat_list": str(concat_list.resolve()),
                "bytes": sum(item["bytes"] for item in segment_meta),
                "segments": segment_meta,
            }, ensure_ascii=False))
            return 0
        except Exception as exc:
            failures.append({"resource_id": resource_id, "error": str(exc)})

    print(json.dumps({"ok": False, "failures": failures}, ensure_ascii=False), file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
