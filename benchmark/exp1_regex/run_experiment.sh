batch_size=4
# vllm with sleep 0.02
set -x
for per_token_control in 1 1 2 2 4 4 8 8 16 16 32 32 64 64 128 128; do
	for i in $(seq 1 $batch_size); do
		python run_vllm_client.py --stream --max_tokens 128 --prompt_length 100 --model "microsoft/Phi-3-medium-128k-instruct" --per_token_control $per_token_control  --sleep_time 0.02 &
	done
	wait
done
set +x


# vllm with sleep 0.05
set -x
for per_token_control in 1 1 2 2 4 4 8 8 16 16 32 32 64 64 128 128; do
	for i in $(seq 1 $batch_size); do
		python run_vllm_client.py --stream --max_tokens 128 --prompt_length 100 --model "microsoft/Phi-3-medium-128k-instruct" --per_token_control $per_token_control  --sleep_time 0.05 &
	done
	wait
done
set +x

# aici
set -x
for per_token_control in 1 1 2 2 4 4 8 8 16 16 32 32 64 64 128 128; do
	for i in $(seq 1 $batch_size); do
		sh scripts/aici.sh run ./benchmark/exp1_regex/controllers/controller_simple_$per_token_control.py | tail -n 5 &
	done
	wait
	rm tmp/response_$n.json || true
	mv tmp/response.json tmp/response_$n.json
done
set +x