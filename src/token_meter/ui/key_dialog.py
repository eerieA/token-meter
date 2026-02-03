from PySide6.QtWidgets import QDialog, QLabel, QLineEdit, QCheckBox, QPushButton, QHBoxLayout, QVBoxLayout
from PySide6.QtCore import Qt


class KeyEntryDialog(QDialog):
    """Simple dialog to prompt the user for an API key and optionally save it."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle("OpenAI API Key")
        self.setModal(True)

        self.label = QLabel("Enter your OpenAI admin API key:")
        self.input = QLineEdit()
        self.input.setEchoMode(QLineEdit.EchoMode.Password)
        self.save_checkbox = QCheckBox("Save key to local config (recommended)")

        self.ok_button = QPushButton("OK")
        self.cancel_button = QPushButton("Cancel")

        btn_layout = QHBoxLayout()
        btn_layout.addStretch()
        btn_layout.addWidget(self.cancel_button)
        btn_layout.addWidget(self.ok_button)

        layout = QVBoxLayout()
        layout.addWidget(self.label)
        layout.addWidget(self.input)
        layout.addWidget(self.save_checkbox)
        layout.addLayout(btn_layout)

        self.setLayout(layout)

        self.ok_button.clicked.connect(self.accept)
        self.cancel_button.clicked.connect(self.reject)

    def get_values(self):
        return self.input.text().strip(), self.save_checkbox.isChecked()
