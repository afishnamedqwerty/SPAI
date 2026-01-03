#!/bin/bash
# Comprehensive setup script for all security scanning tools
# Run this with: sudo bash tools/setup_all_security_tools.sh

set -e

echo "════════════════════════════════════════════════════════════"
echo "    Security Tools Suite - Comprehensive Setup"
echo "════════════════════════════════════════════════════════════"
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
   echo "❌ This script must be run as root (use: sudo bash $0)"
   exit 1
fi

# Get the actual user (not root when using sudo)
ACTUAL_USER="${SUDO_USER:-$USER}"

if [ "$ACTUAL_USER" = "root" ]; then
    echo "⚠️  Running as actual root user. Passwordless sudo not needed."
    echo "   You can run all tools directly without this setup."
    SKIP_SUDO=true
else
    SKIP_SUDO=false
fi

echo "Setting up security tools for user: $ACTUAL_USER"
echo

# Detect package manager
if command -v apt-get &> /dev/null; then
    PKG_MGR="apt-get"
    PKG_UPDATE="apt-get update"
    PKG_INSTALL="apt-get install -y"
elif command -v dnf &> /dev/null; then
    PKG_MGR="dnf"
    PKG_UPDATE="dnf check-update || true"
    PKG_INSTALL="dnf install -y"
elif command -v yum &> /dev/null; then
    PKG_MGR="yum"
    PKG_UPDATE="yum check-update || true"
    PKG_INSTALL="yum install -y"
elif command -v pacman &> /dev/null; then
    PKG_MGR="pacman"
    PKG_UPDATE="pacman -Sy"
    PKG_INSTALL="pacman -S --noconfirm"
else
    echo "❌ Could not detect package manager. Please install manually:"
    echo "   - chkrootkit"
    echo "   - rkhunter"
    echo "   - lynis"
    exit 1
fi

echo "✓ Detected package manager: $PKG_MGR"
echo

# Update package lists
echo "Updating package lists..."
$PKG_UPDATE
echo

# Install tools
declare -A TOOLS
TOOLS=(
    ["chkrootkit"]="/usr/sbin/chkrootkit"
    ["rkhunter"]="/usr/bin/rkhunter"
    ["lynis"]="/usr/sbin/lynis"
    ["htop"]="/usr/bin/htop"
    ["tshark"]="/usr/bin/tshark"
    ["lsof"]="/usr/bin/lsof"
    ["ss"]="/usr/bin/ss"
)

# Additional packages that may have different package names
echo "Installing additional utilities..."
if [ "$PKG_MGR" = "apt-get" ]; then
    apt-get install -y psmisc iproute2 procps 2>/dev/null || true
elif [ "$PKG_MGR" = "dnf" ] || [ "$PKG_MGR" = "yum" ]; then
    $PKG_INSTALL psmisc iproute procps-ng 2>/dev/null || true
elif [ "$PKG_MGR" = "pacman" ]; then
    pacman -S --noconfirm psmisc iproute2 procps-ng 2>/dev/null || true
fi
echo

for tool in "${!TOOLS[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo "✓ $tool is already installed at: $(which $tool)"
    else
        echo "Installing $tool..."
        $PKG_INSTALL "$tool"

        if command -v "$tool" &> /dev/null; then
            echo "✓ $tool installed successfully"
        else
            echo "⚠️  $tool installation may have issues, please verify manually"
        fi
    fi
done

echo

# Install SPAI portlist tool
echo "Installing SPAI portlist tool..."
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PORTLIST_SOURCE="${SCRIPT_DIR}/portlist"

if [ -f "$PORTLIST_SOURCE" ]; then
    cp "$PORTLIST_SOURCE" /usr/local/bin/portlist
    chmod 755 /usr/local/bin/portlist
    echo "✓ portlist installed to /usr/local/bin/portlist"
else
    echo "⚠️  portlist script not found at $PORTLIST_SOURCE"
    echo "   Please ensure tools/portlist exists in the SPAI directory"
fi
echo

