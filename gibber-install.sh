#!/usr/bin/env bash
# gibber-install.sh
# Install the Gibber gibber-only file format into a Claude Code project.
#
# Usage:
#   ./gibber-install.sh                # install into the current directory
#   ./gibber-install.sh /path/to/proj  # install into the given directory
#
# Idempotent: running twice never overwrites existing content.

set -euo pipefail

GIBBER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-$(pwd)}"

if [ ! -d "$TARGET" ]; then
  echo "error: target directory does not exist: $TARGET" >&2
  exit 1
fi

PROJECT_NAME="$(basename "$TARGET")"
DICT_EXT="$TARGET/gibber-dict-$PROJECT_NAME.md"
CLAUDE_MD="$TARGET/CLAUDE.md"
GITIGNORE="$TARGET/.gitignore"

echo "Installing Gibber into: $TARGET"
echo "Project name: $PROJECT_NAME"

# 1. Project extension dictionary
if [ ! -f "$DICT_EXT" ]; then
  cat > "$DICT_EXT" <<EOF
# Gibber Project Extension Dictionary — $PROJECT_NAME

This file extends the standard meta/v1 dictionary with symbols specific to the **$PROJECT_NAME** project. Loaded alongside \`~/.claude/shared/gibber/GIBBER-DICTIONARY.md\`.

Add a symbol with a one-line English definition before using it in any task file.

## Components

(none yet)

## Verbs

(none yet)

## Concepts

(none yet)
EOF
  echo "  created $DICT_EXT"
else
  echo "  exists  $DICT_EXT (left unchanged)"
fi

# 2. CLAUDE.md — append the Gibber Protocol block if not already present
if ! grep -q "Gibber Protocol (gibber-only working files)" "$CLAUDE_MD" 2>/dev/null; then
  if [ ! -f "$CLAUDE_MD" ]; then
    cat > "$CLAUDE_MD" <<EOF
# $PROJECT_NAME

EOF
  fi
  printf '\n' >> "$CLAUDE_MD"
  cat "$GIBBER_DIR/GIBBER-CLAUDE.md" >> "$CLAUDE_MD"
  echo "  appended Gibber Protocol block to $CLAUDE_MD"
else
  echo "  CLAUDE.md already contains the Gibber Protocol block"
fi

# 3. .gitignore — make sure derivative .human files are not committed
if [ -f "$GITIGNORE" ]; then
  if ! grep -q '^\*\.human$' "$GITIGNORE" 2>/dev/null; then
    printf '\n# Gibber: derivative English views are generated on demand and never committed\n*.human\n' >> "$GITIGNORE"
    echo "  appended *.human to $GITIGNORE"
  else
    echo "  $GITIGNORE already ignores *.human"
  fi
else
  cat > "$GITIGNORE" <<'EOF'
# Gibber: derivative English views are generated on demand and never committed
*.human
EOF
  echo "  created $GITIGNORE with *.human entry"
fi

# 4. Create tasks/, learning, session, memory from .gibber templates
mkdir -p "$TARGET/tasks" "$TARGET/memory"

install_template() {
  local src="$1"
  local dst="$2"
  if [ ! -f "$dst" ]; then
    cp "$src" "$dst"
    echo "  created $dst"
  else
    echo "  exists  $dst (left unchanged)"
  fi
}

install_template "$GIBBER_DIR/templates/index.gibber"     "$TARGET/tasks/tasks_index.gibber"
install_template "$GIBBER_DIR/templates/task.gibber"      "$TARGET/tasks/current_task.gibber"
install_template "$GIBBER_DIR/templates/completed.gibber" "$TARGET/tasks/tasks_completed.gibber"
install_template "$GIBBER_DIR/templates/learning.gibber"  "$TARGET/learning.gibber"
install_template "$GIBBER_DIR/templates/session.gibber"   "$TARGET/session.gibber"
install_template "$GIBBER_DIR/templates/memory.gibber"    "$TARGET/memory/MEMORY.gibber"

echo
echo "Gibber installed in $TARGET"
echo "Next: open the project with Claude Code. The agent will read CLAUDE.md,"
echo "see the Gibber Protocol block, and write all working files in the .gibber"
echo "format from its first message. .human views are generated on demand."
