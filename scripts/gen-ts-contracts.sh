#!/usr/bin/env bash
# Generate TypeScript bindings from the Protobuf contracts in `contracts/`.
# Output: web/lib/contracts/ and mobile/lib/contracts/.
#
# Requirements:
#   - protoc (system install)
#   - ts-proto installed in both web/ and mobile/ as a dev dependency
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
CONTRACTS="$ROOT/contracts"
PROTOS=$(find "$CONTRACTS" -name '*.proto' -type f)

for target in web mobile; do
    OUT="$ROOT/$target/lib/contracts"
    PLUGIN="$ROOT/$target/node_modules/.bin/protoc-gen-ts_proto"
    if [ ! -x "$PLUGIN" ]; then
        echo "skip $target: ts-proto not installed (npm install --save-dev ts-proto)"
        continue
    fi
    mkdir -p "$OUT"
    echo "-> generating TS bindings into $OUT"

    # shellcheck disable=SC2086
    protoc \
        --plugin="protoc-gen-ts_proto=$PLUGIN" \
        --ts_proto_out="$OUT" \
        --ts_proto_opt=esModuleInterop=true \
        --ts_proto_opt=outputServices=false \
        --ts_proto_opt=useOptionals=messages \
        --ts_proto_opt=stringEnums=true \
        --ts_proto_opt=useDate=string \
        --proto_path="$CONTRACTS" \
        $PROTOS
done

echo "done"
