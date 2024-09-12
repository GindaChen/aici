#!/bin/bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# project root is specified in env variable or /workspaces/aici or ~/aici, if any exists
PROJECT_ROOT_DIR="${PROJECT_ROOT_DIR:-/workspaces/aici}"

if [ ! -d "$PROJECT_ROOT_DIR" ]; then
    echo "Project root directory not found, using default: $PROJECT_ROOT_DIR"
    PROJECT_ROOT_DIR="/workspaces/aici"
fi

if [ ! -d "$PROJECT_ROOT_DIR" ]; then
    echo "Project root directory not found, using default: $PROJECT_ROOT_DIR"
    PROJECT_ROOT_DIR="$HOME/aici"
fi

if [ ! -d "$PROJECT_ROOT_DIR" ]; then
    echo "Project root directory not found!"
    exit 1  
fi

# Default values
PROMPT_LENGTH=1000
NUM_FAST_FORWARD_TOKENS=128
NUM_ITERATIONS=10
NUM_GENERATE_TOKENS=8

# Parse command line arguments
VLLM_ARGS=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --prompt_length)
            PROMPT_LENGTH="$2"
            shift 2
            ;;
        --num_fast_forward_tokens)
            NUM_FAST_FORWARD_TOKENS="$2"
            shift 2
            ;;
        --num_iterations)
            NUM_ITERATIONS="$2"
            shift 2
            ;;
        --num_generate_tokens)
            NUM_GENERATE_TOKENS="$2"
            shift 2
            ;;
        --vllm_args*)
            VLLM_ARGS="$VLLM_ARGS $2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Update rag_simple.py with new values
CONTROLLER_PATH="$SCRIPT_DIR/controllers/rag_simple.py"

echo "Running experiment with parameters:"
echo " - PROMPT_LENGTH: $PROMPT_LENGTH"
echo " - NUM_FAST_FORWARD_TOKENS: $NUM_FAST_FORWARD_TOKENS"
echo " - NUM_ITERATIONS: $NUM_ITERATIONS"
echo " - NUM_GENERATE_TOKENS: $NUM_GENERATE_TOKENS"

sed -i "s/^num_prompt_tokens = .*/num_prompt_tokens = $PROMPT_LENGTH/" $CONTROLLER_PATH
sed -i "s/^num_fast_forward_tokens = .*/num_fast_forward_tokens = $NUM_FAST_FORWARD_TOKENS/" $CONTROLLER_PATH
sed -i "s/^num_generate_tokens = .*/num_generate_tokens = $NUM_GENERATE_TOKENS/" $CONTROLLER_PATH
sed -i "s/^num_iterations = .*/num_iterations = $NUM_ITERATIONS/" $CONTROLLER_PATH

# Run vllm client
echo ""
echo "Running vllm client..."
python $SCRIPT_DIR/run_vllm_client.py --stream --rag --prompt_length $PROMPT_LENGTH --num_fast_forward_tokens $NUM_FAST_FORWARD_TOKENS --num_iterations $NUM_ITERATIONS --num_generate_tokens $NUM_GENERATE_TOKENS $VLLM_ARGS

# Run aici script
echo ""
echo "Running aici script..."
sh $PROJECT_ROOT_DIR/scripts/aici.sh run $CONTROLLER_PATH | tail -n 4

echo "Benchmark completed."





# -------------------------------------------------------------------------------------------------
# Result
# -------------------------------------------------------------------------------------------------
: <<'RESULT_SECTION'

# Results are without using automatic prefix caching



06:34:51 root@f4251c9801fa exp1_regex ±|feat-add-session-interface--gindachen ✗|→ python run_suite_rag_benchmark.py --vllm_args "--sleep_time 0.1"
Running 3 experiments
Experiment parameters:

prompt_length,num_fast_forward_tokens,num_iterations,num_generate_tokens
2048,1024,4,32
1954,1132,10,46
1954,996,10,46

----------------------------------------
Running experiment with parameters: {'prompt_length': '2048', 'num_fast_forward_tokens': '1024', 'num_iterations': '4', 'num_generate_tokens': '32'}
Running experiment with parameters:
 - PROMPT_LENGTH: 2048
 - NUM_FAST_FORWARD_TOKENS: 1024
 - NUM_ITERATIONS: 4
 - NUM_GENERATE_TOKENS: 32

Running vllm client...
First token latency: 206.42138671875 ms
Per-token latency: 36.408458291330646 ms
Token Per Second: 27.46614514677524 tok / s
Total time: 5849.77001953125 ms

Running aici script...
Usage: {'sampled_tokens': 142, 'ff_tokens': 5258, 'cost': 5542}
Timing: {'http_response': 0.3205382823944092, 'data0': 0.32058048248291016, 'first_token': 0.3543400764465332, 'last_token': 4.467339992523193}
Tokens/sec: {'prompt': 60588.500031740434, 'sampling': 31.786253170266797}
Storage: {}
Benchmark completed.
----------------------------------------
----------------------------------------
Running experiment with parameters: {'prompt_length': '1954', 'num_fast_forward_tokens': '1132', 'num_iterations': '10', 'num_generate_tokens': '46'}
Running experiment with parameters:
 - PROMPT_LENGTH: 1954
 - NUM_FAST_FORWARD_TOKENS: 1132
 - NUM_ITERATIONS: 10
 - NUM_GENERATE_TOKENS: 46

Running vllm client...
First token latency: 205.896728515625 ms
Per-token latency: 42.4988244321166 ms
Token Per Second: 23.530062616138963 tok / s
Total time: 25492.734375 ms

Running aici script...
Usage: {'sampled_tokens': 501, 'ff_tokens': 12633, 'cost': 13635}
Timing: {'http_response': 0.0741584300994873, 'data0': 0.07422423362731934, 'first_token': 0.10749959945678711, 'last_token': 14.74777102470398}
Tokens/sec: {'prompt': 58606.222807005, 'sampling': 33.97123532503829}
Storage: {}
Benchmark completed.
----------------------------------------
----------------------------------------
Running experiment with parameters: {'prompt_length': '1954', 'num_fast_forward_tokens': '996', 'num_iterations': '10', 'num_generate_tokens': '46'}
Running experiment with parameters:
 - PROMPT_LENGTH: 1954
 - NUM_FAST_FORWARD_TOKENS: 996
 - NUM_ITERATIONS: 10
 - NUM_GENERATE_TOKENS: 46

Running vllm client...
First token latency: 205.635986328125 ms
Per-token latency: 41.44902965657272 ms
Token Per Second: 24.126017141668513 tok / s
Total time: 24287.5439453125 ms

Running aici script...
Usage: {'sampled_tokens': 492, 'ff_tokens': 11400, 'cost': 12384}
Timing: {'http_response': 0.07442474365234375, 'data0': 0.07449030876159668, 'first_token': 0.10919308662414551, 'last_token': 14.419909000396729}
Tokens/sec: {'prompt': 56200.55006891633, 'sampling': 34.11949409573}
Storage: {}
Benchmark completed.
----------------------------------------

RESULT_SECTION
