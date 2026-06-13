#!/usr/bin/env python3
"""Search GitHub surfaces for problem-solving evidence.

This helper gathers public GitHub search candidates for a concrete engineering
problem. It searches issues/PRs, repositories, or code and prints metadata only.
It uses only the Python standard library.
"""

from __future__ import annotations

import argparse
import json
import os
import re
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


SEARCH_ENDPOINTS = {
    "issues": "https://api.github.com/search/issues",
    "prs": "https://api.github.com/search/issues",
    "repositories": "https://api.github.com/search/repositories",
    "code": "https://api.github.com/search/code",
}


@dataclass(frozen=True)
class SearchAttempt:
    surface: str
    query: str
    total_count: int
    returned_count: int
    error: str = ""


class GitHubSearchError(RuntimeError):
    """Raised when one GitHub search surface fails."""


_TOKEN_CACHE: str | None = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Search GitHub issues, PRs, repositories, and code for concrete problem solutions."
    )
    parser.add_argument("--query", required=True, help="Problem-focused search terms.")
    parser.add_argument(
        "--surface",
        choices=("issues", "prs", "repositories", "code", "all"),
        default="all",
        help="GitHub surface to search. Defaults to all supported surfaces.",
    )
    parser.add_argument("--repo", help="Restrict search to owner/repo.")
    parser.add_argument("--language", help="GitHub language qualifier for repository or code search.")
    parser.add_argument("--limit", type=int, default=5, help="Results per searched surface.")
    parser.add_argument("--json", action="store_true", help="Print JSON output.")
    parser.add_argument("--markdown", action="store_true", help="Print Markdown output.")
    parser.add_argument("--open-issues-only", action="store_true", help="Restrict issue search to open issues.")
    parser.add_argument("--include-open-prs", action="store_true", help="Include open PRs in PR search.")
    parser.add_argument(
        "--include-forks",
        action="store_true",
        help="Include forked repositories in code search. Defaults to fork:false.",
    )
    return parser.parse_args()


def selected_surfaces(surface: str) -> list[str]:
    if surface == "all":
        return ["issues", "prs", "code", "repositories"]
    return [surface]


def build_query(args: argparse.Namespace, surface: str) -> str:
    parts = [args.query.strip()]
    if args.repo:
        parts.append(f"repo:{args.repo.strip()}")
    if surface == "issues":
        parts.append("is:issue")
        if args.open_issues_only:
            parts.append("is:open")
    elif surface == "prs":
        parts.append("is:pr")
        if not args.include_open_prs:
            parts.append("is:merged")
    elif surface == "code":
        if args.language:
            parts.append(f"language:{quote_qualifier(args.language)}")
        if not args.include_forks:
            parts.append("fork:false")
    elif surface == "repositories":
        parts.append("archived:false")
        if args.language:
            parts.append(f"language:{quote_qualifier(args.language)}")
    return " ".join(part for part in parts if part)


def quote_qualifier(value: str) -> str:
    stripped = value.strip()
    if not stripped:
        return stripped
    if any(char.isspace() for char in stripped):
        return f'"{stripped}"'
    return stripped


