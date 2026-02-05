from PySide6.QtWidgets import (
    QDialog,
    QLabel,
    QLineEdit,
    QPushButton,
    QHBoxLayout,
    QVBoxLayout,
    QDateEdit,
    QTimeEdit,
)
from PySide6.QtCore import Qt, QDate, QTime
from datetime import datetime, timezone
from decimal import Decimal


class BaselineDialog(QDialog):
    """Dialog to enter a baseline credits amount and a start date/time.

    The dialog returns the entered amount as a string and the ISO-8601
    datetime string in UTC (e.g. 2024-01-01T00:00:00+00:00).
    Note: for simplicity this first implementation treats the entered
    date/time as UTC.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Set baseline credits")
        self.setModal(True)

        self.info = QLabel("Enter the remaining credits on the chosen baseline date/time:")

        self.amount_label = QLabel("Amount (USD):")
        self.amount_input = QLineEdit()
        self.amount_input.setPlaceholderText("e.g. 100.00")

        self.date_label = QLabel("Baseline date:")
        self.date_input = QDateEdit()
        self.date_input.setCalendarPopup(True)
        self.date_input.setDate(QDate.currentDate())

        self.time_label = QLabel("Baseline time (UTC):")
        self.time_input = QTimeEdit()
        self.time_input.setTime(QTime(0, 0, 0))

        self.error_label = QLabel("")
        self.error_label.setStyleSheet("color: #f00")

        self.ok_button = QPushButton("OK")
        self.cancel_button = QPushButton("Cancel")

        btn_layout = QHBoxLayout()
        btn_layout.addStretch()
        btn_layout.addWidget(self.cancel_button)
        btn_layout.addWidget(self.ok_button)

        layout = QVBoxLayout()
        layout.addWidget(self.info)
        layout.addWidget(self.amount_label)
        layout.addWidget(self.amount_input)
        layout.addWidget(self.date_label)
        layout.addWidget(self.date_input)
        layout.addWidget(self.time_label)
        layout.addWidget(self.time_input)
        layout.addWidget(self.error_label)
        layout.addLayout(btn_layout)

        self.setLayout(layout)

        self.ok_button.clicked.connect(self._on_ok)
        self.cancel_button.clicked.connect(self.reject)

    def _on_ok(self):
        txt = self.amount_input.text().strip()
        if not txt:
            self.error_label.setText("Please enter an amount.")
            return
        try:
            # Validate decimal
            Decimal(txt)
        except Exception:
            self.error_label.setText("Amount must be a decimal number (e.g. 123.45).")
            return

        # Compose datetime in UTC from date/time widgets
        qdate = self.date_input.date()
        qtime = self.time_input.time()
        dt = datetime(
            qdate.year(),
            qdate.month(),
            qdate.day(),
            qtime.hour(),
            qtime.minute(),
            qtime.second(),
            tzinfo=timezone.utc,
        )

        # Save results on the instance and accept
        self._amount = txt
        self._start_iso = dt.isoformat()
        self.accept()

    def get_values(self):
        """Return (amount_str, start_iso) after the dialog is accepted.

        If dialog was rejected, returns (None, None).
        """
        return getattr(self, "_amount", None), getattr(self, "_start_iso", None)
