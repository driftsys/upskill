#!/bin/sh
# Disposable SessionStart fixture; writes only its declared private trace.
set -eu
root=${CLAUDE_PLUGIN_ROOT:?}
payload=$(cat "$root/rules/index")
case "$payload" in
    rule-a.json) ;;
    *) exit 2 ;;
esac
trace=$(cat "$root/trace-path")
input=$(cat)
printf '\036%s\n' "$input" >> "$trace"
cat "$root/rules/$payload"
