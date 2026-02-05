from typing import Optional
from PySide6.QtWidgets import QWidget, QLabel, QVBoxLayout
from PySide6.QtCore import (
    Qt,
    QTimer,
    QPropertyAnimation,
    QEasingCurve,
    QPoint,
    QSettings,
)
from PySide6.QtGui import QFont, QGuiApplication, QCursor
from decimal import Decimal


class CostPopup(QWidget):
    """A small, frameless popup that displays the current cost.

    It:
    - Remembers the last position using QSettings and shows there by default.
    - Defaults to bottom-right of the active screen if no saved position.
    - Makes the popup draggable (click+drag anywhere on it).
    - Saves the new position after dragging.
    """

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

        # Drag / position state
        self._dragging = False
        self._drag_offset = QPoint(0, 0)

        # Settings to persist last position
        self._settings = QSettings("token_meter", "token_meter")

    def _load_saved_pos(self) -> Optional[QPoint]:
        if self._settings.contains("cost_popup/pos_x") and self._settings.contains(
            "cost_popup/pos_y"
        ):
            try:
                x = int(self._settings.value("cost_popup/pos_x"))
                y = int(self._settings.value("cost_popup/pos_y"))
                return QPoint(x, y)
            except Exception:
                return None
        return None

    def _save_pos(self, pos: QPoint):
        try:
            self._settings.setValue("cost_popup/pos_x", int(pos.x()))
            self._settings.setValue("cost_popup/pos_y", int(pos.y()))
            self._settings.sync()
        except Exception:
            pass

    def _clamp_to_screen(self, pos: QPoint) -> QPoint:
        self.adjustSize()
        screen = QGuiApplication.screenAt(pos) or QGuiApplication.primaryScreen()
        if not screen:
            return pos
        geo = screen.availableGeometry()
        x = min(max(geo.left(), pos.x()), geo.right() - self.width())
        y = min(max(geo.top(), pos.y()), geo.bottom() - self.height())
        return QPoint(x, y)

    def show_at_cursor(self, offset: Optional[QPoint] = None):
        # Keep backwards-compatible explicit cursor placement
        if offset is None:
            offset = QPoint(10, 10)

        pos = QCursor.pos() + offset
        pos = self._clamp_to_screen(pos)
        self.move(pos)
        self.show_with_animation()

    def show_with_animation(self):
        # Position the popup before showing. If the user is dragging, don't override their position.
        if not self._dragging:
            saved = self._load_saved_pos()
            if saved:
                pos = self._clamp_to_screen(saved)
                self.move(pos)
            else:
                # Default to bottom-right of the active screen
                screen = (
                    QGuiApplication.screenAt(QCursor.pos())
                    or QGuiApplication.primaryScreen()
                )
                if screen:
                    geo = screen.availableGeometry()
                    self.adjustSize()
                    margin = 12
                    x = geo.right() - self.width() - margin
                    y = geo.bottom() - self.height() - margin
                    self.move(x, y)

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

    def mousePressEvent(self, event):
        if event.buttons() & Qt.MouseButton.LeftButton:
            self._dragging = True
            # event.pos() is relative to widget, use it to preserve cursor offset
            self._drag_offset = event.pos()
            # Keep the popup visible while dragging
            self._hide_timer.stop()
            # Slightly reduce opacity while dragging for feedback
            try:
                self.setWindowOpacity(0.95)
            except Exception:
                pass
        super().mousePressEvent(event)

    def mouseMoveEvent(self, event):
        if self._dragging:
            # Compute new top-left such that cursor stays at same offset
            global_pos = (
                event.globalPosition().toPoint()
                if hasattr(event, "globalPosition")
                else event.globalPos()
            )
            new_top_left = global_pos - self._drag_offset
            new_top_left = self._clamp_to_screen(new_top_left)
            self.move(new_top_left)
        super().mouseMoveEvent(event)

    def mouseReleaseEvent(self, event):
        if self._dragging and not (event.buttons() & Qt.MouseButton.LeftButton):
            self._dragging = False
            # Save the final position
            try:
                self._save_pos(self.pos())
            except Exception:
                pass
            # restart auto-hide timer when released
            if self.auto_hide_ms:
                self._hide_timer.start(self.auto_hide_ms)
            try:
                self.setWindowOpacity(1.0)
            except Exception:
                pass
        super().mouseReleaseEvent(event)

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

    def show_remaining(
        self,
        remaining: Decimal | float | str,
        spent: Decimal | float | str,
        baseline_amount: Decimal | float | str,
        start_dt=None,
    ):
        """Display remaining credits prominently with color and a tooltip with details.

        start_dt may be a datetime or an ISO string; the tooltip will include the baseline start.
        """
        try:
            # Convert to floats for formatting safely
            if isinstance(remaining, Decimal):
                rem_val = float(remaining)
            else:
                rem_val = float(remaining)
        except Exception:
            # Fallback to raw string display
            self.value_label.setText(str(remaining))
            self.status_label.setText("")
            self.show_with_animation()
            return

        # Choose color: red if negative, orange if low, green otherwise
        color = "#4caf50"  # green
        try:
            # Determine thresholds
            rem_dec = Decimal(str(remaining))
            base_dec = Decimal(str(baseline_amount))
            if rem_dec < 0:
                color = "#e53935"  # red
            else:
                # low if less than 5% of baseline or absolute < 5
                try:
                    if base_dec > 0 and (rem_dec / base_dec) < Decimal("0.05"):
                        color = "#fb8c00"  # orange
                    elif rem_dec < Decimal("5"):
                        color = "#fb8c00"
                except Exception:
                    pass
        except Exception:
            # ignore and keep default color
            pass

        # Update the value label with colored text
        self.value_label.setText(f"${rem_val:,.2f}")
        # Apply inline style for color to the value_label
        try:
            self.value_label.setStyleSheet(f"color: {color};")
        except Exception:
            pass

        # Status line with spent and baseline
        try:
            if isinstance(spent, Decimal):
                spent_val = float(spent)
            else:
                spent_val = float(spent)
            if isinstance(baseline_amount, Decimal):
                base_val = float(baseline_amount)
            else:
                base_val = float(baseline_amount)
            # Format start_dt for display
            start_display = None
            try:
                if start_dt is None:
                    start_display = ""
                elif hasattr(start_dt, "isoformat"):
                    start_display = start_dt.date().isoformat()
                else:
                    # assume string
                    start_display = str(start_dt).split("T")[0]
            except Exception:
                start_display = str(start_dt)

            if start_display:
                self.status_label.setText(
                    f"Since {start_display}: spent ${spent_val:,.2f}"
                )
            else:
                self.status_label.setText(f"Spent ${spent_val:,.2f} since baseline")

            # Tooltip with more details
            tooltip = f"Baseline: ${base_val:,.2f} since {start_display}\nSpent since baseline: ${spent_val:,.2f}\nRemaining: ${rem_val:,.2f}"
            self.setToolTip(tooltip)
        except Exception:
            self.status_label.setText("")

        self.show_with_animation()
