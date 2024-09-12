#!/bin/sh

set -e
set -x

PROJECT_ROOT=${PROJECT_ROOT:-/workspaces/aici/}

# Read in the --model argument from the command line using the getopt library
MODEL="microsoft/Phi-3-mini-128k-instruct" # default model
FOLDER=""

OTHER_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -m|--model)
            MODEL=$2
            shift 2
            ;;
        -f|--folder)
            FOLDER=$2
            shift 2
            ;;
        --max-model-len)
            MAX_MODEL_LEN=$2
            shift 2
            ;;
        *)
            OTHER_ARGS="$OTHER_ARGS $1"
            shift
            ;;
    esac
done

# Construct MODEL_ARGS based on the model name
MODEL_ARGS="--model $MODEL"
MODEL_ARGS_EXTRA="--trust-remote-code"


if [ -z "$FOLDER" ]; then
    if [ "$MODEL" = "microsoft/Orca-2-13b" ]; then
        MODEL_ARGS="$MODEL_ARGS --revision refs/pr/22 --aici-tokenizer orca"
    elif [ "$MODEL" = "microsoft/Phi-3-mini-128k-instruct" ]; then
        MODEL_ARGS="$MODEL_ARGS --disable-sliding-window"
    elif [ "$MODEL" = "microsoft/Phi-3-medium-128k-instruct" ]; then
        MODEL_ARGS="$MODEL_ARGS --disable-sliding-window"
    # elif [ "$MODEL" = "microsoft/Phi-3-mini-4k-instruct" ]; then
    #     MODEL_ARGS="$MODEL_ARGS"
    fi
else
    MODEL_ARGS="$MODEL_ARGS --model ./$FOLDER --aici-tokenizer ./$FOLDER/tokenizer.json --tokenizer ./$FOLDER"
fi

MODEL_ARGS="$MODEL_ARGS $MODEL_ARGS_EXTRA"


(cd $PROJECT_ROOT/aicirt && cargo build --release)

# --enable-chunked-prefill \
# --enable-chunked-prefill \
# --enable-prefix-caching \

ENGINE_ARGS="--disable-log-requests $OTHER_ARGS"

RUST_LOG=info,tokenizers=error,aicirt=info \
RUST_BACKTRACE=1 \
PYTHONPATH=$PROJECT_ROOT/py:$PROJECT_ROOT/py/vllm \
python3 -m pyaici.vllm_server \
    --use-v2-block-manager \
    $ENGINE_ARGS \
    --served-model-name=model \
    --aici-rt $PROJECT_ROOT/target/release/aicirt \
    -A--wasm-timer-resolution-us=3 \
    -A--wasm-max-step-time=1 \
    $MODEL_ARGS \
    --port 4242 --host 127.0.0.1 \
    "$@"

#    --aici-rtarg="--wasm-max-step-time=50" \
#    --aici-rtarg="--wasm-max-pre-step-time=2" \
#    --aici-rtarg="--wasm-max-init-time=1000" \
#    --aici-rtarg="--wasm-max-memory=64" \
#    --aici-rtarg="--wasm-max-pre-step-time=10" \
