#!/usr/bin/env python3
"""Render the runtime DcaPal configuration from the checked-in template."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Any


BACKEND_DIR = Path(__file__).resolve().parents[1]
DEFAULT_TEMPLATE = BACKEND_DIR / "config/dcapal/dcapal.yml"
DEFAULT_OUTPUT = BACKEND_DIR / "dcapal.yml"


def json_value(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False)


def required_environment(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"{name} must be set before rendering dcapal.yml")
    return value


def integer_environment(name: str, default: int) -> int:
    value = os.environ.get(name)
    if value is None:
        return default
    try:
        return int(value)
    except ValueError as error:
        raise SystemExit(f"{name} must be an integer") from error


def boolean_environment(name: str, default: bool) -> bool:
    value = os.environ.get(name)
    if value is None:
        return default
    normalized = value.strip().lower()
    if normalized in {"1", "true", "yes", "on"}:
        return True
    if normalized in {"0", "false", "no", "off"}:
        return False
    raise SystemExit(f"{name} must be a boolean")


def jwks_environment() -> dict[str, list[dict[str, Any]]]:
    raw_value = os.environ.get("DCAPAL_JWT_JWKS", '{"keys": []}')
    try:
        value = json.loads(raw_value)
    except json.JSONDecodeError as error:
        raise SystemExit("DCAPAL_JWT_JWKS must contain valid JSON") from error

    if isinstance(value, list):
        value = {"keys": value}
    if not isinstance(value, dict) or not isinstance(value.get("keys"), list):
        raise SystemExit('DCAPAL_JWT_JWKS must be a JSON object with a "keys" array')
    if not all(isinstance(key, dict) for key in value["keys"]):
        raise SystemExit('DCAPAL_JWT_JWKS "keys" entries must be JSON objects')
    return value


def render(template: str) -> str:
    ip2location_path = os.environ.get("DCAPAL_IP2LOCATION_PATH")
    services = None
    if ip2location_path:
        services = {"ip": {"dbPath": ip2location_path}}

    substitutions = {
        "@@DCAPAL_PRICE_PROVIDER@@": json_value(
            os.environ.get("DCAPAL_PRICE_PROVIDER", "kraken")
        ),
        "@@DCAPAL_CW_API_KEY@@": json_value(
            os.environ.get("DCAPAL_CW_API_KEY", "CW_API_KEY")
        ),
        "@@DCAPAL_IP_API_KEY@@": json_value(
            os.environ.get("DCAPAL_IP_API_KEY", "IP_API_KEY")
        ),
        "@@DCAPAL_CMC_API_KEY@@": json_value(
            os.environ.get("DCAPAL_CMC_API_KEY") or None
        ),
        "@@DCAPAL_JWT_SECRET@@": json_value(required_environment("DCAPAL_JWT_SECRET")),
        "@@DCAPAL_JWT_JWKS@@": json_value(jwks_environment()),
        "@@DCAPAL_SERVICES@@": json_value(services),
        "@@DCAPAL_LOG_LEVEL@@": json_value(
            os.environ.get("DCAPAL_LOG_LEVEL", "dcapal_backend=info,tower_http=debug")
        ),
        "@@DCAPAL_LOG_FILE@@": json_value(
            os.environ.get("DCAPAL_LOG_FILE", "data/dcapal/dcapal.log")
        ),
        "@@DCAPAL_LOG_ENABLE_STDOUT@@": json_value(
            boolean_environment("DCAPAL_LOG_ENABLE_STDOUT", True)
        ),
        "@@DCAPAL_WEB_HOSTNAME@@": json_value(
            os.environ.get("DCAPAL_WEB_HOSTNAME", "127.0.0.1")
        ),
        "@@DCAPAL_WEB_PORT@@": str(integer_environment("DCAPAL_WEB_PORT", 8080)),
        "@@DCAPAL_METRICS_HOSTNAME@@": json_value(
            os.environ.get("DCAPAL_METRICS_HOSTNAME", "127.0.0.1")
        ),
        "@@DCAPAL_METRICS_PORT@@": str(
            integer_environment("DCAPAL_METRICS_PORT", 9000)
        ),
        "@@DCAPAL_REDIS_HOSTNAME@@": json_value(
            os.environ.get("DCAPAL_REDIS_HOSTNAME", "127.0.0.1")
        ),
        "@@DCAPAL_REDIS_PORT@@": str(integer_environment("DCAPAL_REDIS_PORT", 6379)),
        "@@DCAPAL_REDIS_USER@@": json_value(
            os.environ.get("DCAPAL_REDIS_USER", "dcapal")
        ),
        "@@DCAPAL_REDIS_PASSWORD@@": json_value(
            os.environ.get("DCAPAL_REDIS_PASSWORD", "dcapal")
        ),
        "@@DCAPAL_POSTGRES_HOSTNAME@@": json_value(
            os.environ.get("DCAPAL_POSTGRES_HOSTNAME", "127.0.0.1")
        ),
        "@@DCAPAL_POSTGRES_PORT@@": str(
            integer_environment("DCAPAL_POSTGRES_PORT", 5433)
        ),
        "@@DCAPAL_POSTGRES_USER@@": json_value(
            os.environ.get("DCAPAL_POSTGRES_USER", "postgres")
        ),
        "@@DCAPAL_POSTGRES_PASSWORD@@": json_value(
            os.environ.get("DCAPAL_POSTGRES_PASSWORD", "postgres")
        ),
        "@@DCAPAL_POSTGRES_DATABASE@@": json_value(
            os.environ.get("DCAPAL_POSTGRES_DATABASE", "postgres")
        ),
    }

    for placeholder, value in substitutions.items():
        template = template.replace(placeholder, value)

    if "@@" in template:
        raise SystemExit("template contains an unknown or unrendered placeholder")
    return template


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", type=Path, default=DEFAULT_TEMPLATE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    rendered = render(args.template.read_text(encoding="utf-8"))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
