<!-- TOC -->

- [What does it do](#what-does-it-do)
- [Local test steps](#local-test-steps)
    - [Install dependencies](#install-dependencies)
    - [Set admin API key](#set-admin-api-key)
        - [1. Environment variable](#1-environment-variable)
        - [2. Application prompt](#2-application-prompt)
    - [Finally run it](#finally-run-it)
- [Deps management using Poetry](#deps-management-using-poetry)
    - [Prerequisites](#prerequisites)
    - [Installing dependencies](#installing-dependencies)
    - [Activating the Poetry environment](#activating-the-poetry-environment)
    - [Adding or updating dependencies](#adding-or-updating-dependencies)
- [Building](#building)

<!-- /TOC -->

# What does it do

Obtain cost data from provider's endpoint and display it in some GUI.

- UI
  - Runs as a small PySide6 system-tray app with a right‑click menu and status item.
- Data fetching & aggregation
  - Polls OpenAI /v1/organization/costs for month-to-date usage starting at UTC month start.
  - Supports full pagination across pages and normalizes buckets to UsageRecord.
  - Uses Decimal for monetary amounts and aggregates them.
- Resilience & observability
  - Retries requests with exponential backoff (3 attempts; retries on 429 and 5xx).
  - Raises annotated OpenAIProviderError on HTTP failures (status + body available).
  - Rotating file logging (~/ .cache/token_meter.log) for requests, retries, errors, and cache ops.
- Local caching
  - Caches most recent total in JSON (~/.cache/ai_usage.json) for 5 minutes to avoid redundant requests.
- Key handling
  - Reads API key from OPENAI_API_KEY env var or from a local keystore (~/.config/token_meter/credentials.json).
  - If missing, prompts the user in a modal dialog and can save the key locally (file saved with an attempted 0o600 chmod and logged).

# Local test steps

This project uses **Poetry** for dependency management. Do **not** manually activate a virtual environment.

## Install dependencies

From the project root, install all dependencies into Poetry’s managed virtual environment:

```bash
poetry install
```

## Set admin API key

There are two supported ways to provide the OpenAI admin API key.

### 1. Environment variable

> Windows command line

```bash
set OPENAI_API_KEY=sk-admin-xV...
```

(Optional) Verify:

> Windows command line

```bash
echo %OPENAI_API_KEY%
```

### 2. Application prompt

If the environment variable is not set, the application will display a dialog prompting for an admin API key.

- Enter the key in the input field
- Optionally enable **“save to local config…”** to persist the key under:

  ```
  <user_home>\.token-meter\
  ```

On subsequent runs, a previously saved key will be used automatically.

⚠ **Security note:** The admin API key is stored as plain text in a JSON file. Users are responsible for securing their local environment.

## Finally run it

Run the app inside Poetry’s environment:

```bash
poetry run python -m token_meter.main
```

Or, for interactive development:

```bash
poetry shell
python -m token_meter.main
```

# Deps management using Poetry

This project uses **[Poetry](https://python-poetry.org/)** for dependency management and version pinning.

## Prerequisites

- Python **3.11 – 3.14**
- Poetry installed globally

Install Poetry globally once per system:

```bash
pip install poetry
```

Verify:

```bash
poetry --version
```

## Installing dependencies

From the project root:

```bash
poetry install
```

This will:

- Create or reuse a virtual environment
- Install all runtime and development dependencies
- Respect exact versions pinned in `poetry.lock`

## Activating the Poetry environment

Option 1: spawn a Poetry shell:

```bash
poetry shell
```

Option 2: run commands inside the environment without activating it:

```bash
poetry run python -m token_meter.main
```

> If prefer a local `.venv` directory, configure Poetry once:
>
> ```bash
> poetry config virtualenvs.in-project true
> ```
>
> Then re-run `poetry install`.

## Adding or updating dependencies

To add a new dependency:

```bash
poetry add httpx qasync
```

Poetry will:

- Resolve compatible versions
- Update `pyproject.toml`
- Update `poetry.lock`

To update all dependencies within allowed version ranges:

```bash
poetry update
```

# Building

Use PyInstaller, and let it work with poetry env.

```bash
poetry add --group dev pyinstaller
```

Run a "draft" build.

> Windows command line

```bash
poetry run pyinstaller --name token-meter --onefile --windowed src\token_meter\main.py
```

It will generate some files. Of those files, modify `token-meter.spec` correctly.

Or use the existing `token-meter.oned.spec` (onedir build) or `token-meter.onef.spec` (onefile build) in this repo. We rocommend `token-meter.onef.spec`, because the onefile build is about 240 mb, smaller than the onedir build which is about 622 mb.

Then run the real building.

```bash
poetry run pyinstaller token-meter.onef.spec
```
