"""
Example Python client for vllm.entrypoints.api_server

Usage:
    python naive_api_client.py --port 4242 --n 1 --prompt "San Francisco is a" --stream --model "model" --max_tokens 16

Note:
    --model: The value is always "model" (string literal) because the aici version of server has a bug.
"""

import argparse
import json
import warnings
import time
from typing import Iterable, List

import requests


class Timer:
    def __init__(self):
        self.start_time = None
        self.end_time = None
        self.lap_times = []

    @property
    def is_timer_started(self):
        return self.start_time is not None

    def start(self):
        self.start_time = time.time() * 1000  # Convert to milliseconds
        self.lap_times = []

    def lap(self):
        if self.start_time is None:
            raise RuntimeError("Timer not started.")
        self.lap_times.append(time.time() * 1000 - self.start_time)  # Convert to milliseconds

    def stop(self):
        if self.start_time is None:
            raise RuntimeError("Timer not started.")
        self.end_time = time.time() * 1000  # Convert to milliseconds

    def get_first_token_latency(self) -> float:
        if len(self.lap_times) < 1:
            raise RuntimeError("No laps recorded.")
        return self.lap_times[0]

    def get_per_token_latency(self) -> float:
        if len(self.lap_times) < 2:
            raise RuntimeError("Insufficient laps recorded for per-token latency.")
        total_lap_time = self.lap_times[-1] - self.lap_times[0]
        num_tokens = len(self.lap_times) - 1
        return total_lap_time / num_tokens


def clear_line(n: int = 1) -> None:
    LINE_UP = '\033[1A'
    LINE_CLEAR = '\x1b[2K'
    for _ in range(n):
        print(LINE_UP, end=LINE_CLEAR, flush=True)


def post_http_request(
    model: str,
    prompt: str,
    api_url: str,
    n: int = 1,
    stream: bool = False,
    max_tokens: int = 16,
) -> requests.Response:
    headers = {"User-Agent": "Test Client"}
    pload = {
        "model": model,
        "prompt": prompt,
        "temperature": 0.0,
        "max_tokens": max_tokens,
        "n": n,
        "use_beam_search": n > 1,
        "stream": stream,
    }

    # Perform the actual request
    response = requests.post(api_url, headers=headers, json=pload, stream=True)
    return response


def get_streaming_response(response: requests.Response, timer: Timer) -> Iterable[List[str]]:
    for chunk in response.iter_lines(
        chunk_size=8192,
        decode_unicode=False,
        delimiter=b"\n\n",
    ):
        if chunk:
            chunk_decoded = chunk.decode("utf-8")
            items = chunk_decoded.split("\n\n")
            # print(items)
            for item in items:
                # print(item)
                if not item:
                    continue
                item = item[6:]  # Remove the "data: " prefix
                if item[:len('[DONE]')] == '[DONE]':
                    timer.stop()
                    break
                try:
                    data = json.loads(item)
                    timer.lap()  # Record a lap after receiving data
                except Exception as e:
                    print(f"Error parsing JSON ({item})")
                    continue
                output = data["choices"]
                usage = data["usage"]
                yield output


def get_response(response: requests.Response, timer: Timer) -> List[str]:
    data = json.loads(response.content)
    # print(data)
    timer.stop()
    output = [data["choices"][i]['text'] for i in range(len(data["choices"]))]
    usage = data["usage"]
    print(f"{usage = }")
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", type=str, default="localhost")
    parser.add_argument("--port", type=int, default=4242)
    parser.add_argument("--n", type=int, default=4)
    parser.add_argument("--prompt", type=str, default="San Francisco is a")
    parser.add_argument("--stream", action="store_true")
    # parser.add_argument("--model", type=str, default="microsoft/Orca-2-13b")
    parser.add_argument("--model", type=str, default="model")
    parser.add_argument("--max_tokens", type=int, default=16)  # New argument
    args = parser.parse_args()

    model = args.model
    prompt = args.prompt
    api_url = f"http://{args.host}:{args.port}/v1/completions"
    print(f"API URL: {api_url!r}\n", flush=True)
    n = args.n
    stream = args.stream
    max_tokens = args.max_tokens  # Capture the max_tokens argument

    if stream and n > 1:
        warnings.warn("Streaming is not supported for n > 1. Setting n = 1.")
        n = 1

    print(f"Prompt: {prompt!r}\n", flush=True)
    timer = Timer()
    timer.start()
    response = post_http_request(model, prompt, api_url, n, stream, max_tokens)

    if stream:
        num_printed_lines = 0
        for h in get_streaming_response(response, timer):
            # clear_line(num_printed_lines)
            num_printed_lines = 0
            for i, line in enumerate(h):
                num_printed_lines += 1
                print(f"Beam candidate {i}: {line!r}", flush=True)
                # print(f"Beam candidate {i}: {line!r}")
        if timer.is_timer_started:
            print(f"First token latency: {timer.get_first_token_latency()} ms")
            print(f"Per-token latency: {timer.get_per_token_latency()} ms")
    else:
        output = get_response(response, timer)
        for i, line in enumerate(output):
            print(f"Beam candidate {i}: {line!r}", flush=True)
        if timer.is_timer_started:
            print(f"First token latency: {timer.get_first_token_latency()} ms")
            print(f"Per-token latency: {timer.get_per_token_latency()} ms")