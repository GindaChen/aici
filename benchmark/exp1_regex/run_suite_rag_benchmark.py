import subprocess
import os
import csv
from io import StringIO
import sys

# CSV-like string for the argument arrays
csv_data = """
prompt_length,num_fast_forward_tokens,num_iterations,num_generate_tokens
2042,1027,4,16
1660,1120,7,36
1991,1077,7,23
1600,996,9,11
1656,987,5,26
1922,933,11,33
1392,1096,9,20
1933,960,6,25
1409,909,8,20
1404,1049,8,39
2008,1112,6,16
1845,925,10,24
"""

# Prepare arrays from the CSV data
csv_reader = csv.DictReader(StringIO(csv_data.strip()))
experiment_params = list(csv_reader)

# Assert that we have data
if not experiment_params:
    raise ValueError("No experiment parameters found in CSV data")

script_dir = os.path.dirname(os.path.abspath(__file__))

print(f"Running {len(experiment_params)} experiments")
print(f"Experiment parameters: \n{csv_data}")

for params in experiment_params:
    print("----------------------------------------")
    print(f"Running experiment with parameters: {params}")
    
    subprocess.run([
        "bash",
        os.path.join(script_dir, "run_rag_benchmark.sh"),
        "--prompt_length", params['prompt_length'],
        "--num_fast_forward_tokens", params['num_fast_forward_tokens'],
        "--num_iterations", params['num_iterations'],
        "--num_generate_tokens", params['num_generate_tokens'],
        # put argv from command line here
        *sys.argv[1:]
    ])
    
    print("----------------------------------------")


