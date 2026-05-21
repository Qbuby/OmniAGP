import numpy as np
import pyloudnorm as pyln
import soundfile as sf
from scipy.signal import correlate


def normalize_loudness(audio: np.ndarray, sample_rate: int, target_lufs: float) -> np.ndarray:
    meter = pyln.Meter(sample_rate)
    current_lufs = meter.integrated_loudness(audio)
    if np.isinf(current_lufs):
        return audio
    return pyln.normalize.loudness(audio, current_lufs, target_lufs)


def detect_loop_point(audio: np.ndarray, sample_rate: int, min_offset_sec: float = 5.0) -> int:
    min_offset = int(min_offset_sec * sample_rate)
    if len(audio.shape) > 1:
        mono = audio.mean(axis=1)
    else:
        mono = audio

    segment_len = int(2.0 * sample_rate)
    head = mono[:segment_len]
    search_region = mono[min_offset:]

    correlation = correlate(search_region, head, mode="valid")
    best_offset = int(np.argmax(correlation)) + min_offset
    return best_offset


def apply_crossfade(audio: np.ndarray, loop_point: int, fade_samples: int) -> np.ndarray:
    if loop_point + fade_samples > len(audio):
        return audio[:loop_point]

    fade_out = np.linspace(1.0, 0.0, fade_samples)
    fade_in = np.linspace(0.0, 1.0, fade_samples)

    if len(audio.shape) > 1:
        fade_out = fade_out[:, np.newaxis]
        fade_in = fade_in[:, np.newaxis]

    looped = audio[:loop_point].copy()
    tail = audio[loop_point : loop_point + fade_samples]
    head = audio[:fade_samples]

    looped[-fade_samples:] = looped[-fade_samples:] * fade_out + head * fade_in
    return looped


def detect_clipping(audio: np.ndarray, threshold: float = 0.99) -> bool:
    return bool(np.any(np.abs(audio) > threshold))


def detect_silence(audio: np.ndarray, threshold_db: float = -60.0) -> bool:
    rms = np.sqrt(np.mean(audio**2))
    if rms == 0:
        return True
    db = 20 * np.log10(rms)
    return bool(db < threshold_db)


def validate_audio(audio: np.ndarray, sample_rate: int, min_duration_sec: float = 1.0) -> dict:
    duration = len(audio) / sample_rate
    issues = []

    if duration < min_duration_sec:
        issues.append(f"too_short: {duration:.2f}s < {min_duration_sec}s")
    if detect_clipping(audio):
        issues.append("clipping_detected")
    if detect_silence(audio):
        issues.append("silence_detected")

    return {"valid": len(issues) == 0, "duration_sec": duration, "issues": issues}
