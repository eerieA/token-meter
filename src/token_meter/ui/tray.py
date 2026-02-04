from pathlib import Path
import traceback
import asyncio
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor

from token_meter.ui.cost_popup import CostPopup

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
        self.status = self.menu.addAction("Fetching usage...")
        self.menu.addSeparator()
        # Quit should exit the application, not just hide the tray
        self.menu.addAction("Quit", QApplication.quit)

        self.tray.setContextMenu(self.menu)
        self.tray.show()

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
                self._popup.show_status("Failed — see console")
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
                self._popup.show_status("Failed — see console")
            except Exception:
                pass
        finally:
            self._refresh_task = None
