# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""The Stop hook. When Claude thinks it is done, a fresh reviewer checks its
work against the rules and the user's request, in one pass: every file it
touched, and its final reply when the reply contains code. A failed review
blocks finishing, once, with all the findings together, and they go back to
Claude to fix."""

import os
import subprocess
import sys
from pathlib import Path

from common import (
    ENCODING,
    EXIT_BLOCK,
    EXIT_ERROR,
    EXIT_OK,
    LAST_ASSISTANT_MESSAGE_FIELD,
    REQUEST_FILE,
    REVIEWER_GUARD_ENV,
    REVIEWER_GUARD_VALUE,
    RULES_FILE,
    STOP_HOOK_ACTIVE_FIELD,
    TOUCHED_FILE,
    read_hook_input,
    reviewer_running,
)

# The Claude Code command the reviewer runs, the flags that make it answer
# once and exit (-p) on the given model, and the model.
CLAUDE_COMMAND = "claude"
CLAUDE_PRINT_FLAG = "-p"
CLAUDE_MODEL_FLAG = "--model"
REVIEWER_MODEL = "sonnet"

# The reviewer's first line when it finds nothing (rules.md, its last
# paragraph).
PASS_VERDICT = "PASS"

# A reply contains code when it has a Markdown code fence.
CODE_FENCE = "```"

# What the reviewer is given: the rules, the user's request, each file, and
# the reply.
PROMPT_TEMPLATE = """{rules}

## The user's request

{request}

## The files written or edited

{files}

## The final reply

{reply}
"""

# Each file in the prompt: its path, then its lines, each with its number.
FILE_SECTION_TEMPLATE = "=== {path} ===\n{lines}"
NUMBERED_LINE_TEMPLATE = "{number:5}  {line}"

# What stands in a section that has nothing to review.
REQUEST_MISSING = "(The user's request was not recorded.)"
NO_FILES = "(No files were written or edited.)"
NO_REPLY_CODE = "(The reply contains no code.)"

# What the hook says when CLAUDE_COMMAND can't be started, before the command.
CLAUDE_NOT_FOUND_MESSAGE = "The review did not run: no program found at"

# What Claude is told when the review fails, before the findings.
BLOCKED_MESSAGE = (
    "The review found problems. Fix each one, in the files and in your "
    "reply, then give your corrected reply:"
)


def file_section(path: Path) -> str:
    """One file for the prompt, numbered, so a finding can point at a line."""
    lines = path.read_text(encoding=ENCODING).splitlines()
    numbered = [
        NUMBERED_LINE_TEMPLATE.format(number=number, line=line)
        for number, line in enumerate(lines, start=1)
    ]
    return FILE_SECTION_TEMPLATE.format(path=path, lines="\n".join(numbered))


def touched_files() -> list[Path]:
    """The files written or edited this turn that still exist, each once."""
    if not TOUCHED_FILE.exists():
        return []
    touched = TOUCHED_FILE.read_text(encoding=ENCODING).splitlines()
    return [path for path in sorted({Path(line) for line in touched if line}) if path.is_file()]


def main() -> int:
    if reviewer_running():
        return EXIT_OK
    hook_input = read_hook_input()
    # One fix round per turn: the stop after a blocked one is let through.
    if hook_input.get(STOP_HOOK_ACTIVE_FIELD):
        return EXIT_OK

    files = touched_files()
    reply = hook_input.get(LAST_ASSISTANT_MESSAGE_FIELD) or ""
    reply_has_code = CODE_FENCE in reply
    if not files and not reply_has_code:
        return EXIT_OK

    request = (
        REQUEST_FILE.read_text(encoding=ENCODING)
        if REQUEST_FILE.exists()
        else REQUEST_MISSING
    )
    prompt = PROMPT_TEMPLATE.format(
        rules=RULES_FILE.read_text(encoding=ENCODING),
        request=request,
        files="\n\n".join(file_section(path) for path in files) if files else NO_FILES,
        reply=reply if reply_has_code else NO_REPLY_CODE,
    )

    sys.stderr.reconfigure(encoding=ENCODING)
    try:
        result = subprocess.run(
            [CLAUDE_COMMAND, CLAUDE_PRINT_FLAG, CLAUDE_MODEL_FLAG, REVIEWER_MODEL],
            input=prompt,
            capture_output=True,
            text=True,
            encoding=ENCODING,
            env={**os.environ, REVIEWER_GUARD_ENV: REVIEWER_GUARD_VALUE},
        )
    except FileNotFoundError:
        sys.stderr.write(f"{CLAUDE_NOT_FOUND_MESSAGE} {CLAUDE_COMMAND}\n")
        return EXIT_ERROR
    # The reviewer itself failed: say why, and block nothing.
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        return EXIT_ERROR

    verdict = result.stdout.strip()
    first_line = verdict.splitlines()[0].strip() if verdict else ""
    if first_line == PASS_VERDICT:
        TOUCHED_FILE.unlink(missing_ok=True)
        return EXIT_OK
    sys.stderr.write(f"{BLOCKED_MESSAGE}\n{verdict}\n")
    return EXIT_BLOCK


if __name__ == "__main__":
    sys.exit(main())
