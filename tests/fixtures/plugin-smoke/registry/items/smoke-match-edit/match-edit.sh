#!/bin/sh
# Appends the client name and the time to a matcher marker file. Prints nothing.
dir="${TMPDIR:-/tmp}"
printf '%s %s\n' "${UPSKILL_CLIENT:-unknown}" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "${dir%/}/upskill-smoke-match-edit"
cat > /dev/null
exit 0
