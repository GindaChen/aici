import time
from datetime import datetime
import asyncio
import aiohttp
from openai import OpenAI

# Modify OpenAI's API key and API base to use vLLM's API server.
openai_api_key = "EMPTY"
openai_api_base = "http://localhost:4242/v1"

client = OpenAI(
    api_key=openai_api_key,
    base_url=openai_api_base,
)

def debug_print(message):
    """Prints debug messages with a timestamp for easier debugging."""
    now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{now}] DEBUG: {message}")

def print_timeline(elapsed_times):
    """Prints a simple ASCII timeline of events based on elapsed time from program start."""
    # Define a scale
    scale_width = 50
    max_time = max(t['end'] for t in elapsed_times.values())
    scale_step = max_time / scale_width

    def time_to_pos(ms):
        return int(ms / scale_step)

    print("Timeline (elapsed time in ms):")
    for key, t in elapsed_times.items():
        start_pos = time_to_pos(t['start'])
        end_pos = time_to_pos(t['end'])
        timeline_line = [' '] * scale_width
        timeline_line[start_pos] = '|'
        if end_pos < scale_width:
            timeline_line[end_pos] = '|'
        print(f"{key:<12}: {'-' * start_pos + '^' + '-' * (end_pos - start_pos - 1) + '>'}")

async def send_prompt(session, model, prompt, start_time):
    request_start = time.time()
    debug_print("Sending doc prompt...")
    data = {
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": prompt},
        ],
        "max_tokens": 1  # Limit token generation
    }
    async with session.post(f"{openai_api_base}/chat/completions", json=data) as response:
        result = await response.json()
        request_end = time.time()
        debug_print("Received response for doc prompt.")
        return result['choices'][0]['message']['content'], (request_start - start_time) * 1000, (request_end - start_time) * 1000

async def fetch_response(session, model, prompt, question, start_time):
    request_start = time.time()
    debug_print(f"Sending question: {question}")
    data = {
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": f"{prompt} {question}"},
        ]
    }
    async with session.post(f"{openai_api_base}/chat/completions", json=data) as response:
        result = await response.json()
        request_end = time.time()
        debug_print(f"Received response for question: {question}.")
        return result['choices'][0]['message']['content'], (request_start - start_time) * 1000, (request_end - start_time) * 1000

async def main():
    start_time = time.time()

    # Generate a 100-line story for the doc prompt
    from datetime import datetime
    doc_prompt = (
        # Add current date time string
        f"Generated on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
        "Once upon a time in a faraway land, there was a small village nestled in a lush, green valley. "
        "The villagers lived in harmony with nature, tending to their crops and livestock with great care. "
        "Every morning, the sun would rise over the mountains, casting a golden glow across the village. "
        "Children played in the meadows while adults worked in the fields, singing songs of old. "
        "One day, a mysterious traveler arrived at the village, bringing with him tales of distant lands and ancient magic. "
        "He spoke of grand adventures and mythical creatures, captivating the villagers with his stories. "
        "The traveler stayed for several weeks, sharing his knowledge and teaching the villagers about the world beyond their valley. "
        "As time passed, the village grew curious and eager to explore the unknown. "
        "The traveler eventually departed, leaving behind a legacy of wonder and excitement. "
        "The villagers continued their daily lives but now looked at the world with new eyes, inspired by the tales they had heard. "
        "And so, the small village thrived, forever changed by the visit of the mysterious traveler. "
        "As seasons came and went, the story of the traveler was told and retold, becoming a cherished part of the village's history. "
        "Each year, on the anniversary of his departure, the villagers would gather to celebrate the magic and mystery he had brought into their lives. "
        "The story of the traveler became a symbol of the endless possibilities that lay beyond the horizon. "
        "And though the traveler never returned, his impact was felt for generations, reminding the villagers to always dream and explore. "
        # Repeat or extend the story to reach about 100 lines
    ) * 5  # Adjust the repetition to fit approximately 100 lines

    questions = [
        "Is this hate speech?",
        "Is this sexual?",
        "Is this violent?",
        "Is this harmful?",
        "Is this illegal?",
        "Is this unethical?",
        "Is this dangerous?",
        "Is this offensive?",
    ]
    questions = [q + " Only answer yes or no." for q in questions]
    
    async with aiohttp.ClientSession() as session:
        model = "model"

        # Send the doc prompt with max_tokens set to 1
        doc_response, doc_start, doc_end = await send_prompt(session, model, doc_prompt, start_time)
        print(f"Document Prompt Response: {doc_response}\n")
        print(f"Document Prompt Start Time: {doc_start} ms")
        print(f"Document Prompt End Time: {doc_end} ms")

        # Send parallel requests for each question
        debug_print("Sending questions in parallel...")
        tasks = [fetch_response(session, model, doc_prompt, question, start_time) for question in questions]
        responses = await asyncio.gather(*tasks)
        
        debug_print("Received all responses.")

        # Collect elapsed times for visualization
        elapsed_times = {"doc_prompt": {"start": doc_start, "end": doc_end}}
        for i, (response, start, end) in enumerate(responses):
            elapsed_times[f"question_{i}"] = {"start": start, "end": end}
            print(f"Question: {questions[i]}\nAnswer: {response}\n")
            print(f"Question {i} Start Time: {start} ms")
            print(f"Question {i} End Time: {end} ms")

    # Print the ASCII timeline
    print_timeline(elapsed_times)

if __name__ == "__main__":
    asyncio.run(main())