#!/bin/bash
# Helper script to run the local security agent with OLMo-7B via vLLM
#
# Prerequisites:
# 1. vLLM server must be running (use ./local_serve.sh in another terminal)
# 2. MCP security tools must be built
# 3. Security tools (chkrootkit, rkhunter, lynis, htop) must be installed

set -e

echo "==================================================================="
echo "  SPAI Local Security Scan with OLMo-7B"
echo "==================================================================="
echo ""

# Check if vLLM is running
echo "🔍 Checking vLLM server..."
if curl -s http://localhost:8000/health > /dev/null 2>&1; then
    echo "✓ vLLM server is running at http://localhost:8000"
else
    echo "❌ vLLM server is not running!"
    echo ""
    echo "Please start vLLM in another terminal:"
    echo "  ./local_serve.sh"
    echo ""
    echo "Or manually:"
    echo "  python -m vllm.entrypoints.openai.api_server \\"
    echo "      --model allenai/OLMo-7B-1124-Instruct \\"
    echo "      --host 0.0.0.0 \\"
    echo "      --port 8000 \\"
    echo "      --dtype auto \\"
    echo "      --max-model-len 4096 \\"
    echo "      --gpu-memory-utilization 0.9"
    exit 1
fi

# Check if MCP tools are built
echo "🔍 Checking MCP server binaries..."
MISSING_TOOLS=0

for tool in htop chkrootkit rkhunter lynis; do
    BIN_PATH="tools/${tool}-mcp/target/release/${tool}-mcp"
    if [ ! -f "$BIN_PATH" ]; then
        echo "❌ $tool MCP server not found at: $BIN_PATH"
        MISSING_TOOLS=1
    else
        echo "✓ $tool MCP server found"
    fi
done

if [ $MISSING_TOOLS -eq 1 ]; then
    echo ""
    echo "Please build the missing MCP servers:"
    echo "  cd tools && ./build_all_mcp_tools.sh"
    exit 1
fi

# Check if security tools are installed
echo "🔍 Checking security tools installation..."
MISSING_SEC_TOOLS=0

for tool in htop chkrootkit rkhunter lynis; do
    if ! command -v $tool &> /dev/null; then
        echo "❌ $tool is not installed"
        MISSING_SEC_TOOLS=1
    else
        echo "✓ $tool is installed"
    fi
done

if [ $MISSING_SEC_TOOLS -eq 1 ]; then
    echo ""
    echo "Please install missing security tools:"
    echo "  sudo apt-get install chkrootkit rkhunter lynis htop"
    exit 1
fi

echo ""
echo "🚀 All prerequisites met! Running local security scan..."
echo ""
echo "This will:"
echo "  1. Use OLMo-7B (local model via vLLM)"
echo "  2. Run htop for process monitoring"
echo "  3. Run chkrootkit, rkhunter, and lynis for rootkit detection"
echo "  4. Provide comprehensive security assessment with cross-tool correlation"
echo ""
echo "Note: This may take several minutes depending on system size."
echo ""
read -p "Press Enter to continue or Ctrl+C to cancel..."
echo ""

# Set vLLM endpoint (optional, defaults to localhost:8000)
export VLLM_BASE_URL=${VLLM_BASE_URL:-http://localhost:8000}

# Run the local security agent
cargo run --example local_agent_chkrootkit --features mcp-tools

echo ""
echo "==================================================================="
echo "  Security scan complete!"
echo "==================================================================="
