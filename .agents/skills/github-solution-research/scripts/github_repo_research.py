#!/usr/bin/env python3
"""Search public GitHub repositories for solution research.

This helper ranks GitHub repository candidates for a concrete feature search.
It uses only the Python standard library and prints public metadata only.
Final problem fit and local adaptation must be assessed by deep-reading the
candidate projects and local constraints.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import textwrap
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any


API_URL = "https://api.github.com/search/repositories"
DEFAULT_THRESHOLDS = (1000, 500, 100)

_TOKEN_CACHE: str | None = None


@dataclass(frozen=True)
class SearchAttempt:
    threshold: int
    query: str
    total_count: int
    returned_count: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Search public GitHub repositories, sorted by stars, for solution research."
    )
    parser.add_argument("--query", required=True, help="Feature-focused search terms.")
    parser.add_argument("--language", help="GitHub language qualifier, for example Python or TypeScript.")
    parser.add_argument("--topic", action="append", default=[], help="GitHub topic qualifier. Can be repeated.")
    parser.add_argument("--min-stars", type=int, default=1000, help="Initial minimum stars threshold.")
    parser.add_argument("--limit", type=int, default=10, help="Number of repositories to return after filtering.")
    parser.add_argument("--json", action="store_true", help="Print JSON output.")
    parser.add_argument("--markdown", action="store_true", help="Print Markdown output.")
    parser.add_argument(
        "--include-archived",
        action="store_true",
        help="Include archived repositories. Defaults to archived:false.",
    )
    return parser.parse_args()


def build_query(base_query: str, threshold: int, language: str | None, topics: list[str], include_archived: bool) -> str:
    parts = [base_query.strip(), f"stars:>={threshold}"]
    if not include_archived:
        parts.append("archived:false")
    if language:
        parts.append(f"language:{quote_qualifier(language)}")
    for topic in topics:
        parts.append(f"topic:{quote_qualifier(topic)}")
    return " ".join(part for part in parts if part)


def quote_qualifier(value: str) -> str:
    stripped = value.strip()
    if not stripped:
        return stripped
    if any(char.isspace() for char in stripped):
        return f'"{stripped}"'
    return stripped


def thresholds_from_min(min_stars: int) -> list[int]:
    thresholds: list[int] = []
    for threshold in (min_stars, *DEFAULT_THRESHOLDS):
        if threshold > 0 and threshold not in thresholds:
            thresholds.append(threshold)
    thresholds.sort(reverse=True)
    return thresholds


def github_request(query: str, per_page: int) -> dict[str, Any]:
    params = urllib.parse.urlencode(
        {
            "q": query,
            "sort": "stars",
            "order": "desc",
            "per_page": max(1, min(per_page, 100)),
        }
    )
    request = urllib.request.Request(
        f"{API_URL}?{params}",
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "codex-github-solution-research",
            **auth_header(),
        },
    )
    try:
        return open_json(request)
    except urllib.error.HTTPError as exc:
        body = exc.read().decode("utf-8", errors="replace")
        if exc.code in {403, 429} and wait_for_rate_limit_reset(exc):
            try:
                return open_json(request)
            except urllib.error.HTTPError as retry_exc:
                body = retry_exc.read().decode("utf-8", errors="replace")
                raise SystemExit(
                    f"GitHub API error {retry_exc.code}: {rate_limit_context(retry_exc)} {sanitize_error(body)}"
                ) from retry_exc
            except urllib.error.URLError as retry_exc:
                raise SystemExit(f"GitHub API request failed after retry: {retry_exc.reason}") from retry_exc
        raise SystemExit(f"GitHub API error {exc.code}: {rate_limit_context(exc)} {sanitize_error(body)}") from exc
    except urllib.error.URLError as exc:
        raise SystemExit(f"GitHub API request failed: {exc.reason}") from exc


def open_json(request: urllib.request.Request) -> dict[str, Any]:
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.loads(response.read().decode("utf-8"))


def auth_header() -> dict[str, str]:
    token = github_token()
    if not token:
        return {}
    return {"Authorization": f"Bearer {token}"}


def github_token() -> str:
    global _TOKEN_CACHE
    if _TOKEN_CACHE is not None:
        return _TOKEN_CACHE
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        _TOKEN_CACHE = token.strip()
        return _TOKEN_CACHE
    if not shutil.which("gh"):
        _TOKEN_CACHE = ""
        return _TOKEN_CACHE
    try:
        result = subprocess.run(
            ["gh", "auth", "token"],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        _TOKEN_CACHE = ""
        return _TOKEN_CACHE
    _TOKEN_CACHE = result.stdout.strip() if result.returncode == 0 else ""
    return _TOKEN_CACHE


def rate_limit_context(exc: urllib.error.HTTPError) -> str:
    headers = exc.headers
    pieces: list[str] = []
    resource = headers.get("x-ratelimit-resource")
    limit = headers.get("x-ratelimit-limit")
    remaining = headers.get("x-ratelimit-remaining")
    reset = headers.get("x-ratelimit-reset")
    retry_after = headers.get("retry-after")
    if resource:
        pieces.append(f"resource={resource}")
    if limit:
        pieces.append(f"limit={limit}")
    if remaining:
        pieces.append(f"remaining={remaining}")
    if reset and reset.isdigit():
        reset_at = datetime.fromtimestamp(int(reset), timezone.utc).isoformat()
        pieces.append(f"reset_utc={reset_at}")
    if retry_after:
        pieces.append(f"retry_after={retry_after}s")
    if not github_token():
        pieces.append("auth=anonymous; set GITHUB_TOKEN/GH_TOKEN or run gh auth login")
    if not pieces:
        return ""
    return f"({' '.join(pieces)})"


def wait_for_rate_limit_reset(exc: urllib.error.HTTPError) -> bool:
    retry_after = exc.headers.get("retry-after")
    sleep_seconds = 0
    if retry_after and retry_after.isdigit():
        sleep_seconds = int(retry_after)
    elif exc.headers.get("x-ratelimit-remaining") == "0":
        reset = exc.headers.get("x-ratelimit-reset")
        if reset and reset.isdigit():
            sleep_seconds = max(0, int(reset) - int(time.time()) + 1)
    if 0 < sleep_seconds <= 30:
        time.sleep(sleep_seconds)
        return True
    return False


def sanitize_error(text: str) -> str:
    token = github_token()
    if token:
        text = text.replace(token, "[redacted-token]")
    return textwrap.shorten(text.replace("\n", " "), width=500, placeholder="...")


def is_list_or_toy(repo: dict[str, Any]) -> bool:
    name = repo.get("name", "").lower()
    full_name = repo.get("full_name", "").lower()
    description = (repo.get("description") or "").lower()
    if name.startswith("awesome-") or "/awesome-" in full_name:
        return True
    markers = ("course", "homework", "assignment", "bootcamp", "tutorial only")
    return any(marker in description for marker in markers)


def repo_record(repo: dict[str, Any]) -> dict[str, Any]:
    license_info = repo.get("license") or {}
    pushed_at = repo.get("pushed_at")
    return {
        "repo": repo.get("full_name"),
        "url": repo.get("html_url"),
        "description": repo.get("description") or "",
        "stars": repo.get("stargazers_count", 0),
        "forks": repo.get("forks_count", 0),
        "open_issues": repo.get("open_issues_count", 0),
        "language": repo.get("language") or "",
        "license": license_info.get("spdx_id") or "NOASSERTION",
        "pushed_at": pushed_at,
        "archived": bool(repo.get("archived")),
        "topics": repo.get("topics", []),
        "score": metadata_score(repo),
        "risk_flags": risk_flags(repo),
    }


def metadata_score(repo: dict[str, Any]) -> int:
    stars = int(repo.get("stargazers_count") or 0)
    forks = int(repo.get("forks_count") or 0)
    pushed_at = repo.get("pushed_at")
    score = 0
    if stars >= 10_000:
        score += 30
    elif stars >= 5_000:
        score += 25
    elif stars >= 1_000:
        score += 20
    elif stars >= 500:
        score += 15
    elif stars >= 100:
        score += 10
    if forks >= 1_000:
        score += 15
    elif forks >= 250:
        score += 10
    elif forks >= 50:
        score += 5
    if pushed_at:
        score += recency_score(pushed_at)
    if repo.get("license"):
        score += 10
    if repo.get("archived"):
        score -= 20
    if is_list_or_toy(repo):
        score -= 15
    return max(score, 0)


def recency_score(pushed_at: str) -> int:
    try:
        pushed = datetime.fromisoformat(pushed_at.replace("Z", "+00:00"))
    except ValueError:
        return 0
    age_days = (datetime.now(timezone.utc) - pushed).days
    if age_days <= 90:
        return 20
    if age_days <= 365:
        return 15
    if age_days <= 730:
        return 8
    return 0


def risk_flags(repo: dict[str, Any]) -> list[str]:
    flags: list[str] = []
    if repo.get("archived"):
        flags.append("archived")
    if not repo.get("license"):
        flags.append("no-license")
    if is_list_or_toy(repo):
        flags.append("list-or-demo")
    pushed_at = repo.get("pushed_at")
    if pushed_at and recency_score(pushed_at) == 0:
        flags.append("stale")
    return flags


def search(args: argparse.Namespace) -> tuple[list[dict[str, Any]], list[SearchAttempt]]:
    attempts: list[SearchAttempt] = []
    seen: set[str] = set()
    records: list[dict[str, Any]] = []
    per_page = max(args.limit * 2, 20)

    for threshold in thresholds_from_min(args.min_stars):
        query = build_query(args.query, threshold, args.language, args.topic, args.include_archived)
        payload = github_request(query, per_page)
        items = payload.get("items", [])
        attempts.append(
            SearchAttempt(
                threshold=threshold,
                query=query,
                total_count=int(payload.get("total_count") or 0),
                returned_count=len(items),
            )
        )
        for item in items:
            full_name = item.get("full_name")
            if not full_name or full_name in seen:
                continue
            seen.add(full_name)
            record = repo_record(item)
            if "list-or-demo" in record["risk_flags"]:
                continue
            records.append(record)
            if len(records) >= args.limit:
                return records, attempts
        if len(records) >= 5:
            break
        time.sleep(0.2)
    return records[: args.limit], attempts


def print_markdown(records: list[dict[str, Any]], attempts: list[SearchAttempt]) -> None:
    print("# GitHub Repository Research")
    print()
    print("## Search Attempts")
    print()
    print("| min stars | returned | total | query |")
    print("| ---: | ---: | ---: | --- |")
    for attempt in attempts:
        print(f"| {attempt.threshold} | {attempt.returned_count} | {attempt.total_count} | `{attempt.query}` |")
    lowered = threshold_notes(attempts)
    if lowered:
        print()
        for note in lowered:
            print(f"- {note}")
    print()
    print("## Candidates")
    print()
    print("| repo | stars | forks | pushed | license | language | description | score | risk flags |")
    print("| --- | ---: | ---: | --- | --- | --- | --- | ---: | --- |")
    for record in records:
        risk = ", ".join(record["risk_flags"]) if record["risk_flags"] else ""
        description = markdown_cell(record["description"])
        print(
            f"| [{record['repo']}]({record['url']}) | {record['stars']} | {record['forks']} | "
            f"{record['pushed_at'] or ''} | {record['license']} | {record['language']} | "
            f"{description} | {record['score']} | {risk} |"
        )
    print()
    print(
        "Next step: deep-read the best 2-4 high-fit repositories, then report basic content, "
        "problem fit, reusable parts, adaptation cost, and verification before recommending a local implementation."
    )


def markdown_cell(value: str) -> str:
    return textwrap.shorten(str(value).replace("|", "\\|").replace("\n", " "), width=140, placeholder="...")


def threshold_notes(attempts: list[SearchAttempt]) -> list[str]:
    notes: list[str] = []
    for previous, current in zip(attempts, attempts[1:]):
        if previous.threshold > current.threshold:
            notes.append(
                f"Lowered star threshold from {previous.threshold} to {current.threshold} "
                f"because fewer than 5 credible candidates were found at the higher threshold."
            )
    return notes


def main() -> int:
    args = parse_args()
    if not args.json and not args.markdown:
        args.markdown = True
    records, attempts = search(args)
    output = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "search_attempts": [attempt.__dict__ for attempt in attempts],
        "repositories": records,
    }
    if args.json:
        print(json.dumps(output, ensure_ascii=False, indent=2))
    if args.markdown:
        if args.json:
            print()
        print_markdown(records, attempts)
    return 0


if __name__ == "__main__":
    sys.exit(main())