# Update rkhunter database
if command -v rkhunter &> /dev/null; then
    echo "Updating rkhunter database..."
    rkhunter --update || echo "⚠️  rkhunter update had issues (may be normal)"
    echo "✓ rkhunter database updated"
    echo
fi

# Set up passwordless sudo if not root
if [ "$SKIP_SUDO" = false ]; then
    echo "Setting up passwordless sudo..."

    SUDOERS_FILE="/etc/sudoers.d/security-tools"

    # Find actual paths
    CHKROOTKIT_PATH=$(which chkrootkit || echo "/usr/sbin/chkrootkit")
    RKHUNTER_PATH=$(which rkhunter || echo "/usr/bin/rkhunter")
    LYNIS_PATH=$(which lynis || echo "/usr/sbin/lynis")
    TSHARK_PATH=$(which tshark || echo "/usr/bin/tshark")
    DUMPCAP_PATH=$(which dumpcap || echo "/usr/bin/dumpcap")
    PORTLIST_PATH=$(which portlist || echo "/usr/local/bin/portlist")

    cat > "$SUDOERS_FILE" <<EOF
# Allow $ACTUAL_USER to run security tools without password
# Created by SPAI security tools setup script

$ACTUAL_USER ALL=(ALL) NOPASSWD: $CHKROOTKIT_PATH
$ACTUAL_USER ALL=(ALL) NOPASSWD: $RKHUNTER_PATH
$ACTUAL_USER ALL=(ALL) NOPASSWD: $LYNIS_PATH
$ACTUAL_USER ALL=(ALL) NOPASSWD: $TSHARK_PATH
$ACTUAL_USER ALL=(ALL) NOPASSWD: $DUMPCAP_PATH
$ACTUAL_USER ALL=(ALL) NOPASSWD: $PORTLIST_PATH
EOF

    # Set proper permissions
    chmod 0440 "$SUDOERS_FILE"

    # Validate sudoers file
    if visudo -c -f "$SUDOERS_FILE"; then
        echo "✓ Sudoers file created successfully at: $SUDOERS_FILE"
        echo "✓ User '$ACTUAL_USER' can now run security tools without password"
    else
        echo "❌ Error: Invalid sudoers syntax"
        rm -f "$SUDOERS_FILE"
        exit 1
    fi
fi

echo
echo "════════════════════════════════════════════════════════════"
echo "                   Setup Complete!"
echo "════════════════════════════════════════════════════════════"
echo
echo "Installed tools:"
for tool in "${!TOOLS[@]}"; do
    if command -v "$tool" &> /dev/null; then
        echo "  ✓ $tool → $(which $tool)"
    else
        echo "  ✗ $tool → NOT FOUND"
    fi
done

# Check SPAI portlist
if command -v portlist &> /dev/null; then
    echo "  ✓ portlist (SPAI) → $(which portlist)"
else
    echo "  ✗ portlist (SPAI) → NOT FOUND"
fi

if [ "$SKIP_SUDO" = false ]; then
    echo
    echo "Passwordless sudo configured for:"
    echo "  • chkrootkit"
    echo "  • rkhunter"
    echo "  • lynis"
    echo "  • tshark/dumpcap"
    echo "  • portlist (SPAI custom tool)"
    echo
    echo "Additional tools installed:"
    echo "  • htop (process monitoring)"
    echo "  • lsof (list open files)"
    echo "  • ss (socket statistics)"
    echo
    echo "Test with:"
    echo "  sudo chkrootkit -x | head -20"
    echo "  sudo rkhunter --check --skip-keypress --report-warnings-only"
    echo "  sudo lynis audit system --quick"
    echo "  portlist -a -s  # Show all ports and highlight suspicious ones"
fi

echo
echo "Next steps:"
echo "  1. Build the MCP servers:"
echo "     ./tools/build_all_mcp.sh"
echo "  2. Set your OpenRouter API key:"
echo "     export OPENROUTER_API_KEY=your_key"
echo "  3. Run the comprehensive security agent:"
echo "     cargo run --example basic_agent_chkrootkit --features mcp-tools"
echo
