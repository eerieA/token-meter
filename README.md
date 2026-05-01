# Token Meter

A lightweight API usage & cost widget built with Rust + Tauri and a small HTML/CSS frontend. Supports OpenAI and Anthropic API cost tracking.

This repo is a migration from previous Pyside6 stack.

<!-- TOC -->

- [Token Meter](#token-meter)
    - [Features](#features)
    - [Project structure](#project-structure)
    - [Storage / Configuration](#storage--configuration)
    - [Running / Building](#running--building)
        - [Quick start dev](#quick-start-dev)
        - [Build for release](#build-for-release)
        - [Clean cargo cache](#clean-cargo-cache)
    - [API access requirements](#api-access-requirements)
    - [Cache & Fetch behavior](#cache--fetch-behavior)
    - [Development notes](#development-notes)
        - [Debugging](#debugging)
    - [Limitations & Future work](#limitations--future-work)

<!-- /TOC -->

## Features

- Borderless, draggable widget window implemented with HTML/CSS
- Minimal web-based UI (web/index.html) talking to native Rust commands via Tauri
- **OpenAI and Anthropic provider support** — toggle between them with the OAI / ANT buttons
- Fetches month-to-date organization usage costs for the active provider
- Baseline mode: optionally record a baseline credit amount and compute remaining credit since a start date — configured independently per provider
- Local JSON cache to avoid too many API calls (cache considered stale after 3 minutes)
- Per-provider inline key entry: if no API key is saved for the active provider, the widget shows `$--:--` and an inline "Add key" panel
- Retry & pagination logic for robust API requests
- Uses rust_decimal for accurate monetary aggregation
- API keys stored in the user's home directory (see Storage below)

Windows version preview and Linux version preview (Ubuntu with GNOME X11).

<img alt="token meter windows ver preview" src="https://live.staticflickr.com/65535/55102591518_ba15bab69b.jpg" width="320">
<img alt="token meter linux ver preview" src="https://live.staticflickr.com/65535/55106010788_80ddac54f0.jpg" width="320">

> You can see there is ghosting problem on GNOME X11. But it may require significant effort to fix so probably not gonna get fixed.

## Project structure

```
.
├── src/                         # Core Rust library code
│   ├── providers/
│   │   ├── openai.rs            # OpenAI costs API provider
│   │   └── anthropic.rs         # Anthropic cost report API provider
│   ├── aggregator.rs            # Coordinates provider calls, computes totals
│   ├── storage.rs               # Credentials & cache persistence
│   └── domain.rs                # Shared types
├── src-tauri/                   # Tauri app scaffolding and native entrypoint
│   └── src/main.rs              # Tauri commands that the web UI invokes
├── web/                         # Small web UI (HTML/CSS/JS)
│   ├── index.html               # Main widget
│   ├── baseline.html            # Baseline entry overlay window
│   └── overlay.html             # Context menu overlay window
└── package.json                 # npm scripts to run/build the Tauri app
```

Tauri commands exposed to the UI (src-tauri/src/main.rs):

**OpenAI**
- `get_api_key()` — returns saved OpenAI key (or null)
- `save_api_key_command(api_key)` — saves the OpenAI key to disk
- `validate_api_key(api_key)` — validates by performing a test fetch
- `fetch_month_to_date(api_key)` — fetches MTD or baseline-remaining for OpenAI
- `save_baseline_command(amount, start_iso)` — persists an OpenAI baseline
- `clear_baseline_command()` — removes the OpenAI baseline

**Anthropic**
- `get_anthropic_api_key()` — returns saved Anthropic admin key (or null)
- `save_anthropic_api_key_command(api_key)` — saves the Anthropic key to disk
- `validate_anthropic_api_key(api_key)` — validates by performing a test fetch
- `fetch_anthropic_month_to_date(api_key)` — fetches MTD or baseline-remaining for Anthropic
- `save_anthropic_baseline_command(amount, start_iso)` — persists an Anthropic baseline
- `clear_anthropic_baseline_command()` — removes the Anthropic baseline

**Shared**
- `get_cached_data()` — returns `{ openai, anthropic }` with fresh cached data for each provider (baseline config always returned; cost figures only when cache is fresh)
- `set_baseline_provider(provider)` / `get_baseline_provider()` — coordinates which provider the baseline modal is operating on
- `show_context_menu(x, y)` / `hide_context_menu()` — native overlay for context menu
- `show_baseline_modal(x, y)` / `hide_baseline_modal()` — native overlay for baseline entry
- `quit_app()` — exit the application

## Storage / Configuration

An admin API key is required for each provider you want to use. Keys are stored under a `.token-meter` directory in the user's home folder:

- Unix/macOS: `~/.token-meter/`
- Windows: `C:\Users\<you>\.token-meter\`

Files created:

```
~/.token-meter/
├── credentials.json    # { "openai_api_key": "...", "anthropic_api_key": "..." }
└── api_usage.json      # cached totals, timestamps, and baseline metadata per provider
```

`api_usage.json` fields of interest:

| Field | Description |
|-------|-------------|
| `openai_total` | OpenAI month-to-date total (string) |
| `fetched_at` | When OpenAI data was last fetched |
| `baseline` | OpenAI baseline `{ amount, start_iso }` |
| `baseline_used` | OpenAI usage since baseline start |
| `baseline_remaining` | OpenAI baseline amount minus used |
| `anthropic_total` | Anthropic month-to-date total (string) |
| `anthropic_fetched_at` | When Anthropic data was last fetched |
| `anthropic_baseline` | Anthropic baseline `{ amount, start_iso }` |
| `anthropic_baseline_used` | Anthropic usage since baseline start |
| `anthropic_baseline_remaining` | Anthropic baseline amount minus used |

Baselines are independent per provider — you can set one, both, or neither.

## Running / Building

Prerequisites:
- Rust toolchain (stable)
- Node.js + npm

### Quick start dev

Option A — standard Tauri workflow via npm (recommended):

```bash
npm install
npm run dev
```

Option B — using cargo/tauri directly:

```bash
cargo install tauri-cli
cargo tauri dev
```

### Build for release

```bash
# via npm
npm run build
# Bundles will be in src-tauri/target/release/bundle

# or with the Tauri CLI directly
cargo tauri build
```

For a verbose build log:

```bash
cargo build --release -vv 2>&1 | tee build.log
```

### Clean cargo cache

```bash
cd src-tauri
cargo clean
```

## API access requirements

- **OpenAI**: an admin API key with permission to read organization costs (`/v1/organization/costs`).
- **Anthropic**: an admin API key (`sk-ant-admin01-...`) with permission to read the organization cost report (`/v1/organizations/cost_report`).

Keys are stored locally in `credentials.json` and never transmitted anywhere except the respective API endpoint.

## Cache & Fetch behavior

- Cache is considered fresh for 3 minutes per provider (tracked independently via `fetched_at` and `anthropic_fetched_at`).
- Baseline config (`baseline` / `anthropic_baseline`) is always read from cache regardless of freshness, so the correct date range is used even before the first successful fetch.
- When a baseline is configured, the fetch command computes usage since the baseline start date and returns the remaining amount (baseline − used). The display label changes from `MTD` to `BLR`.
- When no baseline is set, the app fetches month-to-date totals from start of the current UTC month.
- The Anthropic provider uses `ending_at = start of today UTC` to avoid requesting incomplete or future daily buckets, which the API rejects. If the month just started (start of month equals today), the provider returns zero without making a network call.
- Both providers implement pagination and exponential backoff retries for transient errors (429 / 5xx).

## Development notes

- Network requests use reqwest; async code uses tokio
- Monetary aggregation uses rust_decimal to avoid floating-point rounding errors
- The Anthropic provider builds query strings manually to avoid reqwest percent-encoding colons in RFC 3339 timestamps, which the Anthropic API does not accept

### Debugging

```bash
RUST_LOG=debug npm run dev
```

If you run into environment issues on Linux (snap-related terminals, etc.), try running in a clean system terminal.

## Limitations & Future work

- UI/UX polishing, unit tests, packaging improvements and more robust config handling are future tasks
