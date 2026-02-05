# PyInstaller spec for token-meter for windows binary, onedir build
# Requires PyInstaller >= 6.3
# Built via: poetry run pyinstaller token-meter.spec

# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all
from PyInstaller.building.datastruct import Tree, TOC

# ------------------------
# Collect PySide6 files
# ------------------------
qt_binaries, qt_datas, qt_hiddenimports = collect_all("PySide6")

# ----------------------
# Prune unused Qt DLLs
# ----------------------
# Keep only essential DLLs for QtCore, QtGui, QtWidgets, platforms, imageformats
essential_dlls = ("Qt6Core", "Qt6Gui", "Qt6Widgets")
platforms_plugin = "platforms/qwindows.dll"
imageformat_plugin = "imageformats/qico.dll"  # tray icons use .ico

qt_binaries = [
    b for b in qt_binaries
    if any(name in b[0] for name in essential_dlls)
       or b[0].endswith(platforms_plugin)
       or b[0].endswith(imageformat_plugin)
]

# ----------------------
# Remove Qt translations
# ----------------------
qt_datas = [d for d in qt_datas if "translations" not in d[0]]

# ----------------------
# Build Analysis
# ----------------------
a = Analysis(
    ["src/token_meter/main.py"],
    pathex=["src"],
    binaries=qt_binaries,
    datas=qt_datas,
    hiddenimports=qt_hiddenimports,
    hookspath=[],
    runtime_hooks=[],
    excludes=[],
    noarchive=True,
)

# ----------------------
# Add application resources
# ----------------------
a.datas += Tree(
    "src/token_meter/resources",
    prefix="token_meter/resources",
    typecode="DATA",
)

# ----------------------
# PYZ and EXE
# ----------------------
pyz = PYZ(a.pure, a.zipped_data, cipher=None)

exe = EXE(
    pyz,
    a.scripts,
    exclude_binaries=True,
    name="token-meter",
    icon="src/token_meter/resources/icon.ico",
    console=False,
)

# ----------------------
# COLLECT to onedir
# ----------------------
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=True,  # compress DLLs slightly for additional size reduction
    name="token-meter",
)
