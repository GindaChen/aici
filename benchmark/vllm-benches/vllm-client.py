"""
Per-token control using vLLM to test the latency.

Usage:
    python run_vllm_client.py --stream --per_token_control 1

"""
import wandb
from typing import Iterable, List
import warnings
import json 
import pathlib
import os   
import subprocess
import asyncio
import requests
import time
import argparse
import logging


def get_project_root():
    import pyaici
    pyaici_dir = os.path.dirname(os.path.abspath(pyaici.__file__))
    pyaici_dir = pathlib.Path(pyaici_dir)
    return pyaici_dir.parent.parent
    

def run_vllm_server():
    project_root = get_project_root()
    benchmark_common_dir = project_root / "benchmark" / "common"
    proc = subprocess.Popen(["sh", str(benchmark_common_dir / "vllm-server.sh")])
    return proc





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


def wait_proc_healthy(host='localhost', port=4242, n=10, timeout = 5):
    logger = logging.getLogger(__name__)
    for i in range(n):
        try:
            response = requests.get(f'http://{host}:{port}/health')
            if response.status_code == 200:
                logger.info("Server is healthy")
                return True
        except requests.exceptions.ConnectionError:
            logger.debug("Server is not healthy yet")
        time.sleep(timeout)  # Use the timeout variable
    raise Exception("Server is not healthy after n retries")



def post_http_request(model: str, prompt: str, api_url: str, n: int = 1, stream: bool = False, max_tokens: int = 16, regex: str = None, sleep_time: float = 0):
    """
    Post a request to the vllm server
    """
    headers = {"User-Agent": "Test Client"}
    pload = {
        # The `model` is fixed if we use the vllm model by aici.
        "model": model,
        "prompt": prompt,
        "temperature": 0.0,
        "max_tokens": max_tokens,
        "n": n,
        "use_beam_search": n > 1,
        "stream": stream,
    }
    if regex:
        pload["regex"] = regex
    
    # Simulate the (extra) network latency to the server
    time.sleep(sleep_time)
    response = requests.post(api_url, headers=headers, json=pload, stream=True)
    # Simulate the (extra) network latency from the server to the client, for the 1st token
    time.sleep(sleep_time)
    return response


def get_streaming_response(response: requests.Response, timer: Timer) -> Iterable[List[str]]:
    logger = logging.getLogger(__name__)
    for chunk in response.iter_lines(
        chunk_size=8192,
        decode_unicode=False,
        delimiter=b"\n\n",
    ):
        if chunk:
            chunk_decoded = chunk.decode("utf-8")
            items = chunk_decoded.split("\n\n")
            logger.debug_detail(f"Received items: {items}")
            for item in items:
                logger.debug_detail(f"Processing item: {item}")
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
                    logger.error(f"Error parsing JSON ({item}): {e}")
                    continue
                output = data["choices"]
                usage = data["usage"]
                logger.debug_detail(f"Usage: {usage}")
                yield output


def get_response(response: requests.Response, timer: Timer) -> List[str]:
    logger = logging.getLogger(__name__)
    data = json.loads(response.content)
    # print(data)
    timer.stop()
    output = [data["choices"][i]['text'] for i in range(len(data["choices"]))]
    usage = data["usage"]
    logger.debug_detail(f"{usage = }")
    return output

