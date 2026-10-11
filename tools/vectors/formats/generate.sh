set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
work="${TMPDIR:-/tmp}/randomon-formats-vectors"
output="$repo/crates/randomon_formats/tests/vectors"

rm -rf "$work"
mkdir -p "$work/classes" "$work/resources" "$work/testpatches" "$work/run"

find "${RANDOMON_UPSTREAM:-$repo/target/upstream}/src" -name '*.java' > "$work/sources.txt"
find "$here/src" -name '*.java' >> "$work/sources.txt"
javac -nowarn -encoding UTF-8 -d "$work/classes" @"$work/sources.txt" > "$work/javac.log" 2>&1 || {
    cat "$work/javac.log"
    exit 1
}

mkdir -p "$work/resources/com/dabomstew/pkrandom"
cp -R "${RANDOMON_UPSTREAM:-$repo/target/upstream}/src/com/dabomstew/pkrandom/patches" "$work/resources/com/dabomstew/pkrandom/"
cp -R "${RANDOMON_UPSTREAM:-$repo/target/upstream}/src/com/dabomstew/pkrandom/config" "$work/resources/com/dabomstew/pkrandom/"

java -Djava.security.manager=allow -Djava.awt.headless=true \
    -cp "$work/classes:$work/resources:$work/testpatches" \
    FormatsVectorGenerator "$output" "$work/run" "$work/testpatches" "${RANDOMON_UPSTREAM:-$repo/target/upstream}/src/com/dabomstew/pkrandom/patches"
