#!/bin/bash
# Build all MCP security tool servers

set -e

echo "════════════════════════════════════════════════════════════"
echo "       Building All MCP Security Tool Servers"
echo "════════════════════════════════════════════════════════════"
echo

MCP_TOOLS=("chkrootkit-mcp" "rkhunter-mcp" "lynis-mcp" "htop-mcp" "tshark-mcp" "procinfo-mcp")

for tool in "${MCP_TOOLS[@]}"; do
    echo "Building $tool..."
    if [ -d "tools/$tool" ]; then
        cargo build --release --manifest-path "tools/$tool/Cargo.toml"

        binary="tools/$tool/target/release/$tool"
        if [ -f "$binary" ]; then
            size=$(du -h "$binary" | cut -f1)
            echo "✓ $tool built successfully ($size)"
        else
            echo "❌ Failed to build $tool"
            exit 1
        fi
    else
        echo "⚠️  $tool directory not found, skipping..."
    fi
    echo
done

echo "════════════════════════════════════════════════════════════"
echo "              All MCP Servers Built Successfully!"
echo "════════════════════════════════════════════════════════════"
echo
echo "Built binaries:"
for tool in "${MCP_TOOLS[@]}"; do
    binary="tools/$tool/target/release/$tool"
    if [ -f "$binary" ]; then
        ls -lh "$binary" | awk '{print "  ✓", $9, "(" $5 ")"}'
    fi
done

echo
echo "Next steps:"
echo "  # Run basic security agent:"
echo "  cargo run --example basic_agent_chkrootkit --features mcp-tools"
echo
echo "  # Run swarm security agent (multi-agent):"
echo "  cargo run --example swarm_security_agent --features mcp-tools"
echo

