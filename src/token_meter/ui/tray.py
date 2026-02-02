from pathlib import Path
from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor

RES_DIR = Path(__file__).parent.parent / "resources"


class UsageTray:
    def __init__(self, aggregator):
        # Debug: whether system tray is available on this platform
        print("Tray available:", QSystemTrayIcon.isSystemTrayAvailable())
        self.aggregator = aggregator

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

        # Debug: balloon to confirm the tray icon is active
        try:
            self.tray.showMessage(
                "Token Meter", "Started", QSystemTrayIcon.MessageIcon.Information, 3000
            )
        except Exception:
            pass

        self.timer = QTimer()
        self.timer.timeout.connect(self.refresh)
        self.timer.start(5 * 60 * 1000)

        self.refresh()

    def refresh(self):
        try:
            total = self.aggregator.fetch()
            self.status.setText(f"OpenAI today: ${total:.2f}")
        except Exception:
            self.status.setText("Usage fetch failed")
