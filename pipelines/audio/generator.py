from __future__ import annotations

import io
import uuid
from enum import Enum
from pathlib import Path
from typing import Optional

import numpy as np
import soundfile as sf
import torch

from .config import AudioPipelineConfig
from .postprocess import (
    apply_crossfade,
    detect_loop_point,
    normalize_loudness,
    validate_audio,
)


class AudioType(str, Enum):
    BGM = "bgm"
    SFX = "sfx"


class AudioGenerator:
    def __init__(self, config: AudioPipelineConfig | None = None):
        self.config = config or AudioPipelineConfig()
        self._musicgen = None
        self._audiogen = None

    def _load_musicgen(self):
        if self._musicgen is not None:
            return
        from audiocraft.models import MusicGen

        self._musicgen = MusicGen.get_pretrained(
            self.config.musicgen_model, device=self.config.device
        )
        if self.config.dtype == "float16":
            self._musicgen.lm = self._musicgen.lm.half()

    def _load_audiogen(self):
        if self._audiogen is not None:
            return
        from audiocraft.models import AudioGen

        self._audiogen = AudioGen.get_pretrained(
            self.config.audiogen_model, device=self.config.device
        )
        if self.config.dtype == "float16":
            self._audiogen.lm = self._audiogen.lm.half()

    def _unload_musicgen(self):
        if self._musicgen is not None:
            del self._musicgen
            self._musicgen = None
            torch.cuda.empty_cache()

    def _unload_audiogen(self):
        if self._audiogen is not None:
            del self._audiogen
            self._audiogen = None
            torch.cuda.empty_cache()

    def generate(
        self,
        prompt: str,
        audio_type: AudioType,
        duration_sec: Optional[float] = None,
        output_dir: Optional[str] = None,
    ) -> dict:
        if audio_type == AudioType.BGM:
            return self._generate_bgm(prompt, duration_sec, output_dir)
        else:
            return self._generate_sfx(prompt, duration_sec, output_dir)

    def _generate_bgm(
        self, prompt: str, duration_sec: Optional[float], output_dir: Optional[str]
    ) -> dict:
        duration = duration_sec or self.config.default_duration_sec
        self._unload_audiogen()
        self._load_musicgen()

        self._musicgen.set_generation_params(duration=duration)
        with torch.no_grad():
            wav = self._musicgen.generate([prompt])

        audio = wav[0].cpu().numpy()
        if audio.ndim == 2:
            audio = audio.T
        else:
            audio = audio.reshape(-1, 1)

        sample_rate = self._musicgen.sample_rate

        audio = normalize_loudness(audio, sample_rate, self.config.bgm_target_lufs)

        fade_samples = int(self.config.fade_duration_sec * sample_rate)
        loop_point = detect_loop_point(audio, sample_rate)
        audio = apply_crossfade(audio, loop_point, fade_samples)

        validation = validate_audio(audio, sample_rate)
        output_path = self._save_output(audio, sample_rate, "bgm", output_dir)

        return {
            "file_path": str(output_path),
            "audio_type": "bgm",
            "duration_sec": len(audio) / sample_rate,
            "sample_rate": sample_rate,
            "loop_point_samples": loop_point,
            "validation": validation,
        }

    def _generate_sfx(
        self, prompt: str, duration_sec: Optional[float], output_dir: Optional[str]
    ) -> dict:
        duration = duration_sec or self.config.sfx_duration_sec
        self._unload_musicgen()
        self._load_audiogen()

        self._audiogen.set_generation_params(duration=duration)
        with torch.no_grad():
            wav = self._audiogen.generate([prompt])

        audio = wav[0].cpu().numpy()
        if audio.ndim == 2:
            audio = audio.T
        else:
            audio = audio.reshape(-1, 1)

        sample_rate = self._audiogen.sample_rate

        audio = normalize_loudness(audio, sample_rate, self.config.sfx_target_lufs)

        validation = validate_audio(audio, sample_rate)
        output_path = self._save_output(audio, sample_rate, "sfx", output_dir)

        return {
            "file_path": str(output_path),
            "audio_type": "sfx",
            "duration_sec": len(audio) / sample_rate,
            "sample_rate": sample_rate,
            "validation": validation,
        }

    def _save_output(
        self, audio: np.ndarray, sample_rate: int, prefix: str, output_dir: Optional[str]
    ) -> Path:
        out_dir = Path(output_dir) if output_dir else Path("output")
        out_dir.mkdir(parents=True, exist_ok=True)

        filename = f"{prefix}_{uuid.uuid4().hex[:8]}.{self.config.output_format}"
        output_path = out_dir / filename

        if self.config.output_format == "ogg":
            sf.write(str(output_path), audio, sample_rate, format="OGG", subtype="VORBIS")
        else:
            sf.write(str(output_path), audio, sample_rate)

        return output_path
