import argparse
import base64
import json
import os
import sys
import urllib.error
import urllib.request
import uuid
from pathlib import Path


TTS_ENDPOINT = "https://openspeech.bytedance.com/api/v3/tts/unidirectional"
VOICE_ENDPOINT = "https://openspeech.bytedance.com/api/v3/tts/get_voice"


def build_auth_headers(args):
    if args.api_key.strip():
        return {"X-Api-Key": args.api_key.strip()}, "api-key"
    if args.app_id.strip() and args.access_key.strip():
        return {
            "X-Api-App-Id": args.app_id.strip(),
            "X-Api-Access-Key": args.access_key.strip(),
        }, "app-id/access-key"
    return {}, "missing"


def parse_chunked_json(raw):
    decoder = json.JSONDecoder()
    index = 0
    while index < len(raw):
        while index < len(raw) and raw[index] in " \r\n\t":
            index += 1
        if index >= len(raw):
            break
        obj, index = decoder.raw_decode(raw, index)
        yield obj


def post_json(endpoint, payload, headers):
    request = urllib.request.Request(
        endpoint,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        method="POST",
        headers=headers,
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            body = response.read()
            return {
                "http_status": response.status,
                "headers": {
                    "content_type": response.headers.get("Content-Type", ""),
                    "x_tt_logid": response.headers.get("X-Tt-Logid")
                    or response.headers.get("X-Tt-LogId")
                    or "",
                },
                "body": body,
            }
    except urllib.error.HTTPError as exc:
        return {
            "http_status": exc.code,
            "headers": {
                "content_type": exc.headers.get("Content-Type", ""),
                "x_tt_logid": exc.headers.get("X-Tt-Logid")
                or exc.headers.get("X-Tt-LogId")
                or "",
            },
            "body": exc.read(),
        }
    except Exception as exc:
        return {"exception": repr(exc), "headers": {}, "body": b""}


def summarize_tts_body(body):
    text = body.decode("utf-8", errors="replace")
    summary = {
        "raw_preview": text[:600],
        "json_events": 0,
        "codes": [],
        "messages": [],
        "audio_chunks": 0,
        "audio_bytes": 0,
        "parse_error": "",
    }
    try:
        for obj in parse_chunked_json(text):
            summary["json_events"] += 1
            if "code" in obj:
                summary["codes"].append(obj.get("code"))
            message = obj.get("message") or obj.get("msg")
            if message:
                summary["messages"].append(str(message))
            data = obj.get("data")
            if isinstance(data, str) and data:
                summary["audio_chunks"] += 1
                try:
                    summary["audio_bytes"] += len(base64.b64decode(data))
                except Exception:
                    pass
    except Exception as exc:
        summary["parse_error"] = repr(exc)
    summary["codes"] = sorted(set(summary["codes"]), key=str)
    summary["messages"] = list(dict.fromkeys(summary["messages"]))
    return summary


def diagnose_voice(auth_headers, auth_kind, speaker, resource_id):
    headers = {
        "Content-Type": "application/json",
        "X-Api-Resource-Id": resource_id,
        "X-Api-Request-Id": str(uuid.uuid4()),
        **auth_headers,
    }
    result = post_json(VOICE_ENDPOINT, {"speaker_id": speaker}, headers)
    body_text = result.pop("body", b"").decode("utf-8", errors="replace")
    try:
        parsed = json.loads(body_text)
    except Exception:
        parsed = None
    return {
        "speaker": speaker,
        "resource_id": resource_id,
        "auth": auth_kind,
        **result,
        "body": parsed if parsed is not None else body_text[:1000],
    }


def diagnose_synthesis(auth_headers, auth_kind, speaker, resource_id, text, explicit_language):
    req_params = {
        "text": text,
        "speaker": speaker,
        "audio_params": {
            "format": "mp3",
            "sample_rate": 24000,
            "speech_rate": 0,
            "loudness_rate": 0,
        },
    }
    if explicit_language:
        req_params["additions"] = json.dumps(
            {"explicit_language": explicit_language},
            ensure_ascii=False,
        )
    headers = {
        "Content-Type": "application/json",
        "X-Api-Resource-Id": resource_id,
        "X-Api-Request-Id": str(uuid.uuid4()),
        **auth_headers,
    }
    result = post_json(
        TTS_ENDPOINT,
        {"user": {"uid": "zsclip-diagnose"}, "req_params": req_params},
        headers,
    )
    body = result.pop("body", b"")
    return {
        "speaker": speaker,
        "resource_id": resource_id,
        "auth": auth_kind,
        **result,
        "body_summary": summarize_tts_body(body),
    }


def main():
    parser = argparse.ArgumentParser(description="Diagnose Volcengine TTS auth/resource/speaker errors.")
    parser.add_argument("--api-key", default=os.environ.get("VOLCENGINE_TTS_API_KEY", ""))
    parser.add_argument("--app-id", default=os.environ.get("VOLCENGINE_TTS_APP_ID", ""))
    parser.add_argument("--access-key", default=os.environ.get("VOLCENGINE_TTS_ACCESS_KEY", ""))
    parser.add_argument("--speakers", default="S_bBxWfXhO1,S_0hFVfXhO1")
    parser.add_argument("--resources", default="seed-icl-2.0,seed-icl-1.0,seed-tts-2.0,seed-tts-1.0")
    parser.add_argument("--voice-query-resource", default="seed-icl-2.0")
    parser.add_argument("--text", default="你好，这是 ZSClip 音色接口测试。")
    parser.add_argument("--explicit-language", default="zh-cn")
    parser.add_argument("--output", default="videos/zsclip-ai-competition/output/volcengine-tts-diagnosis.json")
    args = parser.parse_args()

    auth_headers, auth_kind = build_auth_headers(args)
    speakers = [part.strip() for part in args.speakers.split(",") if part.strip()]
    resources = [part.strip() for part in args.resources.split(",") if part.strip()]
    report = {
        "ok": False,
        "all_synthesis_ok": False,
        "auth": auth_kind,
        "has_auth": bool(auth_headers),
        "voice_status": [],
        "synthesis": [],
    }
    if not auth_headers:
        report["error"] = (
            "Missing auth. Set VOLCENGINE_TTS_API_KEY, or set "
            "VOLCENGINE_TTS_APP_ID and VOLCENGINE_TTS_ACCESS_KEY."
        )
    else:
        for speaker in speakers:
            report["voice_status"].append(
                diagnose_voice(auth_headers, auth_kind, speaker, args.voice_query_resource)
            )
        for speaker in speakers:
            for resource_id in resources:
                item = diagnose_synthesis(
                    auth_headers,
                    auth_kind,
                    speaker,
                    resource_id,
                    args.text,
                    args.explicit_language,
                )
                report["synthesis"].append(item)
                if item.get("http_status") == 200 and item["body_summary"]["audio_chunks"] > 0:
                    report["ok"] = True
        report["all_synthesis_ok"] = bool(report["synthesis"]) and all(
            item.get("http_status") == 200 and item["body_summary"]["audio_chunks"] > 0
            for item in report["synthesis"]
        )

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(str(output.resolve()))
    print(json.dumps({
        "ok": report["ok"],
        "all_synthesis_ok": report["all_synthesis_ok"],
        "auth": report["auth"],
        "has_auth": report["has_auth"],
        "synthesis_results": [
            {
                "speaker": item.get("speaker"),
                "resource_id": item.get("resource_id"),
                "http_status": item.get("http_status"),
                "x_tt_logid": item.get("headers", {}).get("x_tt_logid"),
                "codes": item.get("body_summary", {}).get("codes", []),
                "messages": item.get("body_summary", {}).get("messages", []),
                "audio_chunks": item.get("body_summary", {}).get("audio_chunks", 0),
                "audio_bytes": item.get("body_summary", {}).get("audio_bytes", 0),
            }
            for item in report["synthesis"]
        ],
    }, ensure_ascii=False))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
