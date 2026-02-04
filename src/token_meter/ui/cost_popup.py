from typing import Optional
from PySide6.QtWidgets import QWidget, QLabel, QVBoxLayout
from PySide6.QtCore import (
    Qt,
    QTimer,
    QPropertyAnimation,
    QEasingCurve,
    QPoint,
)
from PySide6.QtGui import QFont, QGuiApplication, QCursor
from decimal import Decimal


class CostPopup(QWidget):
    def __init__(self, parent=None, auto_hide_ms: Optional[int] = 8000):
        flags = (
            Qt.WindowType.Tool
            | Qt.WindowType.FramelessWindowHint
            | Qt.WindowType.WindowStaysOnTopHint
        )
        super().__init__(parent, flags)
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        # Do not take keyboard focus when shown
        self.setWindowFlag(Qt.WindowType.WindowDoesNotAcceptFocus, True)

        self.auto_hide_ms = auto_hide_ms
        self._hide_timer = QTimer(self)
        self._hide_timer.setSingleShot(True)
        self._hide_timer.timeout.connect(self.hide_with_animation)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(16, 12, 16, 12)
        layout.setSpacing(6)

        self.value_label = QLabel("$--.--")
        big_font = QFont()
        big_font.setPointSize(28)
        big_font.setBold(True)
        self.value_label.setFont(big_font)
        self.value_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        self.status_label = QLabel("")
        small_font = QFont()
        small_font.setPointSize(10)
        self.status_label.setFont(small_font)
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)

        layout.addWidget(self.value_label)
        layout.addWidget(self.status_label)

        self.setStyleSheet(
            """
            QWidget {
                background: rgba(18,18,18,0.92);
                color: white;
                border-radius: 10px;
            }
        """
        )

        self._anim = QPropertyAnimation(self, b"windowOpacity", self)
        self._anim.setDuration(220)
        self._anim.setEasingCurve(QEasingCurve.Type.InOutCubic)

    def show_at_cursor(self, offset: Optional[QPoint] = None):
        if offset is None:
            offset = QPoint(10, 10)

        pos = QCursor.pos() + offset
        self.adjustSize()
        # Ensure the popup is entirely on-screen (simple clamp)
        screen = QGuiApplication.screenAt(pos)
        if screen:
            geo = screen.availableGeometry()
            x = min(max(geo.left(), pos.x()), geo.right() - self.width())
            y = min(max(geo.top(), pos.y()), geo.bottom() - self.height())
            self.move(x, y)
        else:
            self.move(pos)
        self.show_with_animation()

    def show_with_animation(self):
        self._anim.stop()
        self.setWindowOpacity(0.0)
        self.show()
        self._anim.setStartValue(0.0)
        self._anim.setEndValue(1.0)
        self._anim.start()
        if self.auto_hide_ms:
            self._hide_timer.start(self.auto_hide_ms)

    def hide_with_animation(self):
        if not self.isVisible():
            return
        self._anim.stop()
        self._anim.setStartValue(self.windowOpacity())
        self._anim.setEndValue(0.0)
        # disconnect previous to avoid multiple connects
        try:
            self._anim.finished.disconnect(self.hide)
        except Exception:
            pass
        self._anim.finished.connect(self.hide)
        self._anim.start()

    def show_cost(self, amount: Decimal | float | str):
        try:
            if isinstance(amount, Decimal):
                val = float(amount)
            else:
                val = float(amount)
            self.value_label.setText(f"${val:,.2f}")
        except Exception:
            # Fallback to raw string
            self.value_label.setText(str(amount))
        self.status_label.setText("")
        self.show_with_animation()

    def show_status(self, text: str):
        self.status_label.setText(text)
        self.show_with_animation()

    def show_placeholder(self):
        self.value_label.setText("$--.--")
        self.status_label.setText("")
        self.show_with_animation()
