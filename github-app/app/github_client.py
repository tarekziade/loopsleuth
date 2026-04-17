"""GitHub API helpers: fetch PR files, post/edit comments."""

import base64
import logging

from github import Auth, Github, GithubIntegration

from .comment_formatter import MARKER
from .config import settings

log = logging.getLogger(__name__)


def _get_installation_client(installation_id: int) -> Github:
    auth = Auth.AppAuth(
        int(settings.GITHUB_APP_ID),
        settings.GITHUB_PRIVATE_KEY,
    )
    gi = GithubIntegration(auth=auth)
    return gi.get_github_for_installation(installation_id)


def get_pr_python_files(
    installation_id: int,
    repo_full_name: str,
    pr_number: int,
    head_sha: str,
) -> list[dict]:
    """Return list of {filename, patch, content} for changed .py files."""
    gh = _get_installation_client(installation_id)
    repo = gh.get_repo(repo_full_name)
    pr = repo.get_pull(pr_number)

    results = []
    count = 0
    for f in pr.get_files():
        if not f.filename.endswith(".py"):
            continue
        if f.status == "removed":
            continue
        if count >= settings.MAX_FILES:
            log.warning("Reached max files limit (%d), skipping rest", settings.MAX_FILES)
            break

        patch = f.patch or ""

        # Fetch full file content at head SHA
        try:
            content_file = repo.get_contents(f.filename, ref=head_sha)
            if isinstance(content_file, list):
                continue  # directory, skip
            content = base64.b64decode(content_file.content).decode("utf-8")
        except Exception:
            log.warning("Could not fetch %s at %s", f.filename, head_sha)
            continue

        results.append({
            "filename": f.filename,
            "patch": patch,
            "content": content,
        })
        count += 1

    return results


def post_or_update_comment(
    installation_id: int,
    repo_full_name: str,
    pr_number: int,
    body: str,
) -> None:
    """Post a new comment or update an existing LoopSleuth comment."""
    gh = _get_installation_client(installation_id)
    repo = gh.get_repo(repo_full_name)
    issue = repo.get_issue(pr_number)

    # Look for existing comment with our marker
    for comment in issue.get_comments():
        if MARKER in (comment.body or ""):
            comment.edit(body)
            log.info("Updated existing comment %d", comment.id)
            return

    issue.create_comment(body)
    log.info("Created new comment on PR #%d", pr_number)
