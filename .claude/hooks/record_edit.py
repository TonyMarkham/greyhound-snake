# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""The PostToolUse hook for Write and Edit. It adds the file just written or
edited to the list the review reads, one path per line."""

import sys

from common import (
    ENCODING,
    EXIT_OK,
    FILE_PATH_FIELD,
    STATE_DIR,
    TOOL_INPUT_FIELD,
    TOUCHED_FILE,
    read_hook_input,
    reviewer_running,
)


def main() -> int:
    if reviewer_running():
        return EXIT_OK
    hook_input = read_hook_input()
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    with TOUCHED_FILE.open("a", encoding=ENCODING) as touched:
        touched.write(hook_input[TOOL_INPUT_FIELD][FILE_PATH_FIELD] + "\n")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
