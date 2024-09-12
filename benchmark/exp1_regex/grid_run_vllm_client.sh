#!/bin/bash


PROJECT_ROOT=${PROJECT_ROOT:-/workspaces/aici/}
SERVER_LOG="server.log"
# mini 3b, small 7b, orca 13b
models=("microsoft/Phi-3-mini-128k-instruct" "microsoft/Phi-3-small-128k-instruct" "microsoft/Orca-2-13b")
prompt_lengths=(100 1000 10000 100000)
per_token_controls=(1 2 4 16)


echo "Running grid search for prompt_length and per_token_control"
echo "- Prompt length: ${prompt_lengths[@]}"
echo "- Per token control: ${per_token_controls[@]}"

gpu_type=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -n 1)

# Loop through all combinations
for model in "${models[@]}"; do
    echo "Running for model: $model"
    # Spin up the server
    sh $PROJECT_ROOT/benchmark/common/vllm-server.sh -m $model >> $SERVER_LOG 2>&1 &
    SERVER_PID=$!

    echo "Server started with PID: $SERVER_PID"
    echo "Server log: $SERVER_LOG"

    # Check health
    while ! curl -s http://127.0.0.1:4242/v1/health > /dev/null; do
        echo "Waiting for server to start..."
        sleep 3
    done

    # Run the client
    for prompt_length in "${prompt_lengths[@]}"; do
        for per_token_control in "${per_token_controls[@]}"; do
            set -x            
            python run_vllm_client.py --stream --max_tokens 128 --prompt_length $prompt_length --per_token_control $per_token_control --model $model --wandb --wandb_project "aici-token-control-no-prefix-caching" --wandb_comment "gpu_type=$gpu_type"
            set +x
        done
    done

    # Kill the server
    kill $SERVER_PID
    echo "Server killed."
    sleep 1
done

# # Kill all python processes related to vllm_server
# pkill -f 'python3 -m pyaici.vllm_server'
echo "Grid search completed."