def github_request(surface: str, query: str, limit: int) -> dict[str, Any]:
    endpoint = SEARCH_ENDPOINTS[surface]
    sort, order = sort_for_surface(surface)
    params = {
        "q": query,
        "per_page": max(1, min(limit, 100)),
    }
    if sort:
        params["sort"] = sort
        params["order"] = order
    request = urllib.request.Request(
        f"{endpoint}?{urllib.parse.urlencode(params)}",
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "codex-github-problem-search",
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
                raise GitHubSearchError(
                    f"GitHub API error {retry_exc.code}: {rate_limit_context(retry_exc)} {sanitize_error(body)}"
                ) from retry_exc
            except urllib.error.URLError as retry_exc:
                raise GitHubSearchError(f"GitHub API request failed after retry: {retry_exc.reason}") from retry_exc
        raise GitHubSearchError(f"GitHub API error {exc.code}: {rate_limit_context(exc)} {sanitize_error(body)}") from exc
    except urllib.error.URLError as exc:
        raise GitHubSearchError(f"GitHub API request failed: {exc.reason}") from exc


def open_json(request: urllib.request.Request) -> dict[str, Any]:
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.loads(response.read().decode("utf-8"))


def sort_for_surface(surface: str) -> tuple[str | None, str | None]:
    if surface in {"issues", "prs"}:
        return "updated", "desc"
    if surface == "repositories":
        return "stars", "desc"
    return None, None


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


def risk_flags(surface: str, item: dict[str, Any]) -> list[str]:
    flags: list[str] = []
    if surface in {"issues", "prs"}:
        if item.get("state") == "open":
            flags.append("unresolved")
        if surface == "prs" and item.get("state") != "closed":
            flags.append("not-merged")
        title_body = f"{item.get('title') or ''} {item.get('body') or ''}".lower()
        if any(marker in title_body for marker in ("workaround", "hack", "temporary")):
            flags.append("workaround")
    if surface == "repositories":
        if item.get("archived"):
            flags.append("archived")
        if not item.get("license"):
            flags.append("no-license")
        if (item.get("name") or "").lower().startswith("awesome-"):
            flags.append("list")
    if surface == "code":
        repo = item.get("repository") or {}
        if repo.get("archived"):
            flags.append("archived-repo")
        if not repo.get("license"):
            flags.append("repo-no-license")
    return flags


def record(surface: str, item: dict[str, Any]) -> dict[str, Any]:
    if surface in {"issues", "prs"}:
        repo_url = item.get("repository_url") or ""
        repo = repo_url.rsplit("/", 2)[-2:]
        return {
            "surface": surface,
            "title": item.get("title") or "",
            "url": item.get("html_url") or "",
            "repository": "/".join(repo) if len(repo) == 2 else "",
            "state": item.get("state") or "",
            "updated_at": item.get("updated_at") or "",
            "summary": compact_text(item.get("body") or ""),
            "risk_flags": risk_flags(surface, item),
        }
    if surface == "repositories":
        license_info = item.get("license") or {}
        return {
            "surface": surface,
            "title": item.get("full_name") or "",
            "url": item.get("html_url") or "",
            "repository": item.get("full_name") or "",
            "state": "archived" if item.get("archived") else "active",
            "updated_at": item.get("pushed_at") or "",
            "summary": compact_text(item.get("description") or ""),
            "stars": item.get("stargazers_count", 0),
            "forks": item.get("forks_count", 0),
            "language": item.get("language") or "",
            "license": license_info.get("spdx_id") or "NOASSERTION",
            "risk_flags": risk_flags(surface, item),
        }
    repo = item.get("repository") or {}
    return {
        "surface": surface,
        "title": item.get("name") or item.get("path") or "",
        "url": item.get("html_url") or "",
        "repository": repo.get("full_name") or "",
        "state": "code",
        "updated_at": "",
        "summary": item.get("path") or "",
        "risk_flags": risk_flags(surface, item),
    }


def compact_text(text: str, width: int = 180) -> str:
    cleaned = re.sub(r"\s+", " ", text).strip()
    return textwrap.shorten(cleaned, width=width, placeholder="...")


def search(args: argparse.Namespace) -> tuple[list[dict[str, Any]], list[SearchAttempt]]:
    records: list[dict[str, Any]] = []
    attempts: list[SearchAttempt] = []
    for surface in selected_surfaces(args.surface):
        query = build_query(args, surface)
        try:
            payload = github_request(surface, query, args.limit)
        except GitHubSearchError as exc:
            attempts.append(
                SearchAttempt(
                    surface=surface,
                    query=query,
                    total_count=0,
                    returned_count=0,
                    error=str(exc),
                )
            )
            continue
        items = payload.get("items", [])
        attempts.append(
            SearchAttempt(
                surface=surface,
                query=query,
                total_count=int(payload.get("total_count") or 0),
                returned_count=len(items),
            )
        )
        records.extend(record(surface, item) for item in items)
    return records, attempts


def print_markdown(records: list[dict[str, Any]], attempts: list[SearchAttempt]) -> None:
    print("# GitHub Problem Search")
    print()
    print("## Search Attempts")
    print()
    print("| surface | returned | total | query | error |")
    print("| --- | ---: | ---: | --- | --- |")
    for attempt in attempts:
        print(
            f"| {attempt.surface} | {attempt.returned_count} | {attempt.total_count} | "
            f"`{attempt.query}` | {escape_table(attempt.error)} |"
        )
    print()
    problem_records = [item for item in records if item["surface"] != "repositories"]
    repo_records = [item for item in records if item["surface"] == "repositories"]
    if problem_records:
        print("## Evidence Candidates")
        print()
        print("| surface | title | repo | state | updated | risk flags | summary |")
        print("| --- | --- | --- | --- | --- | --- | --- |")
    for item in problem_records:
        title = markdown_link(item["title"], item["url"])
        risk = ", ".join(item["risk_flags"]) if item["risk_flags"] else ""
        print(
            f"| {item['surface']} | {title} | {item['repository']} | {item['state']} | "
            f"{item['updated_at']} | {risk} | {escape_table(item['summary'])} |"
        )
    if repo_records:
        if problem_records:
            print()
        print("## Repository Candidates")
        print()
        print("| repo | stars | forks | updated | license | language | state | risk flags | summary |")
        print("| --- | ---: | ---: | --- | --- | --- | --- | --- | --- |")
    for item in repo_records:
        title = markdown_link(item["title"], item["url"])
        risk = ", ".join(item["risk_flags"]) if item["risk_flags"] else ""
        print(
            f"| {title} | {item.get('stars', 0)} | {item.get('forks', 0)} | "
            f"{item['updated_at']} | {item.get('license', '')} | {item.get('language', '')} | "
            f"{item['state']} | {risk} | {escape_table(item['summary'])} |"
        )
    print()
    if repo_records:
        print(
            "Next step: deep-read the strongest matching repositories and evidence, then report basic content, "
            "problem fit, reusable parts, adaptation cost, and verification before recommending a local fix."
        )
    else:
        print("Next step: deep-read the strongest matching issues, PRs, code, examples, or releases before recommending a local fix.")


def markdown_link(label: str, url: str) -> str:
    safe_label = escape_table(label or url or "(untitled)")
    if not url:
        return safe_label
    return f"[{safe_label}]({url})"


def escape_table(text: str) -> str:
    return (text or "").replace("|", "\\|").replace("\n", " ")


def main() -> int:
    args = parse_args()
    if not args.json and not args.markdown:
        args.markdown = True
    records, attempts = search(args)
    output = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "search_attempts": [attempt.__dict__ for attempt in attempts],
        "results": records,
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
