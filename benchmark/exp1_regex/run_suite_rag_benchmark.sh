#!/bin/bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Default values
ARR_PROMPT_LENGTH=(1000 2000 4000 8000 16000)
ARR_NUM_FAST_FORWARD_TOKENS=(128 256 512 1024 2048)
ARR_NUM_ITERATIONS=(10 10 10 10 10)
ARR_NUM_GENERATE_TOKENS=(8 16 32 64 128)

# assert the length of the arrays are the same
if [ ${#ARR_PROMPT_LENGTH[@]} -ne ${#ARR_NUM_FAST_FORWARD_TOKENS[@]} ] || [ ${#ARR_PROMPT_LENGTH[@]} -ne ${#ARR_NUM_ITERATIONS[@]} ] || [ ${#ARR_PROMPT_LENGTH[@]} -ne ${#ARR_NUM_GENERATE_TOKENS[@]} ]; then
    echo "All arrays must have the same length"
    exit 1
fi  

for i in "${!ARR_PROMPT_LENGTH[@]}"; do
    PROMPT_LENGTH=${ARR_PROMPT_LENGTH[$i]}
    NUM_FAST_FORWARD_TOKENS=${ARR_NUM_FAST_FORWARD_TOKENS[$i]}
    NUM_ITERATIONS=${ARR_NUM_ITERATIONS[$i]}
    NUM_GENERATE_TOKENS=${ARR_NUM_GENERATE_TOKENS[$i]}
    
    echo "----------------------------------------"
    echo "Running experiment with parameters: --prompt_length $PROMPT_LENGTH --num_fast_forward_tokens $NUM_FAST_FORWARD_TOKENS --num_iterations $NUM_ITERATIONS --num_generate_tokens $NUM_GENERATE_TOKENS"
    bash $SCRIPT_DIR/run_rag_benchmark.sh --prompt_length $PROMPT_LENGTH --num_fast_forward_tokens $NUM_FAST_FORWARD_TOKENS --num_iterations $NUM_ITERATIONS --num_generate_tokens $NUM_GENERATE_TOKENS
    echo "----------------------------------------"
    
done