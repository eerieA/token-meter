import sys
import os
import asyncio
from PySide6.QtWidgets import QApplication, QDialog
from qasync import QEventLoop
from token_meter.ui.tray import UsageTray
from token_meter.aggregator import UsageAggregator
from token_meter.keystore import load_api_key, save_api_key
from token_meter.ui.key_dialog import KeyEntryDialog
from token_meter.logger import get_logger

logger = get_logger(__name__)


# Module-level reference to the tray object. This is REQUIRED to keep the QSystemTrayIcon alive.
# And add it in global (module scope) to avoid static-typing warnings
_tray: UsageTray | None = None


def main():
    global _tray

    # QApplication must be created before any Qt widgets, including QSystemTrayIcon.
    app = QApplication(sys.argv)

    # This is a tray-only application no open windows.
    app.setQuitOnLastWindowClosed(False)

    # Run the qasync event loop (integrates asyncio with the Qt event loop).
    loop = QEventLoop(app)
    asyncio.set_event_loop(loop)

    # Try environment, then local keystore
    openai_key = os.environ.get("OPENAI_API_KEY") or load_api_key()

    if not openai_key:
        # Prompt the user for a key using a simple dialog.
        dlg = KeyEntryDialog()
        result = dlg.exec()
        if result == QDialog.DialogCode.Accepted:
            key, save = dlg.get_values()
            if not key:
                raise RuntimeError("OpenAI API key not provided")
            if save:
                try:
                    save_api_key(key)
                except Exception:
                    logger.exception("Failed to save API key to keystore")
            openai_key = key
        else:
            raise RuntimeError("OpenAI API key not set")

    # Aggregator encapsulates all network and logic stuff. UI layer should be seprated from it.
    aggregator = UsageAggregator(openai_key)

    _tray = UsageTray(aggregator)

    try:
        with loop:
            # The tray's constructor schedules an initial refresh; start the loop and
            # keep running until the application quits (e.g., user selects Quit).
            loop.run_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
