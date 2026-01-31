import sys
import os
from PySide6.QtWidgets import QApplication
from token_meter.ui.tray import UsageTray
from token_meter.aggregator import UsageAggregator

# Module-level reference to the tray object. This is REQUIRED to keep the QSystemTrayIcon alive.
# And add it in global (module scope) to avoid static-typing warnings
_tray: UsageTray | None = None


def main():
    global _tray

    # QApplication must be created before any Qt widgets, including QSystemTrayIcon.
    app = QApplication(sys.argv)

    # This is a tray-only application no open windows.
    app.setQuitOnLastWindowClosed(False)

    openai_key = os.environ.get("OPENAI_API_KEY")
    if not openai_key:
        raise RuntimeError("OPENAI_API_KEY not set")

    # Aggregator encapsulates all network and logic stuff. UI layer should be seprated from it.
    aggregator = UsageAggregator(openai_key)

    _tray = UsageTray(aggregator)

    # Enter the Qt event loop. This call blocks until the user selects "Quit" from the tray menu.
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