def setup_logging(verbosity: int):
    # Add DEBUG_DETAIL level
    DEBUG_DETAIL = 5
    logging.addLevelName(DEBUG_DETAIL, "DEBUG_DETAIL")
    
    log_levels = [logging.WARNING, logging.INFO, logging.DEBUG, DEBUG_DETAIL]
    log_level = log_levels[min(verbosity, len(log_levels) - 1)]
    
    # Define color codes
    COLORS = {
        'DEBUG_DETAIL': '\033[90m',  # Dark Gray
        'DEBUG': '\033[94m',  # Blue
        'INFO': '\033[92m',   # Green
        'WARNING': '\033[93m',  # Yellow
        'ERROR': '\033[91m',  # Red
        'CRITICAL': '\033[95m',  # Magenta
        'RESET': '\033[0m'    # Reset color
    }
    
    # Custom formatter with colors
    class ColorFormatter(logging.Formatter):
        def format(self, record):
            levelname = record.levelname
            if levelname in COLORS:
                levelname_color = COLORS[levelname] + levelname + COLORS['RESET']
                record.levelname = levelname_color
            return super().format(record)

    # Setup basic configuration
    logging.basicConfig(
        level=log_level,
        format='%(asctime)s - %(levelname)s - %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )

    # Setup console handler with color formatting
    console = logging.StreamHandler()
    console.setLevel(log_level)
    color_formatter = ColorFormatter('%(asctime)s - %(levelname)s - %(message)s')
    console.setFormatter(color_formatter)
    
    # Remove existing handlers and add the new colorful console handler
    logger = logging.getLogger(__name__)
    for handler in logger.handlers[:]:
        logger.removeHandler(handler)
    logger.addHandler(console)

    def debug_detail(message, *args, **kwargs):
        if logger.isEnabledFor(DEBUG_DETAIL):
            logger._log(DEBUG_DETAIL, message, args, **kwargs)
    
    logger.debug_detail = debug_detail

def log_result(message: str):
    """
    Log the result to the console and the log file.
    """
    logger = logging.getLogger(__name__)
    logger.info(message)
    print(message)
    return 


def main(args):
    logger = logging.getLogger(__name__)
    
    model = args.model 
    if not args.use_model_real_name:
        model = "model"
    logger.debug(f"Model: {model!r}")
    prompt = args.prompt
    regex = args.regex
    api_url = f"http://{args.host}:{args.port}/v1/completions"
    logger.info(f"API URL: {api_url!r}")


    n = args.n
    stream = args.stream
    max_tokens = args.max_tokens  # Capture the max_tokens argument
    per_token_control = args.per_token_control

    if stream and n > 1:
        warnings.warn("Streaming is not supported for n > 1. Setting n = 1.")
        n = 1

    logger.info(f"Prompt: {prompt[:50]}..." if len(prompt) > 50 else f"Prompt: {prompt!r}")
    logger.info(f"Prompt length: {len(prompt)}")
    if regex is not None:
        logger.info(f"Regex: {regex!r}")
    timer = Timer()
    timer.start()
    sleep_time = args.sleep_time
    logger.debug(f"Sleep time: {sleep_time} s")

    response = post_http_request(model, prompt, api_url, regex=regex, n=n, stream=stream, max_tokens=max_tokens, sleep_time=sleep_time)

    if stream:
        context = []
        while len(context) < max_tokens:
            num_printed_lines = 0
            num_processed_tokens = 0
            for h in get_streaming_response(response, timer):
                num_printed_lines = 0
                first_line = None
                for i, line in enumerate(h):
                    num_printed_lines += 1
                    logger.debug_detail(f"Beam candidate {i}: {line!r}")
                    if i == 0:
                        first_line = line['text']
                context.append(first_line)
                num_processed_tokens += 1
                if num_processed_tokens == per_token_control:
                    logger.debug(f"Interrupted after {num_processed_tokens} tokens. Currrent context length: {len(context) = } < {max_tokens}")
                    break
            # Resend a request with the context
            new_prompt = prompt + "".join(context)
            logger.debug(f"Sending new request with prompt: {new_prompt if len(new_prompt) < 50 else new_prompt[:50] + '...'}")
            response = post_http_request(model, new_prompt, api_url, regex=regex, n=n, stream=stream, max_tokens=max_tokens, sleep_time=sleep_time)

        if timer.is_timer_started:
            log_result(f"First token latency: {timer.get_first_token_latency()} ms")
            log_result(f"Per-token latency: {timer.get_per_token_latency()} ms")
            log_result(f"Token Per Second: {1000 / timer.get_per_token_latency()} tok / s")
    else:
        output = get_response(response, timer)
        for i, line in enumerate(output):
            logger.debug(f"Beam candidate {i}: {line!r}")
        if timer.is_timer_started:
            log_result(f"First token latency: {timer.get_first_token_latency()} ms")
            log_result(f"Per-token latency: {timer.get_per_token_latency()} ms")
            log_result(f"Token Per Second: {1000 / timer.get_per_token_latency()} tok / s")

    if args.wandb:
        
        data = {
            "first_token_latency": timer.get_first_token_latency(), 
            "per_token_latency": timer.get_per_token_latency(),
            "prompt_length": args.prompt_length,
            "per_token_control": args.per_token_control,
            "max_tokens": args.max_tokens,
            "model": args.model,
            "regex": args.regex,
            "n": args.n,
            "stream": args.stream,
            "comment": args.wandb_comment,
        }
        wandb.log(data)
        
        # table = wandb.Table(columns=list(data.keys()))
        # table.add_data(*list(data.values()))
        # wandb.log({"summary": table})
    

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Run vLLM server and benchmark the latency.",
        epilog="""
        Usage:
            python run_vllm_client.py --stream --per_token_control 1 --max_tokens 16
        
        Experiment:
            python run_vllm_client.py --stream --per_token_control 1 --max_tokens 16
        """,
        # formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--host", type=str, default="localhost")
    parser.add_argument("--port", type=int, default=4242)
    parser.add_argument("--n", type=int, default=4)
    parser.add_argument("--prompt", type=str, default=None)
    parser.add_argument("--prompt_length", type=int, default=None)
    parser.add_argument("--per_token_control", type=int, default=None)
    parser.add_argument("--stream", action="store_true")
    parser.add_argument("--regex", type=str, default=None)
    parser.add_argument("--model", type=str, default="microsoft/Orca-2-13b")
    parser.add_argument("--use_model_real_name", action="store_true")
    parser.add_argument("--max_tokens", type=int, default=16)  # New argument
    parser.add_argument("--start_vllm_server", action="store_true")
    parser.add_argument("-v", "--verbose", action="count", default=0, help="Increase output verbosity (e.g., -v, -vv, -vvv)")
    parser.add_argument("--wandb", action="store_true")
    parser.add_argument("--wandb_project", type=str)
    parser.add_argument("--wandb_comment", type=str, default="")
    parser.add_argument("--sleep_time", type=float, default=0)

    
    args = parser.parse_args()
    setup_logging(args.verbose)

    logger = logging.getLogger(__name__)
    # Truncate the prompt if it's too long
    args_dict = vars(args)
    if args.prompt and len(args.prompt) > 50:
        args_dict['prompt'] = args.prompt[:50] + '...'
    logger.debug(f"Arguments: {args_dict}")

    # prompt and prompt_length are mutually exclusive
    if args.prompt_length is not None and args.prompt is not None:
        raise ValueError("prompt and prompt_length are mutually exclusive")
    if args.prompt is not None and args.prompt_length is not None:
        raise ValueError("prompt and prompt_length need to be provided at least one")
    
    if args.prompt_length is not None:
        args.prompt = " a" * args.prompt_length
    
    if args.prompt is None:
        import transformers
        # Load the tokenizer of the model and tokenize the prompt 
        tokenizer = transformers.AutoTokenizer.from_pretrained(args.model)
        tokenized_result = tokenizer(args.prompt)['input_ids']
        logger.debug(f"tokenized_result: {len(tokenized_result) = }")
        args.prompt_length = len(tokenized_result)

    if args.wandb:
        wandb.init(
            project=args.wandb_project, 
            name=f"token_control-m{args.model}-p{args.prompt_length}-ptc{args.per_token_control}-mt{args.max_tokens}-n{args.n}-s{args.stream}-r{args.regex}"
        )
    
    
    main(args)
