# LoopSleuth GitHub App

A GitHub App that automatically analyzes Python code in pull requests for
performance issues using LoopSleuth and a HF Inference Endpoint.

## What it does

When a PR is opened or updated, the app:

1. Fetches the changed `.py` files from the PR
2. Identifies which Python functions were modified (using AST parsing + diff)
3. Runs LoopSleuth against those files via a HF Inference Endpoint
4. Filters results to only the changed functions
5. Posts (or updates) a comment on the PR with the findings

## Setup

### 1. Register a GitHub App

Go to **Settings > Developer settings > GitHub Apps > New GitHub App** and configure:

- **Webhook URL**: `https://your-server.com/webhook`
- **Webhook secret**: generate a random string
- **Permissions**:
  - Pull requests: Read & Write
  - Contents: Read
- **Subscribe to events**: Pull request

After creating the app, generate a private key (PEM file).

### 2. Environment variables

```bash
# GitHub App credentials
export GITHUB_APP_ID="123456"
export GITHUB_PRIVATE_KEY="$(cat your-app.pem)"
export GITHUB_WEBHOOK_SECRET="your-webhook-secret"

# HF Inference Endpoint
export HF_TOKEN="hf_..."
export LOOPSLEUTH_API_URL="https://your-endpoint.aws.endpoints.huggingface.cloud"

# Optional
export LOOPSLEUTH_MAX_FILES="15"  # max Python files to analyze per PR
```

### 3. Run locally

```bash
pip install -r requirements.txt
pip install loopsleuth

uvicorn app.main:app --port 8000
```

Use [smee.io](https://smee.io) or ngrok to expose your local server to GitHub
webhooks during development.

### 4. Deploy with Docker

```bash
docker build -t loopsleuth-app .

docker run -p 8000:8000 \
  -e GITHUB_APP_ID="..." \
  -e GITHUB_PRIVATE_KEY="$(cat your-app.pem)" \
  -e GITHUB_WEBHOOK_SECRET="..." \
  -e HF_TOKEN="hf_..." \
  -e LOOPSLEUTH_API_URL="https://your-endpoint.aws.endpoints.huggingface.cloud" \
  loopsleuth-app
```

## How it works

```
PR opened/synchronized
        |
GitHub webhook (pull_request event)
        |
FastAPI server (returns 200 immediately)
        |
Background task:
  1. GET /repos/:repo/pulls/:pr/files  -> changed .py files
  2. Parse diffs -> changed line numbers
  3. AST parse each file -> map lines to functions
  4. Write files to temp dir, run `loopsleuth --format json --api-url ...`
  5. Filter results to only changed functions
  6. POST/PATCH comment on the PR
```

## PR comment example

The app posts a collapsible comment like:

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

On subsequent pushes, the comment is updated in place (not duplicated).
