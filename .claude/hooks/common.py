"""What the three review hooks share: the review's files, the names Claude
Code uses for a hook's input, and reading that input. Each name is written
once, here."""

import json
import os
import sys
from pathlib import Path

# Every file the hooks read or write is UTF-8, whatever the system's default.
ENCODING = "utf-8"

# The environment variable Claude Code sets for every hook: the project's
# root folder, where the session started.
PROJECT_DIR_ENV = "CLAUDE_PROJECT_DIR"

# The review's files, under the project's .claude folder. The state folder
# holds the hooks' working files and is ignored by git (.gitignore).
REVIEW_DIR = Path(os.environ[PROJECT_DIR_ENV]) / ".claude" / "review"
RULES_FILE = REVIEW_DIR / "rules.md"
STATE_DIR = REVIEW_DIR / "state"
REQUEST_FILE = STATE_DIR / "request.txt"
TOUCHED_FILE = STATE_DIR / "touched.txt"

# The fields of a hook's input that these hooks read (Claude Code's hooks
# documentation): the user's message, the edited file's path, whether
# Claude is already continuing because a stop was blocked, and Claude's
# final reply.
PROMPT_FIELD = "prompt"
TOOL_INPUT_FIELD = "tool_input"
FILE_PATH_FIELD = "file_path"
STOP_HOOK_ACTIVE_FIELD = "stop_hook_active"
LAST_ASSISTANT_MESSAGE_FIELD = "last_assistant_message"

# Set while the reviewer runs, so the reviewer's own session triggers none of
# these hooks.
REVIEWER_GUARD_ENV = "REVIEW_HOOK_RUNNING"
REVIEWER_GUARD_VALUE = "1"

# What a hook's exit code tells Claude Code: 0 lets things go on, 2 blocks
# and sends what the hook wrote to stderr back to Claude, and any other code
# is a failed hook, which blocks nothing.
EXIT_OK = 0
EXIT_ERROR = 1
EXIT_BLOCK = 2


def reviewer_running() -> bool:
    """Whether this hook was started inside the reviewer's own session."""
    return os.environ.get(REVIEWER_GUARD_ENV) == REVIEWER_GUARD_VALUE


def read_hook_input() -> dict:
    """The JSON object Claude Code sends a hook on its standard input. Read as
    bytes and decoded as UTF-8, since Windows' console default is not."""
    return json.loads(sys.stdin.buffer.read().decode(ENCODING))
