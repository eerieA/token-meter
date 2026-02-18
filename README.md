#  Token Meter

A lightweight token API usage & cost widget built with Rust + Tauri and a small HTML/CSS frontend.

This repository is a migration from the previous Rust + egui implementation, which was preceded by a Pyside 6 implementation. the UI is now a tiny web-based widget (web/index.html) and the native layer is provided by Tauri.

<!-- TOC -->

- [Token Meter](#token-meter)
    - [Features](#features)
    - [Project structure](#project-structure)
    - [Storage / Configuration](#storage--configuration)
    - [Running / Building](#running--building)
    - [How the UI works](#how-the-ui-works)
    - [token API access requirements](#token-api-access-requirements)
    - [Cache & Fetch behavior](#cache--fetch-behavior)
    - [Development notes](#development-notes)
    - [Debugging](#debugging)
    - [Limitations & Future work](#limitations--future-work)

<!-- /TOC -->

## Features

- Borderless, draggable widget window implemented with HTML/CSS
- Minimal web-based UI (web/index.html) talking to native Rust commands via Tauri
- Fetches month-to-date organization usage from the token API costs API
- Local JSON cache to avoid unnecessary API calls (cache considered stale after 1 hour)
- Retry & pagination logic for robust token API requests
- Uses rust_decimal for accurate monetary aggregation
- API key stored in the user's home directory (see Storage below)

Windows version preview is below. Linux verision preview will be here later.

<img alt="token meter windows ver preview" src="https://live.staticflickr.com/65535/55102591518_ba15bab69b.jpg" width="280">

Visual bug: now if it says MTD (Month To Date) then that is from cache json, if it is BLR (Baseline Remaining) then the cost numer is freshly obtained. I would make them both be BLR when a baseline is present. Will fix in the future.


## Project structure

```
.
├── src/                 # Core Rust library code (providers, aggregator, storage, domain)
├── src-tauri/           # Tauri app scaffolding and native entrypoint (build config, bundling)
│   └── src/main.rs      # Tauri commands that the web UI invokes (move_window, fetch, cache, etc.)
├── web/                 # Small web UI (HTML/CSS/JS) used as the Tauri frontend
└── package.json         # npm scripts to run/build the Tauri app
```

Important Tauri commands exposed to the UI (see src-tauri/src/main.rs):
- move_window(window, x, y) - move/position the native window
- get_api_key() - returns saved API key or an error
- get_cached_data() - returns cached MTD total if cache is still fresh
- fetch_month_to_date(api_key) - fetches MTD total from token API and returns JSON result (and saves cache on success)
- save_api_key_command(api_key) - writes the API key to disk

## Storage / Configuration

On first run the app needs an token API admin API key. The app stores configuration under a `.token-meter` directory in the user's home folder. Example locations:

- Unix/macOS: ~/.token-meter/
- Windows: C:\Users\<you>\.token-meter\

Files created:

```
~/.token-meter/
├── credentials.json    # { "openai_api_key": "..." }
└── api_usage.json      # cached data, timestamps, baseline info
```

Note: the code uses the user's home directory for storage (env::home_dir()).

## Running / Building

Prerequisites:
- Rust toolchain (stable)
- Node.js + npm

Quick start (development):

Option A - standard Tauri workflow via npm (recommended):

```bash
%  Install JS deps
npm install

%  Run the app in dev mode (this runs the Tauri dev workflow)
npm run dev
```

Option B - using the cargo-tauri commands directly:

```bash
%  If you don't have the tauri CLI installed, install it first
cargo install tauri-cli

%  Run the app in dev mode (hot-reloads frontend and native)
cargo tauri dev
```

Build for release:

Option A - via npm (uses the local Tauri CLI):

```bash
npm run build
%  The produced bundles are in src-tauri/target/release/bundle (platform-dependent)
```

Option B - with cargo-tauri directly:

```bash
%  Produce release bundles using the tauri CLI
cargo tauri build
%  Bundles will appear under src-tauri/target/release/bundle (platform-dependent)
```

. Or with very verbose log and save the log in a file.

```bash
cargo build --release -vv 2>&1 | tee build.log
```

`npm run dev` is convenient when working on the web frontend; `cargo tauri dev` is useful if you prefer invoking the native tooling directly.

## How the UI works

The web UI (web/index.html) is intentionally minimal. It:
- Invokes Tauri commands to read/save the API key and to fetch cached or fresh usage
- Displays MTD (month-to-date) cost and a status line
- Allows manual refresh via a button
- Implements dragging using a small drag-handle and the move_window Tauri command

## token API access requirements

- An admin API key that can read organization costs/usage is required.

## Cache & Fetch behavior

- Cache is considered fresh for 1 hour. If cache is fresh, the UI will show cached data on startup.
- fetch_month_to_date will query the token API organization costs endpoint, handling pagination and retry/backoff for transient errors.
- On successful fetch the native code saves the total to api_usage.json so subsequent starts can display cached data quickly.

## Development notes

- Network requests use reqwest; async code uses tokio
- Monetary aggregation uses rust_decimal to avoid floating-point rounding errors
- Errors and transient HTTP failures are retried with exponential backoff (see providers/openai.rs)

## Debugging

You can enable more verbose Rust logging with:

```bash
RUST_LOG=debug npm run dev
```

Or run the Rust binary directly with RUST_LOG set.

On Ubuntu and such, if use a terminal that is related to snap, dev run commands like `npm run dev` might encounter some lib paths issue. In such cases, try running in a clean system terminal.

## Limitations & Future work

- No multi-provider UI (the code is structured to add other providers)
