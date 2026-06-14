"""Tests for structured logging configuration."""

import os
import sys
from unittest.mock import patch

import pytest

from ans_nerves.logging import configure_logging, get_logger, _ensure_configured


class TestConfigureLogging:
    def test_dev_mode_enabled(self):
        with patch.dict(os.environ, {"ANS_LOG_DEV": "1"}, clear=False):
            configure_logging()

    def test_dev_mode_with_yes(self):
        with patch.dict(os.environ, {"ANS_LOG_DEV": "yes"}, clear=False):
            configure_logging()

    def test_dev_mode_with_true(self):
        with patch.dict(os.environ, {"ANS_LOG_DEV": "true"}, clear=False):
            configure_logging()

    def test_json_mode_default(self):
        with patch.dict(os.environ, {}, clear=True):
            if "ANS_LOG_DEV" in os.environ:
                del os.environ["ANS_LOG_DEV"]
            if "ANS_LOG_LEVEL" in os.environ:
                del os.environ["ANS_LOG_LEVEL"]
            configure_logging()

    def test_custom_log_level(self):
        with patch.dict(os.environ, {"ANS_LOG_LEVEL": "DEBUG"}, clear=False):
            configure_logging()

    def test_invalid_log_level_falls_back_to_info(self):
        with patch.dict(os.environ, {"ANS_LOG_LEVEL": "INVALID"}, clear=False):
            configure_logging()

    def test_idempotent(self):
        configure_logging()
        configure_logging()


class TestGetLogger:
    def test_with_name(self):
        logger = get_logger("tests.test_logging")
        assert logger is not None

    def test_without_name(self):
        logger = get_logger()
        assert logger is not None


class TestEnsureConfigured:
    def test_first_call_configures(self):
        from ans_nerves import logging as mod
        old = mod._logging_configured
        mod._logging_configured = False
        try:
            _ensure_configured()
            assert mod._logging_configured
        finally:
            mod._logging_configured = old

    def test_second_call_is_noop(self):
        from ans_nerves import logging as mod
        old = mod._logging_configured
        mod._logging_configured = True
        try:
            _ensure_configured()
            assert mod._logging_configured
        finally:
            mod._logging_configured = old
