#!/bin/bash
S=/private/tmp/claude-501/-Users-luke-Code-crumbkit/3ca13248-6bce-4d4c-910a-e070854b67a0/scratchpad/rereview
for d in ex-old ex-new ex-clap; do
  cd $S/$d
  cargo generate-lockfile -q --offline 2>/dev/null
  echo "== $d tree-unique: $(cargo tree -e normal --prefix none --offline 2>/dev/null | sort -u | wc -l | tr -d ' ')"
  rm -rf $S/target/$d
  start=$(date +%s.%N)
  CARGO_TARGET_DIR=$S/target/$d cargo build --release --offline 2> $S/$d.build.log
  end=$(date +%s.%N)
  echo "== $d compiling-lines: $(grep -c Compiling $S/$d.build.log)  time: $(echo "$end - $start" | bc)s"
  bin=$(ls $S/target/$d/release/ | grep -E '^books' | head -1)
  echo "== $d binary: $(stat -f %z $S/target/$d/release/$bin) bytes"
  echo "== $d warm rebuild after touch:"
  touch src/main.rs; start=$(date +%s.%N); CARGO_TARGET_DIR=$S/target/$d cargo build --release --offline 2>/dev/null; end=$(date +%s.%N); echo "   $(echo "$end - $start" | bc)s"
done
cd $S/new/crates/crumbkit
echo "== new crumbkit all-features tree-unique: $(cargo tree -e normal --prefix none --all-features --offline 2>/dev/null | sort -u | wc -l | tr -d ' ')"
echo "== new crumbkit default tree-unique: $(cargo tree -e normal --prefix none --offline 2>/dev/null | sort -u | wc -l | tr -d ' ')"
cd $S/old/crates/crumbkit
echo "== old crumbkit all-features tree-unique: $(cargo tree -e normal --prefix none --all-features --offline 2>/dev/null | sort -u | wc -l | tr -d ' ')"
echo "== old crumbkit default tree-unique: $(cargo tree -e normal --prefix none --offline 2>/dev/null | sort -u | wc -l | tr -d ' ')"
echo DONE
