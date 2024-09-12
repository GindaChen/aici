import pyaici.server as aici
import time
import os

aici.log_level = 1

knowledge_base = {}

async def main(num_fast_forward_tokens, num_generate_tokens, num_prompt_tokens, num_iterations):
    diff_prompt_tokens = " a" * num_prompt_tokens
    for _ in range(num_iterations):
        await aici.FixedTokens(diff_prompt_tokens)
        tokens = await aici.gen_tokens(max_tokens=num_generate_tokens)
        diff_prompt_tokens = " a" * num_fast_forward_tokens
    return


num_prompt_tokens = 1845
num_fast_forward_tokens = 925
num_generate_tokens = 24
num_iterations = 10
print(f"Running AICI with: {num_fast_forward_tokens = } {num_generate_tokens = } {num_prompt_tokens = } {num_iterations = }")
aici.start(main(num_fast_forward_tokens, num_generate_tokens, num_prompt_tokens, num_iterations))