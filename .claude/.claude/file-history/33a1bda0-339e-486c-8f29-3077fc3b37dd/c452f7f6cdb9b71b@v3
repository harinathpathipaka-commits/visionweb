"""Shared LLM client — raw httpx backend (openai SDK hangs on Windows Python 3.14).

Uses prompt-only JSON instructions (no response_format API flags)
so it works identically on DeepSeek, GPT-4o, Claude, and any compatible API.
"""

from __future__ import annotations

import asyncio
import json
import time
from dataclasses import dataclass, field
from typing import Any

import httpx

from ans_nerves.config import get_api_key, get_vision_api_key, get_config
from ans_nerves.logging import get_logger

logger = get_logger(__name__)

_JSON_INSTRUCTION = "\n\nRespond ONLY with valid JSON. No markdown fences, no extra text."


@dataclass
class TokenUsage:
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0
    cost_cents: float = 0.0


@dataclass
class LLMResponse:
    content: str
    parsed: dict[str, Any] | None = None
    usage: TokenUsage = field(default_factory=TokenUsage)
    model: str = ""
    latency_ms: float = 0.0


class LLMClient:
    """LLM client using raw httpx — reliable on all platforms."""

    def __init__(self) -> None:
        config = get_config()
        self._api_key = get_api_key()
        self._model = config.llm.model
        self._base_url = config.llm.base_url.rstrip("/")
        self._max_tokens = config.llm.max_tokens
        self._temperature = config.llm.temperature
        self._max_retries = config.llm.max_retries
        self._timeout = config.llm.request_timeout_secs
        self._cost_input = config.llm.cost_per_1k_input_tokens
        self._cost_output = config.llm.cost_per_1k_output_tokens

    @property
    def model(self) -> str:
        return self._model

    # ── Public API ──────────────────────────────────────────

    async def complete(
        self,
        system_prompt: str,
        user_prompt: str,
        *,
        json_mode: bool = True,
        max_tokens: int | None = None,
        temperature: float | None = None,
        model_override: str | None = None,
    ) -> LLMResponse:
        system = system_prompt + _JSON_INSTRUCTION if json_mode else system_prompt
        messages = [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt},
        ]
        return await self._chat(
            messages,
            max_tokens=max_tokens or self._max_tokens,
            temperature=temperature if temperature is not None else self._temperature,
            model_override=model_override,
            json_mode=json_mode,
        )

    async def complete_structured(
        self,
        system_prompt: str,
        user_prompt: str,
        json_schema: dict[str, Any],
        *,
        max_tokens: int | None = None,
        temperature: float | None = None,
        model_override: str | None = None,
    ) -> LLMResponse:
        schema_text = json.dumps(json_schema, indent=2)
        system = (
            system_prompt
            + f"\n\nYou MUST respond with valid JSON matching this schema:\n{schema_text}"
            + _JSON_INSTRUCTION
        )
        messages = [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt},
        ]
        return await self._chat(
            messages,
            max_tokens=max_tokens or self._max_tokens,
            temperature=temperature if temperature is not None else self._temperature,
            model_override=model_override,
            json_mode=True,
        )

    async def complete_vision(
        self,
        system_prompt: str,
        user_prompt: str,
        screenshot_base64: str,
        *,
        json_schema: dict[str, Any] | None = None,
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> LLMResponse:
        config = get_config()
        api_key = self._api_key
        base_url = self._base_url
        model = self._model

        if config.llm.vision_provider != config.llm.provider:
            api_key = get_vision_api_key()
            base_url = config.llm.vision_base_url.rstrip("/")
            model = config.llm.vision_model
            # Gemini uses a different API format
            if config.llm.vision_provider == "google":
                return await self._gemini_vision(
                    system_prompt, user_prompt, screenshot_base64,
                    api_key, base_url, model,
                    json_schema, max_tokens, temperature,
                )

        system = system_prompt
        if json_schema is not None:
            schema_text = json.dumps(json_schema, indent=2)
            system += f"\n\nYou MUST respond with valid JSON matching this schema:\n{schema_text}"
        system += _JSON_INSTRUCTION

        user_content = [
            {"type": "text", "text": user_prompt},
            {
                "type": "image_url",
                "image_url": {
                    "url": f"data:image/png;base64,{screenshot_base64}",
                    "detail": "auto",
                },
            },
        ]

        messages = [
            {"role": "system", "content": system},
            {"role": "user", "content": user_content},
        ]

        t0 = time.monotonic()
        body = await self._post(
            base_url, api_key, model,
            messages,
            max_tokens or self._max_tokens,
            temperature if temperature is not None else self._temperature,
        )
        latency_ms = (time.monotonic() - t0) * 1000.0

        return self._build_response(body, model, latency_ms)

    async def _gemini_vision(
        self,
        system_prompt: str,
        user_prompt: str,
        screenshot_base64: str,
        api_key: str,
        base_url: str,
        model: str,
        json_schema: dict[str, Any] | None,
        max_tokens: int,
        temperature: float | None,
    ) -> LLMResponse:
        """Call Gemini Flash for vision via generateContent API."""
        text = system_prompt + "\n\n" + user_prompt
        if json_schema is not None:
            schema_text = json.dumps(json_schema, indent=2)
            text += f"\n\nYou MUST respond with valid JSON matching this schema:\n{schema_text}"
        text += _JSON_INSTRUCTION

        payload = {
            "contents": [{
                "parts": [
                    {"text": text},
                    {"inline_data": {"mime_type": "image/png", "data": screenshot_base64}},
                ],
            }],
            "generationConfig": {
                "maxOutputTokens": max_tokens,
                "temperature": temperature if temperature is not None else 0.3,
            },
        }

        url = f"{base_url}/models/{model}:generateContent?key={api_key}"

        def _do():
            with httpx.Client(timeout=self._timeout) as client:
                r = client.post(url, json=payload)
                r.raise_for_status()
                return r.json()

        t0 = time.monotonic()
        body = await asyncio.to_thread(_do)
        latency_ms = (time.monotonic() - t0) * 1000.0

        # Map Gemini response to OpenAI-compatible format for _build_response
        candidates = body.get("candidates", [{}])
        content_text = candidates[0].get("content", {}).get("parts", [{}])[0].get("text", "") or ""
        usage = body.get("usageMetadata", {})

        # Gemini doesn't report model name the same way
        mapped = {
            "choices": [{"message": {"content": content_text}}],
            "model": model,
            "usage": {
                "prompt_tokens": usage.get("promptTokenCount", 0),
                "completion_tokens": usage.get("candidatesTokenCount", 0),
                "total_tokens": usage.get("totalTokenCount", 0),
            },
        }
        return self._build_response(mapped, model, latency_ms)

    # ── Internals ───────────────────────────────────────────

    async def _chat(
        self,
        messages: list[dict],
        max_tokens: int,
        temperature: float,
        model_override: str | None,
        json_mode: bool,
    ) -> LLMResponse:
        model = model_override or self._model

        t0 = time.monotonic()
        for attempt in range(self._max_retries + 1):
            try:
                body = await self._post(
                    self._base_url, self._api_key, model,
                    messages, max_tokens, temperature,
                )
            except Exception:
                if attempt == self._max_retries:
                    raise
                await asyncio.sleep(min(2 ** attempt, 30))
                continue

            content = (body.get("choices", [{}])[0].get("message", {}).get("content", "") or "")
            if content.strip():
                latency_ms = (time.monotonic() - t0) * 1000.0
                return self._build_response(body, model, latency_ms)

            # Empty response — retry with higher temperature
            current_temp = min(temperature + 0.15 * (attempt + 1), 1.0)
            logger.warning(
                "LLM returned empty response (attempt %d/%d), retrying with temp=%.2f...",
                attempt + 1, self._max_retries + 1, current_temp,
            )
            temperature = current_temp

        latency_ms = (time.monotonic() - t0) * 1000.0
        return self._build_response(body, model, latency_ms)

    async def _post(
        self, base_url: str, api_key: str, model: str,
        messages: list[dict], max_tokens: int, temperature: float,
    ) -> dict:
        url = f"{base_url}/chat/completions"
        payload = {
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
        }
        headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        }

        def _do_post() -> httpx.Response:
            with httpx.Client(timeout=self._timeout) as client:
                r = client.post(url, headers=headers, json=payload)
                r.raise_for_status()
                return r

        response = await asyncio.to_thread(_do_post)
        return response.json()

    def _build_response(self, body: dict, model: str, latency_ms: float) -> LLMResponse:
        choice = body.get("choices", [{}])[0]
        content = choice.get("message", {}).get("content", "") or ""
        usage_raw = body.get("usage", {})

        usage = TokenUsage(
            prompt_tokens=usage_raw.get("prompt_tokens", 0),
            completion_tokens=usage_raw.get("completion_tokens", 0),
            total_tokens=usage_raw.get("total_tokens", 0),
            cost_cents=round(
                (usage_raw.get("prompt_tokens", 0) * self._cost_input
                 + usage_raw.get("completion_tokens", 0) * self._cost_output)
                / 1000.0 * 100.0, 4,
            ),
        )

        return LLMResponse(
            content=content,
            parsed=self._parse_json(content),
            usage=usage,
            model=body.get("model", model),
            latency_ms=latency_ms,
        )

    # ── JSON parsing ────────────────────────────────────────

    @staticmethod
    def _strip_fences(content: str) -> str:
        cleaned = content.strip()
        if cleaned.startswith("```"):
            lines = cleaned.split("\n")
            if lines[0].startswith("```"):
                lines = lines[1:]
            if lines and lines[-1].strip() == "```":
                lines = lines[:-1]
            cleaned = "\n".join(lines).strip()
        return cleaned

    @classmethod
    def _parse_json(cls, content: str) -> dict[str, Any] | None:
        if not content.strip():
            return None
        cleaned = cls._strip_fences(content)
        try:
            return json.loads(cleaned)
        except json.JSONDecodeError:
            pass
        repaired = cls._repair_json(cleaned)
        if repaired:
            try:
                return json.loads(repaired)
            except json.JSONDecodeError:
                pass
        return None

    @staticmethod
    def _repair_json(text: str) -> str | None:
        import re
        text = re.sub(r',\s*([}\]])', r'\1', text)
        first_brace = text.find('{')
        first_bracket = text.find('[')
        if first_brace == -1 and first_bracket == -1:
            return None
        start = first_brace if first_brace != -1 else first_bracket
        end = text.rfind('}') if first_brace != -1 else text.rfind(']')
        if end > start:
            text = text[start:end + 1]
        open_braces = text.count('{') - text.count('}')
        open_brackets = text.count('[') - text.count(']')
        if 0 < open_braces + open_brackets <= 3:
            text += ']' * open_brackets
            text += '}' * open_braces
        elif open_braces + open_brackets > 3:
            return None
        return text


# Global singleton
_client: LLMClient | None = None


def get_llm_client() -> LLMClient:
    global _client
    if _client is None:
        _client = LLMClient()
    return _client
