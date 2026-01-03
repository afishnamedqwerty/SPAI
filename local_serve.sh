#!/usr/bin/env bash
set -euo pipefail

# python3 -m venv ./.venv (if not already done so)
# pip install -r requirements.txt (if not already done so)
source /home/outrun/projects/spai/.venv/bin/activate

# Launch a local vLLM OpenAI-compatible server with the OLMo 7B instruct model.
 python -m vllm.entrypoints.openai.api_server \
  --model allenai/OLMo-3-7B-Think \
  --host 0.0.0.0 \
  --port 8000 \
  --dtype auto \
  --max-model-len 16224 \
  --gpu-memory-utilization 0.9
