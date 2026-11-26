#!/bin/sh
# Session-start hook of the upskill plugin. Prints one JSON line for the
# client: the install message when the upskill CLI is missing (at most once
# per day), else the output of `upskill update --notice`. Always exits 0.
# UPSKILL_CLIENT is set by `upskill pack`: claude or copilot.

case "${UPSKILL_CLIENT:-}" in
claude | copilot) ;;
*) exit 0 ;;
esac

if [ -n "${COPILOT_PROJECT_DIR:-}" ]; then
  cd -- "$COPILOT_PROJECT_DIR" >/dev/null 2>&1 || exit 0
fi

# emit TEXT: print TEXT as the JSON line of the client.
emit() {
  t=$(printf '%s\n' "$1" | tr '\001-\011\013-\037' ' ' | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | awk 'NR>1{printf "\\n"} {printf "%s",$0}')
  if [ "$UPSKILL_CLIENT" = claude ]; then
    printf '{"systemMessage":"%s","hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"%s"}}\n' "$t" "$t"
  else
    printf '{"additionalContext":"Tell the user at the start of your first reply: %s"}\n' "$t"
  fi
}

if ! command -v upskill >/dev/null 2>&1; then
  cache="${XDG_CACHE_HOME:-}"
  case "$cache" in
  /*) ;;
  *) cache="${HOME:-/nonexistent}/.cache" ;;
  esac
  marker="$cache/upskill/setup-offered"
  today=$(date +%Y-%m-%d)
  if [ "$(cat "$marker" 2>/dev/null)" = "$today" ]; then
    exit 0
  fi
  emit "The upskill CLI is not installed. Install it with:
curl -fsSL https://raw.githubusercontent.com/driftsys/upskill/main/install.sh | sh"
  { mkdir -p "$cache/upskill" && printf '%s\n' "$today" >"$marker"; } >/dev/null 2>&1
  exit 0
fi

if [ "$UPSKILL_CLIENT" = copilot ]; then
  n=$(upskill update --notice --repeat 2>/dev/null) || exit 0
else
  n=$(upskill update --notice 2>/dev/null) || exit 0
fi
[ -n "$n" ] || exit 0
emit "$n"
exit 0
