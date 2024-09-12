import pyaici.server as aici
import time
import os

aici.log_level = 1

async def main(per_token_control, max_tokens, num_prompt_tokens):
    await aici.FixedTokens(" a" * num_prompt_tokens)
    num_iter = max_tokens // per_token_control
    tokens_per_iter = per_token_control
    global regex_pattern
    for _ in range(num_iter):
        tokens = await aici.gen_tokens(max_tokens=tokens_per_iter, regex=regex_pattern)
    return

# aici.test(main())
per_token_control = 16
max_tokens = 128
num_prompt_tokens = 100
print(f"Running AICI with: {per_token_control = 16
regex_pattern = "([a-z]+){" + str(per_token_control) + "}"
# print(f"regex_pattern: {regex_pattern}")
aici.start(main(per_token_control, max_tokens, num_prompt_tokens))