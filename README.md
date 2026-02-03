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

Activate venv.

```bash
.venv\Scripts\activate
```

Two main ways to set admin api key.

1. Set environment variable for openAI API admin key.

    > Windows command line
    ```bash
    set OPENAI_API_KEY=sk-admin-xV...
    ```

    Confirm if you want.

    > Windows command line
    ```bash
    echo %OPENAI_API_KEY%
    ```

2. Do not set env var, the app will pop up a small dialogue window to prompt inputting an admin key. Input it in the input field. Optionally toggle on the "save to local config..." option, and it will be saved under `<user_home>\.token-meter\`.

    Next time if the app finds a previously saved api key it will directly use that.

    ⚠ The admin api key is saved as plain text in a json. The user should watch out for its security on their own.

Finally run the app.

```bash
python -m token_meter.main
```