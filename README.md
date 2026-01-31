# Steps

Activate venv.

```bash
.venv\Scripts\activate
```

Set environment variable for openAI API admin key.

- Windows command line
    ```bash
    set OPENAI_API_KEY=sk-admin-xV...
    ```

Confirm if you want.

- Windows command line
    ```bash
    echo %OPENAI_API_KEY%
    ```

Finally run the app.

```bash
python -m token_meter.main
```