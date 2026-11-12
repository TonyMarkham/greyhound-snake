# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""The UserPromptSubmit hook. It saves the user's message for the review, and
starts a fresh list of edited files, since each message is a new task."""

import sys

from common import (
    ENCODING,
    EXIT_OK,
    PROMPT_FIELD,
    REQUEST_FILE,
    STATE_DIR,
    TOUCHED_FILE,
    read_hook_input,
    reviewer_running,
)


def main() -> int:
    if reviewer_running():
        return EXIT_OK
    hook_input = read_hook_input()
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    REQUEST_FILE.write_text(hook_input[PROMPT_FIELD], encoding=ENCODING)
    TOUCHED_FILE.unlink(missing_ok=True)
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
