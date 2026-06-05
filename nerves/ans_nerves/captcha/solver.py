"""CapSolver API integration for automated CAPTCHA solving.

Flow:
  1. CAPTCHA detected by Vision Eye (page_type=captcha)
  2. Extract sitekey from page DOM via regex or CDP evaluate
  3. Create CapSolver task with sitekey + page URL
  4. Poll for result (usually 10-30s)
  5. Inject gReCaptchaResponse token into page
  6. Submit the form / trigger the callback
"""

from __future__ import annotations

import asyncio
import os
import re
import time
from dataclasses import dataclass, field
from typing import Any

import httpx

from ans_nerves.logging import get_logger

logger = get_logger(__name__)

# ── Constants ─────────────────────────────────────────────

CAPSOLVER_API = "https://api.capsolver.com"
CREATE_TASK_URL = f"{CAPSOLVER_API}/createTask"
GET_RESULT_URL = f"{CAPSOLVER_API}/getTaskResult"

# reCAPTCHA sitekey patterns
RECAPTCHA_SITEKEY_RE = re.compile(
    r'(?:data-sitekey|sitekey|site_key|"sitekey")\s*[=:]\s*["\']([A-Za-z0-9_-]{30,50})["\']',
    re.IGNORECASE,
)

# hCaptcha
HCAPTCHA_SITEKEY_RE = re.compile(
    r'(?:data-hcaptcha-sitekey|data-sitekey)\s*=\s*["\']([A-Za-z0-9_-]{20,60})["\']',
    re.IGNORECASE,
)

# Cloudflare Turnstile
TURNSTILE_SITEKEY_RE = re.compile(
    r'(?:data-turnstile-sitekey|turnstile\.render)\s*[\(=]\s*["\']([A-Za-z0-9_-]{20,60})["\']',
    re.IGNORECASE,
)

RESULT_POLL_INTERVAL = 1.0  # seconds
RESULT_MAX_WAIT = 60.0  # seconds
DEFAULT_CAPTCHA_TYPE = "ReCaptchaV2TaskProxyLess"


@dataclass
class CaptchaToken:
    """A solved CAPTCHA token ready for injection."""

    token: str
    captcha_type: str
    sitekey: str
    solve_time_ms: int = 0

    def is_valid(self) -> bool:
        return bool(self.token) and len(self.token) > 20


