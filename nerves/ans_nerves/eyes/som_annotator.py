"""Set-of-Marks (SOM) screenshot annotator.

Draws numbered bounding boxes on screenshots so the LLM can reference
elements by their visual index — the same approach used by browser-use
and other SoM-based web agents.

Elements are drawn as:
  - Filled circle with white number at the center of each interactive element
  - Thin colored border around the element's bounding box
  - Semi-transparent label with the element_index next to the circle
"""

from __future__ import annotations

import base64
import io
from typing import Any

from PIL import Image, ImageDraw, ImageFont

from ans_nerves.logging import get_logger

logger = get_logger(__name__)

# Per-element-type colors (dark background for contrast with circle + number)
_ELEMENT_COLORS: dict[str, tuple[int, int, int]] = {
    "input": (255, 80, 80),       # red
    "button": (80, 180, 80),      # green
    "link": (80, 140, 255),       # blue
    "select": (255, 200, 60),     # amber
    "textarea": (255, 140, 60),   # orange
    "checkbox": (200, 120, 255),  # purple
    "radio": (255, 120, 200),     # pink
}
_DEFAULT_COLOR = (160, 160, 160)  # grey

_CIRCLE_RADIUS = 14
_BORDER_WIDTH = 2
_LABEL_OFFSET_X = 18
_LABEL_OFFSET_Y = -8

# Cache the font so we don't load it per-call.
_font_cache: ImageFont.FreeTypeFont | None = None


def _get_font(size: int = 14) -> ImageFont.FreeTypeFont:
    """Return a suitable font, falling back to PIL default."""
    global _font_cache
    if _font_cache is not None:
        return _font_cache
    for candidate in ("arial.ttf", "segoeui.ttf", "DejaVuSans.ttf", "C:\\Windows\\Fonts\\segoeui.ttf"):
        try:
            _font_cache = ImageFont.truetype(candidate, size)
            return _font_cache
        except OSError:
            continue
    _font_cache = ImageFont.load_default()
    return _font_cache


def annotate_screenshot(
    screenshot_b64: str,
    interactive_elements: list[dict[str, Any]],
) -> str:
    """Draw numbered Set-of-Marks overlays on a base64 screenshot.

    Args:
        screenshot_b64: Base64-encoded PNG screenshot.
        interactive_elements: List of dicts, each must have:
            - ``element_index`` (int)
            - ``bounding_box`` (dict or None with x, y, width, height)
            - ``element_type`` (str, optional — for color coding)

    Returns:
        Base64-encoded annotated PNG screenshot, or the original if
        annotation fails.
    """
    if not screenshot_b64 or not interactive_elements:
        return screenshot_b64

    try:
        img_data = base64.b64decode(screenshot_b64)
        img = Image.open(io.BytesIO(img_data)).convert("RGBA")
    except Exception:
        logger.warning("som_annotator: failed to decode screenshot", exc_info=True)
        return screenshot_b64

    # Draw on a transparent overlay then composite for anti-aliased edges.
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay)
    font = _get_font(14)
    small_font = _get_font(10)

    for el in interactive_elements:
        idx = el.get("element_index")
        if idx is None:
            continue
        bb = el.get("bounding_box")
        if not bb:
            continue
        try:
            x = int(bb.get("x", 0))
            y = int(bb.get("y", 0))
            w = int(bb.get("width", 0))
            h = int(bb.get("height", 0))
        except (TypeError, ValueError):
            continue
        if w <= 0 or h <= 0:
            continue

        elem_type = str(el.get("element_type", "") or el.get("tag", ""))
        color = _ELEMENT_COLORS.get(elem_type, _DEFAULT_COLOR)

        # Center point
        cx = x + w // 2
        cy = y + h // 2

        # Bounding box border
        draw.rectangle(
            [x, y, x + w, y + h],
            outline=color + (200,),
            width=_BORDER_WIDTH,
        )

        # Filled circle at center
        r = _CIRCLE_RADIUS
        draw.ellipse(
            [cx - r, cy - r, cx + r, cy + r],
            fill=color + (220,),
            outline=(255, 255, 255, 255),
            width=2,
        )

        # Number inside circle
        num_str = str(idx)
        # Center the text roughly
        if len(num_str) <= 2:
            tx = cx - 5 - (len(num_str) - 1) * 3
        else:
            tx = cx - 9
        ty = cy - 8
        draw.text((tx, ty), num_str, fill=(255, 255, 255, 255), font=font)

        # Label next to circle
        label = (el.get("label") or el.get("text") or el.get("tag") or "")[:25]
        if label:
            label_text = f"{idx}: {label}"
            draw.text(
                (cx + _LABEL_OFFSET_X, cy + _LABEL_OFFSET_Y),
                label_text,
                fill=(255, 255, 255, 255),
                font=small_font,
            )

    # Composite overlay onto original
    img = Image.alpha_composite(img, overlay)

    # Encode back to base64 PNG
    buf = io.BytesIO()
    img.save(buf, format="PNG", optimize=True)
    return base64.b64encode(buf.getvalue()).decode("ascii")
