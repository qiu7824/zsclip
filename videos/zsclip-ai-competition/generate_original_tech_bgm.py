import argparse
import math
import wave
from pathlib import Path

import numpy as np


SAMPLE_RATE = 48_000
BPM = 92
BEAT = 60.0 / BPM
CHUNK_SECONDS = 4


def smoothstep(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


def note_freq(midi):
    return 440.0 * (2.0 ** ((midi - 69) / 12.0))


CHORDS = np.array(
    [
        [note_freq(45), note_freq(52), note_freq(57), note_freq(60)],
        [note_freq(41), note_freq(48), note_freq(53), note_freq(57)],
        [note_freq(48), note_freq(55), note_freq(60), note_freq(64)],
        [note_freq(43), note_freq(50), note_freq(55), note_freq(59)],
    ],
    dtype=np.float64,
)

ARP_DEGREES = np.array([0, 1, 2, 1, 3, 2, 1, 2, 0, 2, 3, 2, 1, 0, 2, 1])


def synth_chunk(start_sample, count, total_samples, rng):
    idx = np.arange(start_sample, start_sample + count, dtype=np.float64)
    t = idx / SAMPLE_RATE
    duration = total_samples / SAMPLE_RATE

    fade_in = smoothstep(t / 3.2)
    fade_out = smoothstep((duration - t) / 6.0)
    master = np.minimum(fade_in, fade_out)

    beat_pos = t / BEAT
    beat_index = np.floor(beat_pos).astype(np.int64)
    beat_phase = np.mod(t, BEAT)
    chord_index = (np.floor(beat_pos / 8).astype(np.int64)) % len(CHORDS)
    chord = CHORDS[chord_index]

    pad_l = np.zeros_like(t)
    pad_r = np.zeros_like(t)
    for i in range(4):
        freq = chord[:, i]
        layer = np.sin(2 * math.pi * freq * t)
        overtone = 0.28 * np.sin(2 * math.pi * freq * 2.01 * t + i * 0.7)
        pad_l += (layer + overtone) * (0.020 - i * 0.002)
        pad_r += (np.sin(2 * math.pi * freq * (t + 0.011 + i * 0.002)) + overtone) * (
            0.020 - i * 0.002
        )
    pad_l *= 0.76 + 0.24 * np.sin(2 * math.pi * t / 12.0)
    pad_r *= 0.76 + 0.24 * np.sin(2 * math.pi * (t / 12.0 + 0.19))

    root = chord[:, 0] / 2
    duck = 1.0 - 0.18 * np.exp(-beat_phase * 6.5)
    bass = (
        np.sin(2 * math.pi * root * t)
        + 0.22 * np.sin(2 * math.pi * root * 2.0 * t)
    ) * 0.055 * duck

    arp_step = BEAT / 2
    arp_index = np.floor(t / arp_step).astype(np.int64)
    arp_phase = np.mod(t, arp_step) / arp_step
    degree = ARP_DEGREES[arp_index % len(ARP_DEGREES)]
    arp_freq = chord[np.arange(count), degree] * 2.0
    arp_env = np.exp(-arp_phase * 6.0) * smoothstep(arp_phase / 0.08)
    arp = (np.sin(2 * math.pi * arp_freq * t) + 0.35 * np.sin(2 * math.pi * arp_freq * 2 * t)) * 0.035 * arp_env
    arp_pan = ((arp_index % 4) / 3.0) * 0.52 + 0.24

    kick_phase = beat_phase
    kick_mask = ((beat_index % 4) == 0) & (kick_phase < 0.24)
    kick_integral = 52.0 * kick_phase + 120.0 * (1.0 - np.exp(-kick_phase * 18.0)) / 18.0
    kick = np.where(
        kick_mask,
        np.sin(2 * math.pi * kick_integral) * np.exp(-kick_phase * 18.0) * 0.16,
        0.0,
    )

    tick_phase = np.mod(t + BEAT * 0.5, BEAT)
    tick_mask = tick_phase < 0.045
    noise = rng.normal(0.0, 1.0, count)
    tick = np.where(tick_mask, noise * np.exp(-tick_phase * 95.0) * 0.010, 0.0)

    shimmer = (
        np.sin(2 * math.pi * note_freq(81) * t + np.sin(2 * math.pi * t / 9.0))
        + 0.5 * np.sin(2 * math.pi * note_freq(88) * t)
    ) * 0.006 * (0.5 + 0.5 * np.sin(2 * math.pi * t / 16.0))

    left = (pad_l + bass + kick + tick + arp * (1.0 - arp_pan) + shimmer) * master
    right = (pad_r + bass + kick + tick * 0.82 + arp * arp_pan + shimmer * 0.92) * master
    stereo = np.stack([left, right], axis=1)
    return np.clip(stereo * 0.92, -0.98, 0.98)


def generate(output: Path, duration: float):
    total_samples = int(round(duration * SAMPLE_RATE))
    chunk_samples = CHUNK_SECONDS * SAMPLE_RATE
    rng = np.random.default_rng(7824)
    output.parent.mkdir(parents=True, exist_ok=True)

    with wave.open(str(output), "wb") as wav:
        wav.setnchannels(2)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        for start in range(0, total_samples, chunk_samples):
            count = min(chunk_samples, total_samples - start)
            chunk = synth_chunk(start, count, total_samples, rng)
            pcm = (chunk * 32767.0).astype("<i2")
            wav.writeframes(pcm.tobytes())


def main():
    parser = argparse.ArgumentParser(description="Generate an original soft tech BGM for ZSClip videos.")
    parser.add_argument("--duration", type=float, default=216.0)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    generate(Path(args.output), args.duration)
    print(Path(args.output).resolve())


if __name__ == "__main__":
    main()
