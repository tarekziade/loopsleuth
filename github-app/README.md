# LoopSleuth GitHub Integration

Automatically analyze Python code in pull requests for performance issues
using LoopSleuth and a HF Inference Endpoint.

Two ways to use it:
- **GitHub Action** (recommended) -- add a workflow file to your repo, no server needed
- **GitHub App** (self-hosted) -- deploy a webhook server for richer integration

---

## Option A: GitHub Action (recommended)

### 1. Set up secrets

In your repo, go to **Settings > Secrets and variables > Actions** and add:

| Secret | Value |
|---|---|
| `HF_TOKEN` | Your Hugging Face API token |
| `LOOPSLEUTH_API_URL` | Your HF Inference Endpoint URL |

### 2. Add the workflow

Copy this into `.github/workflows/loopsleuth.yml` in your repo:

```yaml
name: LoopSleuth

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  pull-requests: write
  contents: read

jobs:
  analyze:
    name: Performance analysis
    runs-on: ubuntu-latest
    steps:
      - uses: tarekziade/loopsleuth/github-app@main
        with:
          hf_token: ${{ secrets.HF_TOKEN }}
          api_url: ${{ secrets.LOOPSLEUTH_API_URL }}
```

That's it. Every PR that touches Python files will get a comment with any
performance issues found.

### Action inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `hf_token` | yes | | HF API token |
| `api_url` | yes | | HF Inference Endpoint URL |
| `max_files` | no | `15` | Max Python files to analyze per PR |
| `checks` | no | all | Comma-separated list of checks to run |
| `github_token` | no | `GITHUB_TOKEN` | Token for posting PR comments |

### What happens

1. The action fetches the PR diff
2. Parses it to find which Python functions were modified
3. Runs `loopsleuth --format json --api-url ...` on each changed file
4. Filters results to only the modified functions
5. Posts a comment on the PR (or updates the existing one)

---

## Option B: GitHub App (self-hosted)

For organizations that want a centralized deployment without per-repo
workflow files.

### 1. Register a GitHub App

Go to **Settings > Developer settings > GitHub Apps > New GitHub App**:

- **Webhook URL**: `https://your-server.com/webhook`
- **Webhook secret**: generate a random string
- **Permissions**:
  - Pull requests: Read & Write
  - Contents: Read
- **Subscribe to events**: Pull request

Generate a private key (PEM file) after creating the app.

### 2. Deploy the server

```bash
# Environment variables
export GITHUB_APP_ID="123456"
export GITHUB_PRIVATE_KEY="$(cat your-app.pem)"
export GITHUB_WEBHOOK_SECRET="your-webhook-secret"
export HF_TOKEN="hf_..."
export LOOPSLEUTH_API_URL="https://your-endpoint.aws.endpoints.huggingface.cloud"

# Run with Docker
docker build -t loopsleuth-app .
docker run -p 8000:8000 \
  -e GITHUB_APP_ID -e GITHUB_PRIVATE_KEY -e GITHUB_WEBHOOK_SECRET \
  -e HF_TOKEN -e LOOPSLEUTH_API_URL \
  loopsleuth-app
```

Or run directly:

```bash
pip install -r requirements.txt
pip install loopsleuth
uvicorn app.main:app --port 8000
```

### 3. Install the app

Install it on any repo from the app's GitHub page. PRs are analyzed
automatically -- no workflow files needed.

---

## How it works

```
PR opened / updated
        |
        v
Fetch changed .py files + diffs
        |
        v
Parse diffs -> changed line numbers
        |
        v
AST-parse each file -> map lines to function names
        |
        v
Run `loopsleuth --format json --api-url ...` per file
        |
        v
Filter results to only changed functions
        |
        v
Post / update PR comment
```

## PR comment example

> ## LoopSleuth Performance Analysis
>
> Found **2 issue(s)** in **2 function(s)** across **1 file(s)**.
>
> <details>
> <summary><b>utils.py</b> — 2 issue(s)</summary>
>
> ### `find_duplicates` (line 42)
>
> **Quadratic Complexity** (confidence: 99%)
>
> <details>
> <summary>Suggested fix</summary>
>
> ```diff
> - for i in items: for j in items:
> + seen = set()
> ```
> </details>
> </details>

On subsequent pushes to the same PR, the comment is updated in place (not
duplicated).

## File structure

```
github-app/
  action.yml            # GitHub Action definition (composite action)
  analyze_pr.py         # Self-contained script for the Action (no extra deps)
  example-workflow.yml  # Copy this into your repo
  app/                  # GitHub App server (Option B)
    main.py             # FastAPI webhook handler
    config.py           # Environment config
    analyzer.py         # Runs loopsleuth CLI
    diff_parser.py      # Unified diff parser
    function_extractor.py  # AST-based function mapping
    github_client.py    # GitHub API (PyGithub)
    comment_formatter.py   # Markdown formatting
  Dockerfile            # Container for the server
  requirements.txt      # Server dependencies
```
