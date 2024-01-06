import subprocess
import ujson
import sys
import os
import argparse

import pyaici.ast as ast
import pyaici.rest
import pyaici.util


def upload_wasm(prog: str):
    r = subprocess.run(["cargo", "build", "--release"], cwd=prog)
    if r.returncode != 0:
        sys.exit(1)
    file_path = "target/wasm32-wasi/release/aici_" + prog + ".wasm"
    return pyaici.rest.upload_module(file_path)


def ask_completion(cmd_args, *args, **kwargs):
    for k in ["max_tokens", "prompt", "ignore_eos"]:
        v = getattr(cmd_args, k)
        if v is not None:
            kwargs[k] = v
    pyaici.rest.log_level = cmd_args.log_level or pyaici.rest.log_level
    res = pyaici.rest.completion(*args, **kwargs)
    print("\n[Prompt] " + res["request"]["prompt"] + "\n")
    for text in res["text"]:
        print("[Response] " + text + "\n")
    os.makedirs("tmp", exist_ok=True)
    path = "tmp/response.json"
    with open(path, "w") as f:
        ujson.dump(res, f, indent=1)
    print(f"response saved to {path}")
    print("Usage:", res["usage"])
    print("Storage:", res["storage"])


def main():
    parser = argparse.ArgumentParser(
        description="Upload an AICI VM and completion request to rllm or vllm",
        epilog="""
The --vm is determined automatically, if the file ends with .py (pyvm), .json (declvm),
or .txt (no vm).

--log-level defaults to 1 for no vm, and 3 otherwise.
""",
    )
    parser.add_argument("--prompt", "-p", default="", type=str, help="specify prompt")
    parser.add_argument(
        "--log-level", "-l", type=int, help="log level (higher is more)"
    )
    parser.add_argument(
        "--max-tokens", "-t", type=int, help="maximum number of tokens to generate"
    )
    parser.add_argument(
        "--ignore-eos", action="store_true", help="ignore EOS tokens generated by model"
    )
    parser.add_argument(
        "--vm", type=str, help="path to .wasm file to upload or 'pyvm' or 'declvm'"
    )
    parser.add_argument(
        "file", type=str, nargs="?", help="path to file to pass to the vm"
    )
    args = parser.parse_args()

    fn: str | None = args.file

    if fn is not None and fn.endswith(".txt") and args.vm is None:
        pyaici.rest.log_level = 1
        args.prompt = open(fn).read()
        fn = None

    aici_arg = ""
    if fn is not None:
        aici_arg = open(fn).read()
    aici_module = ""

    vm: str = args.vm
    if vm is None and fn is not None:
        if fn.endswith(".py"):
            vm = "pyvm"
        elif fn.endswith(".json"):
            vm = "declvm"
        else:
            print("Can't determine VM type from file name: " + fn)
            sys.exit(1)

    if vm == "pyvm":
        aici_module = upload_wasm("pyvm")
    elif vm == "declvm":
        aici_module = upload_wasm("declvm")
    elif vm is None:
        if args.prompt == "":
            parser.print_help()
            print("\nError: --prompt empty and no --vm; bailing")
            sys.exit(1)
        pass
    else:
        aici_module = pyaici.rest.upload_module(vm)

    if aici_module:
        pyaici.rest.log_level = 3
        ask_completion(
            args,
            aici_module=aici_module,
            aici_arg=aici_arg,
            ignore_eos=True,
            max_tokens=2000,
        )
    else:
        pyaici.rest.log_level = 1
        ask_completion(
            args,
            aici_module=None,
            aici_arg=None,
            max_tokens=100,
        )


main()
