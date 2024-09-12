#!/bin/sh

set -e
set -x

if [ -z "$FOLDER" ]; then
    # MODEL_ARGS="--model microsoft/Orca-2-13b --revision refs/pr/22 --aici-tokenizer orca --trust-remote-code"
    MODEL_ARGS="--model microsoft/Phi-3-mini-128k-instruct --trust-remote-code --disable-sliding-window"
    # MODEL_ARGS="--model microsoft/Phi-3-medium-128k-instruct --trust-remote-code"
    # MODEL_ARGS="--model microsoft/Phi-3-mini-4k-instruct --trust-remote-code"
else
    MODEL_ARGS="--model ./$FOLDER --aici-tokenizer ./$FOLDER/tokenizer.json --tokenizer ./$FOLDER"
fi

(cd aicirt && cargo build --release)

# --enable-chunked-prefill \
# --enable-chunked-prefill \
# --enable-prefix-caching \
    
RUST_LOG=info,tokenizers=error,aicirt=info \
RUST_BACKTRACE=1 \
PYTHONPATH=py:py/vllm \
python3 -m pyaici.vllm_server \
    --enforce-eager \
    --use-v2-block-manager \
    --served-model-name=model \
    --aici-rt ./target/release/aicirt \
    -A--wasm-timer-resolution-us=10 \
    $MODEL_ARGS \
    --port 4242 --host 127.0.0.1 \
    "$@"

#    --aici-rtarg="--wasm-max-step-time=50" \
#    --aici-rtarg="--wasm-max-pre-step-time=2" \
#    --aici-rtarg="--wasm-max-init-time=1000" \
#    --aici-rtarg="--wasm-max-memory=64" \
#    --aici-rtarg="--wasm-max-pre-step-time=10" \