@dataclass
class CapSolverClient:
    """Stateless CapSolver API client.

    Reads CAPSOLVER_API_KEY from environment.
    Falls back gracefully if key is missing.
    """

    api_key: str = field(default_factory=lambda: os.getenv("CAPSOLVER_API_KEY", ""))
    _client: httpx.AsyncClient | None = field(default=None, init=False)

    @property
    def configured(self) -> bool:
        return bool(self.api_key)

    # ── Public API ──────────────────────────────────────────

    async def solve_captcha(
        self,
        page_url: str,
        dom_html: str = "",
        captcha_type: str = "",
    ) -> CaptchaToken | None:
        """Attempt to detect and solve a CAPTCHA on the page.

        Returns CaptchaToken on success, None if unsolvable or not configured.
        """
        if not self.configured:
            logger.warning("captcha: CAPSOLVER_API_KEY not set, cannot auto-solve")
            return None

        # 1. Detect sitekey from DOM
        sitekey = self._extract_sitekey(dom_html, captcha_type)
        if not sitekey:
            logger.warning("captcha: could not extract sitekey from DOM")
            return None

        # 2. Determine task type
        task_type = self._detect_task_type(dom_html, captcha_type)

        logger.info(
            "captcha: solving type=%s sitekey=%s url=%s",
            task_type, sitekey[:8] + "...", page_url[:60],
        )

        start = time.monotonic()

        # 3. Create CapSolver task
        task_id = await self._create_task(task_type, sitekey, page_url)
        if not task_id:
            return None

        # 4. Poll for result
        token = await self._poll_result(task_id)
        if not token:
            return None

        elapsed = int((time.monotonic() - start) * 1000)
        logger.info("captcha: solved in %dms (type=%s)", elapsed, task_type)

        return CaptchaToken(
            token=token,
            captcha_type=task_type,
            sitekey=sitekey,
            solve_time_ms=elapsed,
        )

    def build_injection_script(self, token: CaptchaToken) -> str:
        """Build a JS snippet that injects the solved token into the page.

        For reCAPTCHA: sets grecaptcha.getResponse() return value and
        calls the data-callback if one exists.
        """
        if "recaptcha" in token.captcha_type.lower():
            return self._recaptcha_injection(token)
        elif "hcaptcha" in token.captcha_type.lower():
            return self._hcaptcha_injection(token)
        elif "turnstile" in token.captcha_type.lower():
            return self._turnstile_injection(token)
        else:
            return self._recaptcha_injection(token)

    # ── Detection ───────────────────────────────────────────

    @staticmethod
    def _extract_sitekey(dom_html: str, captcha_type: str = "") -> str:
        """Extract the CAPTCHA sitekey from DOM HTML."""
        if not dom_html:
            return ""

        for pattern in [RECAPTCHA_SITEKEY_RE, HCAPTCHA_SITEKEY_RE, TURNSTILE_SITEKEY_RE]:
            m = pattern.search(dom_html)
            if m:
                return m.group(1)
        return ""

    @staticmethod
    def _detect_task_type(dom_html: str, captcha_type: str = "") -> str:
        """Determine which CapSolver task type to use."""
        ct = captcha_type.lower()
        if "hcaptcha" in ct or "h-captcha" in ct:
            return "HCaptchaTaskProxyLess"
        if "turnstile" in ct or "cloudflare" in ct:
            return "TurnstileTaskProxyLess"
        if "recaptcha" in ct or "g-recaptcha" in ct:
            if "v3" in ct or "enterprise" in ct:
                return "ReCaptchaV3TaskProxyLess"
            return "ReCaptchaV2TaskProxyLess"

        # Auto-detect from DOM
        dom_lower = dom_html.lower()
        if "hcaptcha" in dom_lower:
            return "HCaptchaTaskProxyLess"
        if "turnstile" in dom_lower or "cf-turnstile" in dom_lower:
            return "TurnstileTaskProxyLess"
        return DEFAULT_CAPTCHA_TYPE

    # ── CapSolver API calls ─────────────────────────────────

    async def _get_client(self) -> httpx.AsyncClient:
        if self._client is None:
            self._client = httpx.AsyncClient(timeout=httpx.Timeout(30.0))
        return self._client

    async def _create_task(
        self, task_type: str, sitekey: str, page_url: str
    ) -> str | None:
        """Create a CapSolver task. Returns task_id or None."""
        payload: dict[str, Any] = {
            "clientKey": self.api_key,
            "task": {
                "type": task_type,
                "websiteKey": sitekey,
                "websiteURL": page_url,
            },
        }

        try:
            client = await self._get_client()
            resp = await client.post(CREATE_TASK_URL, json=payload)
            data = resp.json()

            if data.get("errorId") not in (None, 0):
                error = data.get("errorDescription", data.get("errorCode", "unknown"))
                logger.error("captcha: CapSolver createTask error: %s", error)
                return None

            task_id = data.get("taskId")
            if not task_id:
                logger.error("captcha: no taskId in CapSolver response: %s", data)
                return None

            logger.info("captcha: CapSolver task created id=%s", task_id)
            return str(task_id)

        except httpx.HTTPError as e:
            logger.error("captcha: CapSolver HTTP error: %s", e)
            return None

    async def _poll_result(self, task_id: str) -> str | None:
        """Poll for the CAPTCHA solution token. Returns token or None."""
        deadline = time.monotonic() + RESULT_MAX_WAIT
        payload = {"clientKey": self.api_key, "taskId": task_id}

        while time.monotonic() < deadline:
            await asyncio.sleep(RESULT_POLL_INTERVAL)

            try:
                client = await self._get_client()
                resp = await client.post(GET_RESULT_URL, json=payload)
                data = resp.json()

                if data.get("errorId") not in (None, 0):
                    error = data.get("errorDescription", data.get("errorCode", "unknown"))
                    logger.error("captcha: CapSolver getTaskResult error: %s", error)
                    return None

                status = data.get("status", "")
                if status == "ready":
                    solution = data.get("solution", {})
                    token = (
                        solution.get("gRecaptchaResponse")
                        or solution.get("token")
                        or solution.get("cf_clearance")
                        or ""
                    )
                    if token:
                        return token
                    logger.error("captcha: no token in solution: %s", data)
                    return None

                if status == "failed":
                    logger.error("captcha: CapSolver task failed: %s", data)
                    return None

                # Still processing — continue polling
                logger.debug("captcha: polling task %s status=%s", task_id, status)

            except httpx.HTTPError as e:
                logger.warning("captcha: poll HTTP error (retrying): %s", e)

        logger.error("captcha: timed out waiting for solution (task=%s)", task_id)
        return None

    # ── Token injection scripts ─────────────────────────────

    def _recaptcha_injection(self, token: CaptchaToken) -> str:
        """Inject reCAPTCHA response token into the page."""
        return f"""
(function() {{
    var token = '{token.token}';
    // Set the response textarea value (standard reCAPTCHA)
    var textarea = document.querySelector('textarea[name="g-recaptcha-response"], #g-recaptcha-response');
    if (textarea) {{
        textarea.value = token;
        textarea.style.display = 'block';
    }}
    // Call the site's data-callback if one exists
    var widget = document.querySelector('.g-recaptcha, [data-sitekey="{token.sitekey}"]');
    if (widget) {{
        var callback = widget.getAttribute('data-callback');
        if (callback && typeof window[callback] === 'function') {{
            window[callback](token);
            return 'callback_fired';
        }}
    }}
    // Try grecaptcha object
    if (typeof grecaptcha !== 'undefined' && grecaptcha.getResponse) {{
        // Override getResponse to return our token
        var origGetResponse = grecaptcha.getResponse;
        grecaptcha.getResponse = function(widget_id) {{
            return token;
        }};
        // Also call the global callback on the first widget
        try {{
            var widgets = grecaptcha.render && typeof grecaptcha.$$widgets !== 'undefined' ?
                Object.keys(grecaptcha.$$widgets || {{}}) : [];
            if (widgets.length > 0) {{
                var cb = grecaptcha.$$widgets[widgets[0]].callback;
                if (cb) {{ cb(token); return 'widget_callback_fired'; }}
            }}
        }} catch(e) {{}}
    }}
    return 'token_injected';
}})();
"""

    def _hcaptcha_injection(self, token: CaptchaToken) -> str:
        """Inject hCaptcha response token."""
        return f"""
(function() {{
    var token = '{token.token}';
    var textarea = document.querySelector('textarea[name="h-captcha-response"], textarea[name="g-recaptcha-response"]');
    if (textarea) {{
        textarea.value = token;
        return 'hcaptcha_token_injected';
    }}
    if (typeof hcaptcha !== 'undefined') {{
        hcaptcha.getResponse = function() {{ return token; }};
        return 'hcaptcha_override_injected';
    }}
    return 'no_hcaptcha_found';
}})();
"""

    def _turnstile_injection(self, token: CaptchaToken) -> str:
        """Inject Cloudflare Turnstile token."""
        return f"""
(function() {{
    var token = '{token.token}';
    var input = document.querySelector('input[name="cf-turnstile-response"]');
    if (input) {{
        input.value = token;
        return 'turnstile_injected';
    }}
    if (typeof turnstile !== 'undefined') {{
        turnstile.render = function() {{ return 'patched'; }};
        return 'turnstile_patched';
    }}
    return 'no_turnstile_found';
}})();
"""
