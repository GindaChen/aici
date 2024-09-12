from openai import OpenAI
import json
import time
# Modify OpenAI's API key and API base to use vLLM's API server.
openai_api_key = "EMPTY"
openai_api_base = "http://localhost:4242/v1"

client = OpenAI(
    # defaults to os.environ.get("OPENAI_API_KEY")
    api_key=openai_api_key,
    base_url=openai_api_base,
)

models = client.models.list()
model = models.data[0].id

# Completion API
json_schema = {
  # Summarize the json schema if you want an LLM to generate this json schema:
  # Generate a json schema for a service that provides a REST API for a backend service.
  # The service has a provider key, a service token, and a backend version.
  # The service has a backend that has an endpoint and a host.
  # The service has a proxy that has an API backend, hosts, and proxy rules.
  # The proxy rules have a http method, a pattern, a metric system name, and a delta.
  "$schema": "http://json-schema.org/draft-04/schema#",
  "type": "object",
  "properties": {
    "provider_key": {
      "type": "string"
    },
    "services": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "myid": {
            "type": "integer"
          },
          "backend_authentication_type": {
            "type": "string",
            "enum": [
              "provider",
              "service_token"
            ]
          },
          "backend_authentication_value": {
            "type": "string"
          },
          "backend_version": {
            "type": "string",
            "enum": [
              "1",
              "2",
              "oauth"
            ]
          },
          "backend": {
            "type": "object",
            "properties": {
              "endpoint": {
                "type": "string"
              },
              "host": {
                "type": "string"
              }
            },
            "required": [
              "endpoint"
            ]
          },
          "proxy": {
            "type": "object",
            "properties": {
              "api_backend": {
                "type": "string"
              },
              "hosts": {
                "type": "array",
                "items": {
                  "type": "string"
                }
              },
              "proxy_rules": {
                "type": "array",
                "items": {
                  "type": "object",
                  "properties": {
                    "http_method": {
                      "type": "string"
                    },
                    "pattern": {
                      "type": "string"
                    },
                    "metric_system_name": {
                      "type": "string"
                    },
                    "delta": {
                      "type": "integer"
                    }
                  },
                  "required": [
                    "http_method",
                    "pattern",
                    "metric_system_name",
                    "delta"
                  ]
                }
              }
            },
            "required": [
              "api_backend",
              "hosts",
              "proxy_rules"
            ]
          }
        },
        "required": [
          "id",
          "backend_authentication_type",
          "backend_authentication_value",
          "backend_version",
          "proxy"
        ]
      }
    }
  },
  "required": [
    "services"
  ]
}
# regex_pattern = "(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-][0-9]+)?"
regex_pattern = "[^a-z]{4,8}c*@gmail.com"

# stream = False
stream = True

timestamps = []
timestamps.append({
    "event": "start",
    "ts": time.time(),
})
prompt = """
Generate a json schema for a service that provides a REST API for a backend service.
The service has a provider key, a service token, and a backend version.
The service has a backend that has an endpoint and a host.
The service has a proxy that has an API backend, hosts, and proxy rules.
The proxy rules have a http method, a pattern, a metric system name, and a delta.
It follows the following json schema:
{json_schema}
"""
completion = client.completions.create(
    model=model,
    # prompt="What is Pi? Give me the first 15 digits:",
    prompt=prompt,
    echo=False,
    n=1,
    stream=stream,
    # logprobs=3,
    logprobs=1,
    max_tokens=100,
    extra_body=dict(
        guided_json=json.dumps(json_schema),
        # guided_regex=regex_pattern,
    ),
)


print("Completion results:")
if stream:
    for i, c in enumerate(completion):
        # print(c)
        print(c.choices[0].text)
        if i == 0:
            timestamps.append({
                "event": "first_output_token",
                "ts": time.time(),
            })
        else:
            timestamps.append({
                "event": "output_token",
                "ts": time.time(),
            })
else:
    print(completion)

timestamps.append({
    "event": "end",
    "ts": time.time(),
})
# Calculate the time between each "start" and "first_output_token" events
first_token_latency = timestamps[1]["ts"] - timestamps[0]["ts"]
# Calculate the average time between each "output_token" events
output_token_latency = 0
for i in range(1, len(timestamps)):
    output_token_latency += timestamps[i]["ts"] - timestamps[i-1]["ts"]
output_token_latency /= len(timestamps) - 1

print(f"First token latency: {first_token_latency * 1000:.2f} ms")
print(f"Average output token latency: {output_token_latency * 1000:.2f} ms")
print(f"Total latency: {(timestamps[-1]['ts'] - timestamps[0]['ts']) * 1000:.2f} ms")

print(f"Prompting throughput: {1 / first_token_latency:.2f} tokens/s")
print(f"Sampling throughput: {1 / output_token_latency:.2f} tokens/s")