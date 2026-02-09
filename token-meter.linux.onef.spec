# PyInstaller spec for token-meter for Linux binary (onefile)
# Requires PyInstaller >= 6.3
# Build via: poetry run pyinstaller token-meter.linux.onef.spec

# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all
from PyInstaller.building.datastruct import Tree, TOC

# ------------------------
# Collect PySide6 files
# ------------------------
qt_binaries, qt_datas, qt_hiddenimports = collect_all("PySide6")

# ------------------------
# Prune unnecessary Qt libs/plugins
# ------------------------
essential_dlls = ("Qt6Core", "Qt6Gui", "Qt6Widgets")
# On Linux the platform plugin is libqxcb.so
platforms_plugin = "platforms/libqxcb.so"

qt_binaries = [
    b for b in qt_binaries
    if any(name in b[0] for name in essential_dlls)
       or b[0].endswith(platforms_plugin)
       or "imageformats" in b[0]
]

# ------------------------
# Remove Qt translations (not needed for English tray app)
# ------------------------
qt_datas = [d for d in qt_datas if "translations" not in d[0]]

# ------------------------
# Analysis
# ------------------------
a = Analysis(
    ["src/token_meter/main.py"],
    pathex=["src"],
    binaries=qt_binaries,
    datas=qt_datas,
    hiddenimports=qt_hiddenimports,
    hookspath=[],
    runtime_hooks=[],
    excludes=[],
    noarchive=False,  # onefile compresses by default
)

# ------------------------
# Include app resources
# ------------------------
a.datas += Tree(
    "src/token_meter/resources",
    prefix="token_meter/resources",
    typecode="DATA",
)

# ------------------------
# PYZ + EXE
# ------------------------
pyz = PYZ(a.pure, a.zipped_data, cipher=None)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="token-meter",
    icon="src/token_meter/resources/icon.png",
    console=False,
)
