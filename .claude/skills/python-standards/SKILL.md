---
name: python-standards
description: Enforces strict typing, modern standard library usage, and clean architecture when writing or refactoring Python code.
---

# Python Coding Standards

## Environment & Packages
- Always use `uv`: `uv run` to run code and tools, `uv add` / `uv remove` to change dependencies, `uv sync` to install them.
- Run `uv sync` from the project's folder (the one holding `pyproject.toml`) when setting it up, and again whenever `pyproject.toml` or `uv.lock` changes, for example after a pull or a branch switch. It makes `.venv` match `uv.lock` exactly, installing what is missing and removing what is not listed. `uv add`, `uv remove` and `uv run` sync on their own.
- Every project runs in its own virtual environment, the `.venv` that `uv sync` creates beside its `pyproject.toml`. `uv run` uses it without activating it. Never create one another way (`python -m venv`, `virtualenv`), and never install into the system or global Python.
- Never use `pip` (including `uv pip` and `python -m pip`) without the user's explicit permission, asked for and given in chat before each use.

## Code Philosophy & Structure
- Source code goes in a `src/` directory beside the project's `pyproject.toml`, never at the project root.
- Match existing project naming, importing, and signature patterns.
- Write small, single-purpose pure functions (ideally <50 lines). Do not create trivial wrappers for 2 lines or less.
- Enforce keyword-only arguments (`*`) for functions with 4 or more parameters.
- Declare variables close to their usage site and inline single-use variables.
- Avoid mutable or risky default parameter values unless universally required.

## Typing & Validation
- Include complete PEP 484 type annotations for all function signatures and class attributes.
- Model fixed sets of strings or codes using `typing.Literal` or `Enum`.
- Use `pydantic` models for external data parsing and input validation.
- Verify types at runtime with `isinstance()` before using `typing.cast()`.

## Error Handling & Filesystem
- Prefer Look Before You Leap (LBYL) checks over broad `try/except` blocks.
- Never swallow exceptions silently. Let unexpected errors bubble up or log them explicitly.
- Always use `pathlib.Path` instead of `os.path`. Check `path.exists()` before calling `path.resolve()`.

## Verification
- Ensure code passes `uv run ruff check`, `uv run ruff format`, and `uv run mypy --strict` before finishing.
