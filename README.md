This is a minimal Rust + egui prototype of the token-meter app, which was originally built with PySide6.

Structure:
- Cargo.toml
- src/
  - main.rs        : egui application and background worker
  - providers/openai.rs : OpenAI HTTP provider + pagination + retries
  - aggregator.rs  : high-level aggregation helpers
  - storage.rs     : simple config/cache persistence to user config dir
  - domain.rs      : small UsageRecord definition

What it implements:
- Async OpenAI cost fetching (pagination + simple retry) using reqwest + tokio
- Aggregation of month-to-date total into a Decimal
- Small egui window allowing entering API key, saving it, and triggering a fetch
- Network work runs on a background tokio runtime thread and results are posted back to the UI via channels

What it does NOT implement (for scoping and tech stack practicals):
- No system tray / context menu
- No animated frameless popup

How to run:
- Install Rust toolchain
- From the token-meter-egui directory run: cargo run --release

How the prototype works (quick)
- The egui window shows a text field for the OpenAI admin API key, a Save button (saves to user config dir), and a Fetch button.
- When you click Fetch, the UI sends a request to the background worker.
- A tokio runtime on a background thread performs the HTTP requests (pagination + retry) and sends back either success with Decimal total or a failure message.
- The UI displays status and the resulting total.

Notes and caveats
- This is a minimal prototype focused on the backend HTTP calls, aggregation, and a simple widget UI. It intentionally does not implement a system tray or platform-specific popup.
- Used rust_decimal for monetary calculations to keep precision similar to the Python Decimal usage.
- The OpenAI response parsing assumes the organization/costs endpoint shape similar to what the old Python code expects (buckets with results that have amount.value). If the API responses differ, we can adapt the deserialization.

Next steps possibly
- Add automatic periodic refresh (like the original app’s QTimer) that triggers fetches on an interval.
- Add a small settings UI for baseline credits and persistence (storage has helpers; UI wiring remains).
- Implement a more resilient background worker (task queue, cancellation support).
- Implement system tray + popup (this requires platform-specific tray crate or integration).
- Tune Cargo features to further reduce binary size (strip, s or z opt-levels, disable default features).
