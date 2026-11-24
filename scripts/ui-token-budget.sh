#!/bin/bash
# Ratchet for hardcoded style literals in the interface.
#
# Counts direct Color constructors and FontSize literals in
# ro-next/src/interface and compares them with the committed budget in
# ro-next/test-data/ui-token-budget.ron. The budget may only go down; a
# failure means a new hardcoded value was added instead of using a design token
# (`client_theme().tokens()...`).
#
# Usage:
#     scripts/ui-token-budget.sh            # check (exit 1 when the budget is exceeded)
#     scripts/ui-token-budget.sh --update   # lower the budget to the current counts
#     scripts/ui-token-budget.sh --report   # print the counts and the worst files
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INTERFACE="$REPO_ROOT/ro-next/src/interface"
BUDGET="$REPO_ROOT/ro-next/test-data/ui-token-budget.ron"

MODE="check"
case "${1:-}" in
  --update) MODE="update";;
  --report) MODE="report";;
  -h|--help) sed -n '2,18p' "${BASH_SOURCE[0]}"; exit 0;;
  "") ;;
  *) echo "unknown argument: $1" >&2; exit 2;;
esac

# `grep` exits 1 when it finds no literal at all; that is a valid state for a
# fully migrated interface, so both counts are allowed to be zero.
color_literals=$(grep -rE 'Color::(rgb|rgba|monochrome)_u8' --include='*.rs' "$INTERFACE" | wc -l | tr -d ' ' || true)
font_literals=$(grep -rE 'FontSize\(' --include='*.rs' "$INTERFACE" | wc -l | tr -d ' ' || true)

budget_colors=$(sed -n 's/.*max_color_literals: *\([0-9]*\).*/\1/p' "$BUDGET")
budget_fonts=$(sed -n 's/.*max_font_size_literals: *\([0-9]*\).*/\1/p' "$BUDGET")

echo "interface style literals: colors=$color_literals (budget $budget_colors), FontSize=$font_literals (budget $budget_fonts)"

if [ "$MODE" = "report" ]; then
  echo
  echo "most color literals:"
  grep -rEc 'Color::(rgb|rgba|monochrome)_u8' --include='*.rs' "$INTERFACE" | grep -v ':0$' | sort -t: -k2 -rn | head -10
  echo
  echo "most FontSize sites:"
  grep -rEc 'FontSize\(' --include='*.rs' "$INTERFACE" | grep -v ':0$' | sort -t: -k2 -rn | head -10
  exit 0
fi

if [ "$MODE" = "update" ]; then
  if [ "$color_literals" -gt "$budget_colors" ] || [ "$font_literals" -gt "$budget_fonts" ]; then
    echo "error: refusing to raise the token budget; migrate the new literals to design tokens first" >&2
    exit 1
  fi
  sed -i "s/max_color_literals: *[0-9]*/max_color_literals: $color_literals/" "$BUDGET"
  sed -i "s/max_font_size_literals: *[0-9]*/max_font_size_literals: $font_literals/" "$BUDGET"
  echo "token budget lowered to colors=$color_literals, FontSize=$font_literals"
  exit 0
fi

status=0
[ "$color_literals" -le "$budget_colors" ] || { echo "error: color literal budget exceeded ($color_literals > $budget_colors)" >&2; status=1; }
[ "$font_literals" -le "$budget_fonts" ] || { echo "error: FontSize budget exceeded ($font_literals > $budget_fonts)" >&2; status=1; }
if [ "$status" -ne 0 ]; then
  echo "hint: use the design tokens in ro-next/src/state/theme/tokens.rs instead of hardcoded values" >&2
fi
exit "$status"
