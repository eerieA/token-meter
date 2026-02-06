from pathlib import Path
import traceback
import asyncio
from decimal import Decimal
from datetime import datetime, timezone
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor

from token_meter.ui.cost_popup import CostPopup
from token_meter.ui.baseline_dialog import BaselineDialog
from token_meter.storage import load_baseline, save_baseline, clear_baseline
from token_meter.config import REFRESH_INTERVAL_MS

RES_DIR = Path(__file__).parent.parent / "resources"


class UsageTray:
    def __init__(self, aggregator):
        # Debug: whether system tray is available on this platform
        print("Tray available:", QSystemTrayIcon.isSystemTrayAvailable())
        self.aggregator = aggregator
        self._refresh_task: asyncio.Task | None = None

        # Try to load an icon file from resources; fallback to a generated icon
        icon_path = RES_DIR / "icon.png"
        if icon_path.exists():
            icon = QIcon(str(icon_path))
        else:
            # Create a small programmatical icon
            pix = QPixmap(64, 64)
            pix.fill(QColor(0, 0, 0, 0))  # transparent background
            painter = QPainter(pix)
            # Use the RenderHint enum so Pylance recognizes the attribute
            painter.setRenderHint(QPainter.RenderHint.Antialiasing)
            painter.setBrush(QColor(30, 136, 229))
            painter.setPen(QColor(255, 255, 255))
            painter.drawEllipse(8, 8, 48, 48)
            painter.end()
            icon = QIcon(pix)

        self.tray = QSystemTrayIcon(icon)
        self.tray.setIcon(icon)
        self.menu = QMenu()

        # Top status action shows either usage or baseline info
        self.status = self.menu.addAction("Fetching usage...")

        # Actions for baseline management
        self.menu.addAction("Set baseline credits...", self._on_set_baseline)
        self._clear_baseline_action = self.menu.addAction(
            "Clear baseline", self._on_clear_baseline
        )

        # Separator and quit
        self.menu.addSeparator()
        # Quit should exit the application, not just hide the tray
        self.menu.addAction("Quit", QApplication.quit)

        self.tray.setContextMenu(self.menu)
        self.tray.show()

        # Initialize status based on existing baseline
        try:
            b = load_baseline()
            if b:
                amt = b.get("amount")
                start = b.get("start")
                self.status.setText(f"Baseline: ${amt} since {start}")
            else:
                self.status.setText("Fetching usage...")
        except Exception:
            # If anything goes wrong, keep the default status
            pass

        # Create the CostPopup and show a placeholder at startup.
        # Use the saved position if available, or bottom-right by default
        self._popup = CostPopup(auto_hide_ms=None)
        try:
            self._popup.show_placeholder()
            self._popup.show_with_animation()
        except Exception:
            # If positioning fails for any reason, just ensure the popup is visible
            self._popup.show()

        # Debug: balloon to confirm the tray icon is active
        """ try:
            self.tray.showMessage(
                "Token Meter", "Started", QSystemTrayIcon.MessageIcon.Information, 3000
            )
        except Exception:
            pass """

        # Timer for next refresh
        self.timer = QTimer()
        self.timer.timeout.connect(self.refresh)
        self.timer.start(REFRESH_INTERVAL_MS)

        # Schedule initial refresh
        self.refresh()

    def refresh(self):
        """Schedule a non-blocking refresh by creating a task for
        UsageAggregator.fetch() on the running asyncio event loop.

        Requires an active event loop (the app uses qasync). This function
        does NOT attempt to run a synchronous aggregator.fetch() and will
        raise or misbehave if no event loop is present.
        """
        # If a previous refresh task is running, don't start another
        if self._refresh_task and not self._refresh_task.done():
            return

        # Always treat aggregator.fetch() as an async coroutine. Schedule it on the
        # running event loop so the UI thread is never blocked.
        try:
            try:
                self._popup.show_status("Retrieving…")
            except Exception:
                pass

            coro = self.aggregator.fetch()
            loop = asyncio.get_event_loop()
            self._refresh_task = loop.create_task(self._refresh_async(coro))
        except Exception:
            tb = traceback.format_exc()
            print(tb)
            self.status.setText("Usage fetch failed (see console)")
            try:
                self._popup.show_status("Failed - see console log")
            except Exception:
                pass

    async def _refresh_async(self, coro):
        try:
            total = await coro
            # total is a Decimal, format to two decimals
            try:
                self.status.setText(f"OpenAI today: ${total:.2f}")
            except Exception:
                # Fallback: cast to float for display
                self.status.setText(f"OpenAI today: ${float(total):.2f}")

            try:
                self._popup.show_cost(total)
            except Exception:
                pass

            # If a baseline is configured, attempt to compute and show remaining
            try:
                b = load_baseline()
                if b:
                    amt = b.get("amount")
                    start = b.get("start")
                    try:
                        baseline_amount = Decimal(str(amt))
                    except Exception:
                        baseline_amount = None

                    if baseline_amount is not None and start:
                        try:
                            start_dt = datetime.fromisoformat(start)
                            if start_dt.tzinfo is None:
                                start_dt = start_dt.replace(tzinfo=timezone.utc)
                            # Fetch spent since baseline
                            spent = await self.aggregator.fetch_since(start_dt)
                            remaining = baseline_amount - spent

                            try:
                                self.status.setText(
                                    f"Remaining: ${remaining:.2f} since {start_dt.date().isoformat()}"
                                )
                            except Exception:
                                self.status.setText("Remaining: (see popup)")

                            try:
                                self._popup.show_remaining(
                                    remaining, spent, baseline_amount, start_dt
                                )
                            except Exception:
                                pass
                        except Exception:
                            pass
            except Exception:
                pass

        except Exception:
            tb = traceback.format_exc()
            print(tb)
            self.status.setText("Usage fetch failed (see console)")
            try:
                self._popup.show_status("Failed - see console log")
            except Exception:
                pass
        finally:
            self._refresh_task = None

    def _on_set_baseline(self):
        try:
            dlg = BaselineDialog()
            ok = dlg.exec()
            if ok:
                amount_str, start_iso = dlg.get_values()
                if amount_str and start_iso:
                    try:
                        save_baseline(amount_str, start_iso)
                        self.status.setText(
                            f"Baseline: ${amount_str} since {start_iso}"
                        )
                    except Exception:
                        # If saving fails, surface a message in the tray status
                        self.status.setText("Failed to save baseline (see console)")
        except Exception:
            pass

    def _on_clear_baseline(self):
        try:
            clear_baseline()
            self.status.setText("Baseline cleared")
        except Exception:
            # If clearing fails, ignore but log to console
            print("Failed to clear baseline")
