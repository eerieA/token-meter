from pathlib import Path
import traceback
import asyncio
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor

from token_meter.ui.cost_popup import CostPopup
from token_meter.ui.baseline_dialog import BaselineDialog
from token_meter.storage import load_baseline, save_baseline, clear_baseline

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

        self.timer = QTimer()
        self.timer.timeout.connect(self.refresh)
        self.timer.start(5 * 60 * 1000)

        # Schedule initial refresh
        self.refresh()

    def refresh(self):
        """Start a non-blocking refresh. If the aggregator.fetch is async it will
        be scheduled on the asyncio event loop; otherwise we call it synchronously.
        """
        # If a previous refresh task is running, don't start another
        if self._refresh_task and not self._refresh_task.done():
            return

        try:
            result = self.aggregator.fetch()
            # If fetch() returned a coroutine, schedule it on the asyncio loop
            if asyncio.iscoroutine(result):
                # Show fetching status in the popup
                try:
                    self._popup.show_status("Retrieving…")
                except Exception:
                    pass

                loop = asyncio.get_event_loop()
                self._refresh_task = loop.create_task(self._refresh_async(result))
            else:
                # Synchronous value
                total = result
                self.status.setText(f"OpenAI today: ${total:.2f}")
                try:
                    self._popup.show_cost(total)
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
