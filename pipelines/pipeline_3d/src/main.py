import logging
import sys

import uvicorn

from .config import settings

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)


def main():
    uvicorn.run(
        "src.api.routes:app",
        host="0.0.0.0",
        port=8090,
        reload=False,
        log_level="info",
    )


if __name__ == "__main__":
    main()
