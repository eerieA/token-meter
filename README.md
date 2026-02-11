# Token Meter

A lightweight Rust + egui application that monitors OpenAI API usage and costs in a borderless widget window.

> Originally implemented using Pyside6; see branch legacy-pyside6.

## Features

- **Automatic Setup**: First-run setup wizard for API key configuration
- **Smart Caching**: Local cache reduces API calls and improves responsiveness
- **Real-time Updates**: Fetch current month-to-date usage from OpenAI API
- **Secure Storage**: API key stored in `~/.token-meter/credentials.json`
- **Background Processing**: Non-blocking UI with async background worker
- **Retry Logic**: Built-in retry mechanism for network reliability
- **Pagination**: Handles OpenAI API pagination automatically

## Project Structure

```
src/
├── main.rs              # Main egui application and UI state management
├── providers/
│   ├── mod.rs           # Provider module exports
│   └── openai.rs       # OpenAI HTTP client with pagination & retries
├── aggregator.rs        # High-level usage aggregation logic
├── storage.rs          # Configuration and cache persistence
└── domain.rs           # Data structures and type definitions
```

## How It Works

### First Run Setup
1. Launch the application - it detects if no API key is stored
2. Enter your OpenAI admin API key in the setup window
3. Click "Save & Start" - the key is stored securely and initial data is fetched

### Normal Operation
- **Automatic Loading**: On startup, loads cached data if fresh (within 1 hour)
- **Smart Refreshing**: Fetches new data if cache is outdated or missing
- **Manual Refresh**: Click "📊 Refresh" to force update from API
- **Persistent Storage**: Costs data cached in `~/.token-meter/api_usage.json`

### Data Flow
1. UI sends fetch request to background worker thread
2. Background tokio runtime makes HTTP requests to OpenAI API
3. Results are sent back to UI via channels
4. UI updates display and saves data to cache
5. Cache persists across app restarts

## Installation & Running

### Prerequisites
- Rust toolchain (latest stable recommended)

### Build & Run
```bash
# Clone and navigate to project
cd token-meter

# Run in debug mode
cargo run

# Or build optimized release
cargo run --release
```

### Configuration Files
The app creates a `.token-meter` directory in user home folder:

```
~/.token-meter/
├── credentials.json    # Stored API key
└── api_usage.json    # Cached usage data with timestamps
```

> Similarly on Windows the files are stored in `%APPDATA%/token-meter`

## API Requirements

- **OpenAI Admin API Key**: Required to access organization usage data
- **Permissions**: Must have organization usage read permissions

## UI Components

### Main Widget
- **Draggable Header**: Click and drag to move the window
- **Usage Display**: Shows month-to-date total cost
- **Status Indicator**: Current operation status (Loading, Fetched, etc.)
- **Refresh Button**: Force update from OpenAI API

### Setup Window (First Run Only)
- **API Key Input**: Secure text field for your OpenAI admin key
- **Save & Start**: Stores key and transitions to main widget
- **Cancel**: Close the application

## Technical Details

### Caching System
- **Cache Duration**: 1 hour before considered outdated
- **Cache Format**: JSON with timestamps for validation
- **Automatic Updates**: Cache updated on every successful fetch

### Error Handling
- **Network Retries**: Automatic retry logic for failed requests
- **Graceful Degradation**: Shows cached data if API is unavailable
- **User Feedback**: Clear status messages for all operations

### Performance Optimizations
- **Background Threading**: Non-blocking UI operations
- **Decimal Precision**: Uses `rust_decimal` for accurate monetary calculations

## Development Notes

### Architecture Decisions
- **egui**: Immediate mode GUI for simplicity and performance
- **tokio**: Async runtime for efficient network operations
- **reqwest**: HTTP client with TLS support
- **serde**: JSON serialization for API responses and cache

### Security Considerations
- API key stored in user home directory (not in application directory)
- No API key display in main UI after setup
- Plain text storage (acceptable for personal utility application)

## Limitations

- No system tray integration
- No automatic periodic refresh (manual refresh only)
- No baseline credit tracking (storage helpers available)
- Windows/Linux only (tested platforms)

## Future Enhancements

- **System Tray**: Background operation with tray icon
- **Auto-refresh**: Configurable automatic update intervals
- **Baseline Tracking**: Compare usage against allocated credits
- **Multiple Providers**: Support for other AI service providers
- **Usage History**: Track usage trends over time

## Troubleshooting

### Common Issues
1. **"Failed to save API key"**: Check write permissions to home directory
2. **"Failed to fetch"**: Verify API key has organization access
3. **No data showing**: Check internet connection and API key validity

### Debug Information
Run with `RUST_LOG=debug cargo run` to see detailed logging.

## License

This project maintains the same license as the original token-meter application.
