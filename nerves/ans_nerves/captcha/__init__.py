"""CAPTCHA solving integration for ANS.

When the Vision Eye detects a CAPTCHA (page_type=captcha), the solver:
1. Extracts the reCAPTCHA/hCaptcha sitekey from page DOM
2. Calls CapSolver API (createTask → poll → get token)
3. Injects the token into the page via CDP evaluate
4. Returns control to the agent loop to resume execution

Supported: reCAPTCHA v2, reCAPTCHA v3, hCaptcha, Cloudflare Turnstile.
"""

from ans_nerves.captcha.solver import CapSolverClient, CaptchaToken

__all__ = ["CapSolverClient", "CaptchaToken"]
