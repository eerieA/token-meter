from PySide6.QtWidgets import QSystemTrayIcon, QMenu, QApplication
from PySide6.QtCore import QTimer
from PySide6.QtGui import QIcon, QPixmap, QPainter, QColor

class UsageTray:
    def __init__(self, aggregator):
        # Debug: whether system tray is available on this platform
        print("Tray available:", QSystemTrayIcon.isSystemTrayAvailable())
        self.aggregator = aggregator

        # Create a small default icon placeholder
        pix = QPixmap(64, 64)
        pix.fill(QColor(0, 0, 0, 0))  # transparent background
        painter = QPainter(pix)
        painter.setRenderHint(QPainter.Antialiasing)
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
            self.tray.showMessage("Token Meter", "Started", QSystemTrayIcon.Information, 3000)
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

