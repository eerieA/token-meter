# Token Meter

A lightweight token API usage & cost widget built with Rust + Tauri and a small HTML/CSS frontend.

This repo is a migration from previous Pyside6 stack.

<!-- TOC -->

- [Token Meter](#token-meter)
    - [Features](#features)
    - [Project structure](#project-structure)
    - [Storage / Configuration](#storage--configuration)
    - [Running / Building](#running--building)
    - [token API access requirements](#token-api-access-requirements)
    - [Cache & Fetch behavior](#cache--fetch-behavior)
    - [Development notes](#development-notes)
        - [Debugging](#debugging)
    - [Limitations & Future work](#limitations--future-work)

<!-- /TOC -->

## Features

- Borderless, draggable widget window implemented with HTML/CSS
- Minimal web-based UI (web/index.html) talking to native Rust commands via Tauri
- Fetches month-to-date organization usage from the token API costs endpoint
- Baseline mode: optionally record a baseline credit and compute remaining credit since the baseline start date
- Local JSON cache to avoid too many API calls (cache considered stale after 3 minutes)
- Retry & pagination logic for robust token API requests
- Uses rust_decimal for accurate monetary aggregation
- API key stored in the user's home directory (see Storage below)

Windows version preview and Linux version preview (Ubuntu with GNOME X11).

<img alt="token meter windows ver preview" src="https://live.staticflickr.com/65535/55102591518_ba15bab69b.jpg" width="320">
<img alt="token meter linux ver preview" src="https://live.staticflickr.com/65535/55106010788_80ddac54f0.jpg" width="320">

> You can see there is ghosting problem on GNOME X11. But it may require significant effort to fix so probably not gonna get fixed.

## Project structure

```
.
├── src/                 # Core Rust library code (providers, aggregator, storage, domain)
├── src-tauri/           # Tauri app scaffolding and native entrypoint (build config, bundling)
│   └── src/main.rs      # Tauri commands that the web UI invokes
├── web/                 # Small web UI (HTML/CSS/JS) used as the Tauri frontend
└── package.json         # npm scripts to run/build the Tauri app
```

Important Tauri commands exposed to the UI (see src-tauri/src/main.rs):
- get_api_key() - returns saved API key (or null)
- get_cached_data() - returns cached data (may include baseline fields) if cache is still fresh
- fetch_month_to_date(api_key) - fetches MTD or baseline-derived remaining credit from the provider (and saves cache on success)
- validate_api_key(api_key) - quick validation by performing a fetch
- save_api_key_command(api_key) - writes the API key to disk
- save_baseline_command(amount, start_iso) - persist a baseline amount and start date
- clear_baseline_command() - remove the configured baseline
- show_context_menu(x, y) / hide_context_menu() - native overlay for context menu
- show_baseline_modal(x, y) / hide_baseline_modal() - native overlay for baseline modal (fallback to in-DOM modal available)
- quit_app() - exit the application

## Storage / Configuration

On first run the app requires an admin token API key (able to read organization costs/usage). The app stores configuration under a `.token-meter` directory in the user's home folder. Example locations:

- Unix/macOS: ~/.token-meter/
- Windows: C:\Users\<you>\.token-meter\

Files created:

```
~/.token-meter/
├── credentials.json    # { "openai_api_key": "..." }
└── api_usage.json      # cached data, timestamps, baseline metadata, baseline aggregates
```

api_usage.json fields of interest:
- openai_total: the month-to-date total fetched from the provider (string)
- fetched_at: ISO timestamp when those values were saved
- baseline: optional object { amount, start_iso }
- baseline_used: optional string representing usage since baseline start
- baseline_remaining: optional string representing baseline amount minus used

Behavior notes:
- When a baseline is configured, the app computes used & remaining and saves baseline_used / baseline_remaining without overwriting openai_total. This lets the app show either MTD totals or baseline remaining depending on context.

## Running / Building

Prerequisites:
- Rust toolchain (stable)
- Node.js + npm

Quick start (development):

Option A - standard Tauri workflow via npm (recommended):

```bash
// Install JS deps
npm install

// Run the app in dev mode (this runs the Tauri dev workflow)
npm run dev
```

Option B - using the cargo/tauri commands directly:

```bash
// If you don't have the tauri CLI installed, install it first
cargo install tauri-cli

// Run the app in dev mode (hot-reloads frontend and native)
cargo tauri dev
```

Build for release:

```bash
// via npm (uses the local Tauri CLI)
npm run build
// Bundles will be in src-tauri/target/release/bundle

// or with the Tauri CLI directly
cargo tauri build
```

For a very verbose build log:

```bash
cargo build --release -vv 2>&1 | tee build.log
```

## token API access requirements

- An admin API key that can read organization costs/usage is required (this project has an OpenAI provider implementation as an example). The key is stored locally in credentials.json.

## Cache & Fetch behavior

- Cache is considered fresh for 3 minutes. If cache is fresh, the UI will show cached data on startup.
- When a baseline is configured, fetch_month_to_date computes usage since the baseline start date and returns the remaining amount (baseline - used). The native code saves baseline_used and baseline_remaining to the cache so the frontend can show BLR from cache without re-querying.
- When no baseline is present, the app fetches month-to-date totals from the provider and saves openai_total.
- fetch_month_to_date handles pagination and retries/backoff for transient errors.

## Development notes

- Network requests use reqwest; async code uses tokio
- Monetary aggregation uses rust_decimal to avoid floating-point rounding errors
- Errors and transient HTTP failures are retried with exponential backoff (see src/providers/openai.rs)

### Debugging

You can enable more verbose Rust logging with:

```bash
RUST_LOG=debug npm run dev
```

If you run into environment issues on Linux (snap-related terminals, etc.), try running in a clean system terminal.

## Limitations & Future work

- No multi-provider UI yet (the code is structured to add other providers easily)
- UI/UX polishing, unit tests, packaging improvements and more robust config handling are future tasks
