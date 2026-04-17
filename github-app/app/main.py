"""FastAPI webhook server for the LoopSleuth GitHub App."""

import hashlib
import hmac
import logging

from fastapi import BackgroundTasks, FastAPI, Header, HTTPException, Request

from .analyzer import analyze_files
from .comment_formatter import format_clean_comment, format_comment
from .config import settings
from .diff_parser import extract_changed_lines
from .function_extractor import find_changed_functions
from .github_client import get_pr_python_files, post_or_update_comment

logging.basicConfig(level=logging.INFO)
log = logging.getLogger(__name__)

app = FastAPI(title="LoopSleuth GitHub App")


def verify_signature(payload: bytes, signature: str) -> None:
    expected = "sha256=" + hmac.new(
        settings.GITHUB_WEBHOOK_SECRET.encode(),
        payload,
        hashlib.sha256,
    ).hexdigest()
    if not hmac.compare_digest(expected, signature):
        raise HTTPException(status_code=401, detail="Invalid signature")


@app.post("/webhook")
async def webhook(
    request: Request,
    background_tasks: BackgroundTasks,
    x_github_event: str = Header(None),
    x_hub_signature_256: str = Header(None),
):
    payload = await request.body()

    if x_hub_signature_256:
        verify_signature(payload, x_hub_signature_256)

    if x_github_event != "pull_request":
        return {"status": "ignored", "reason": "not a pull_request event"}

    data = await request.json()
    action = data.get("action")

    if action not in ("opened", "synchronize", "reopened"):
        return {"status": "ignored", "reason": f"action={action}"}

    pr = data["pull_request"]
    repo_full_name = data["repository"]["full_name"]
    pr_number = pr["number"]
    head_sha = pr["head"]["sha"]
    installation_id = data["installation"]["id"]

    log.info(
        "Received PR #%d (%s) on %s — scheduling analysis",
        pr_number, action, repo_full_name,
    )

    background_tasks.add_task(
        process_pr,
        installation_id=installation_id,
        repo_full_name=repo_full_name,
        pr_number=pr_number,
        head_sha=head_sha,
    )

    return {"status": "processing"}


def process_pr(
    installation_id: int,
    repo_full_name: str,
    pr_number: int,
    head_sha: str,
) -> None:
    """Background task: fetch changed files, run loopsleuth, post comment."""
    try:
        log.info("Analyzing PR #%d on %s", pr_number, repo_full_name)

        # 1. Fetch changed Python files
        pr_files = get_pr_python_files(
            installation_id, repo_full_name, pr_number, head_sha,
        )

        if not pr_files:
            log.info("No Python files changed in PR #%d", pr_number)
            return

        # 2. Extract changed functions per file
        files_content: dict[str, str] = {}
        changed_functions: dict[str, set[str]] = {}

        for pf in pr_files:
            filename = pf["filename"]
            content = pf["content"]
            patch = pf["patch"]

            files_content[filename] = content

            if patch:
                changed_lines = extract_changed_lines(patch)
                funcs = find_changed_functions(content, changed_lines)
                changed_functions[filename] = funcs
            else:
                # New file — analyze all functions
                changed_functions[filename] = set()

        log.info(
            "Found %d Python file(s) with %d changed function(s)",
            len(files_content),
            sum(len(v) for v in changed_functions.values()),
        )

        # 3. Run loopsleuth
        issues = analyze_files(files_content, changed_functions)

        # 4. Post comment only if issues found
        if issues:
            body = format_comment(issues)
            post_or_update_comment(
                installation_id, repo_full_name, pr_number, body,
            )

        log.info("Done with PR #%d — %d file(s) with issues", pr_number, len(issues))

    except Exception:
        log.exception("Error processing PR #%d on %s", pr_number, repo_full_name)
