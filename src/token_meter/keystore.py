import json
import os
from pathlib import Path
from typing import Optional
from token_meter.logger import get_logger

logger = get_logger(__name__)

CONFIG_DIR = Path.home() / ".token-meter"
CRED_PATH = CONFIG_DIR / "credentials.json"


def save_api_key(api_key: str) -> None:
    """Save the API key to a local credentials file with restrictive permissions.

    The key is stored in plaintext JSON in ~/.token_meter/credentials.json with
    an attempt to set file mode to 0o600 where supported. This is intentionally simple
    local storage (no OS keychain).
    """
    try:
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        data = {"openai_api_key": api_key}
        CRED_PATH.write_text(json.dumps(data))

        # Attempt to set restrictive permissions and log the resulting mode
        try:
            os.chmod(CRED_PATH, 0o600)
            mode = os.stat(CRED_PATH).st_mode & 0o777
            if os.name == "posix":
                logger.info("Set credentials file mode to %03o", mode)
            else:
                # On non-POSIX platforms (e.g. Windows) os.chmod is limited; record what was reported
                logger.info(
                    "Called os.chmod on non-posix platform; reported mode %03o", mode
                )
                logger.warning(
                    "This platform does not support POSIX permissions; file may not be restricted to the current user."
                    " Consider using an OS keyring for stronger protection."
                )
        except Exception:
            logger.debug(
                "Could not chmod credentials file (platform may not support it)"
            )

        logger.info("Saved API key to %s", CRED_PATH)
    except Exception as e:
        logger.exception("Failed to save API key: %s", e)
        raise


def load_api_key() -> Optional[str]:
    """Load the API key from local credentials file, or return None if not found."""
    try:
        if not CRED_PATH.exists():
            logger.info("No credentials file at %s", CRED_PATH)
            return None
        text = CRED_PATH.read_text()
        data = json.loads(text)
        key = data.get("openai_api_key")
        logger.info("Loaded API key from %s", CRED_PATH)
        return key
    except Exception as e:
        logger.exception("Failed to load API key: %s", e)
        return None


def delete_api_key() -> None:
    try:
        if CRED_PATH.exists():
            CRED_PATH.unlink()
            logger.info("Deleted credentials file %s", CRED_PATH)
    except Exception as e:
        logger.exception("Failed to delete API key: %s", e)
        raise
