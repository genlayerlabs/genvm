mkdir -p "$TMPDIR"
FILE_1=$(mktemp)
ninja "$@" > "$FILE_1" 2>&1
STATUS=$?
if [ $STATUS -ne 0 ]; then
    ninja "$@" 2>&1
    STATUS=$?
else
    cat "$FILE_1"
fi
rm "$FILE_1"
exit $STATUS
