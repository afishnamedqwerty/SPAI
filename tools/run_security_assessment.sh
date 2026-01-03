#!/bin/bash
#
# SPAI Security Assessment - Direct Tool Execution
# This script runs the security tools DIRECTLY and captures REAL output.
# The LLM is ONLY used for final analysis of already-collected data.
#
# Usage: ./tools/run_security_assessment.sh
#

set -e

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_DIR="security_assessment_$TIMESTAMP"
TOOLS_DIR="tools"

mkdir -p "$OUTPUT_DIR"

echo "═══════════════════════════════════════════════════════════════"
echo "   SPAI Direct Security Assessment"
echo "   Timestamp: $TIMESTAMP"
echo "   Output: $OUTPUT_DIR/"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# ─────────────────────────────────────────────────────────────────────────
# PHASE 1: NETWORK ANALYSIS (Direct execution, no LLM)
# ─────────────────────────────────────────────────────────────────────────
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  PHASE 1: Network Analysis (Direct)                         │"
echo "└─────────────────────────────────────────────────────────────┘"

{
    echo "=== LISTENING PORTS ==="
    ss -tulnp 2>/dev/null || netstat -tulnp 2>/dev/null || echo "Command not available"
    echo ""
    
    echo "=== ESTABLISHED CONNECTIONS ==="
    ss -tupn state established 2>/dev/null || echo "No established connections"
    echo ""
    
    echo "=== PROCESS NETWORK ACTIVITY (lsof) ==="
    lsof -i -n -P 2>/dev/null | head -50 || echo "lsof not available"
    echo ""
    
    echo "=== NETWORK CONNECTIONS BY PROCESS ==="
    for pid in $(ss -tupn 2>/dev/null | grep -oP 'pid=\K[0-9]+' | sort -u | head -20); do
        proc_name=$(ps -p $pid -o comm= 2>/dev/null || echo "unknown")
        echo "PID $pid ($proc_name):"
        ss -tupn 2>/dev/null | grep "pid=$pid" | head -3
        echo ""
    done
} > "$OUTPUT_DIR/01_network_analysis.txt"
echo "   ✓ Saved to $OUTPUT_DIR/01_network_analysis.txt"

# ─────────────────────────────────────────────────────────────────────────
# PHASE 2: PROCESS ANALYSIS (Direct execution, no LLM)
# ─────────────────────────────────────────────────────────────────────────
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  PHASE 2: Process Analysis (Direct)                         │"
echo "└─────────────────────────────────────────────────────────────┘"

{
    echo "=== TOP 30 PROCESSES BY CPU ==="
    ps aux --sort=-%cpu | head -31
    echo ""
    
    echo "=== TOP 30 PROCESSES BY MEMORY ==="
    ps aux --sort=-%mem | head -31
    echo ""
    
    echo "=== PROCESS TREE ==="
    pstree -pa 2>/dev/null | head -100 || ps auxf | head -100
    echo ""
    
    echo "=== SUSPICIOUS PROCESS INDICATORS ==="
    echo "Processes with hidden names (starting with .):"
    ps aux | awk '$11 ~ /^\./' | head -10 || echo "  None found"
    echo ""
    echo "Processes running from /tmp or /dev/shm:"
    ps aux | grep -E '/tmp/|/dev/shm/' | grep -v grep | head -10 || echo "  None found"
    echo ""
    echo "Processes with very high CPU (>50%):"
    ps aux | awk 'NR>1 && $3>50' | head -10 || echo "  None found"
    echo ""
    echo "Processes with very high memory (>20%):"
    ps aux | awk 'NR>1 && $4>20' | head -10 || echo "  None found"
} > "$OUTPUT_DIR/02_process_analysis.txt"
echo "   ✓ Saved to $OUTPUT_DIR/02_process_analysis.txt"

# ─────────────────────────────────────────────────────────────────────────
# PHASE 3: ROOTKIT DETECTION (Direct execution, no LLM)
# ─────────────────────────────────────────────────────────────────────────
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  PHASE 3: Rootkit Detection (Direct)                        │"
echo "└─────────────────────────────────────────────────────────────┘"

{
    echo "=== CHKROOTKIT SCAN ==="
    if command -v chkrootkit &> /dev/null; then
        sudo chkrootkit 2>&1 | head -200 || echo "chkrootkit failed"
    else
        echo "chkrootkit NOT INSTALLED - run: sudo apt install chkrootkit"
    fi
    echo ""
    
    echo "=== RKHUNTER SCAN ==="
    if command -v rkhunter &> /dev/null; then
        sudo rkhunter --check --skip-keypress --report-warnings-only 2>&1 | head -200 || echo "rkhunter completed"
    else
        echo "rkhunter NOT INSTALLED - run: sudo apt install rkhunter"
    fi
    echo ""
    
    echo "=== MANUAL ROOTKIT INDICATORS ==="
    echo "SUID files in /tmp:"
    find /tmp -perm -4000 2>/dev/null | head -10 || echo "  None found"
    echo ""
    echo "Hidden files in /tmp:"
    find /tmp -name ".*" -type f 2>/dev/null | head -10 || echo "  None found"
    echo ""
    echo "Recently modified binaries in /bin or /sbin (last 7 days):"
    find /bin /sbin -type f -mtime -7 2>/dev/null | head -10 || echo "  None found"
} > "$OUTPUT_DIR/03_rootkit_detection.txt"
echo "   ✓ Saved to $OUTPUT_DIR/03_rootkit_detection.txt"

# ─────────────────────────────────────────────────────────────────────────
# PHASE 4: SYSTEM HARDENING (Direct execution, no LLM)
# ─────────────────────────────────────────────────────────────────────────
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  PHASE 4: System Hardening Audit (Direct)                   │"
echo "└─────────────────────────────────────────────────────────────┘"

{
    echo "=== LYNIS AUDIT ==="
    if command -v lynis &> /dev/null; then
        sudo lynis audit system --quick --no-colors 2>&1 | head -500 || echo "lynis completed"
    else
        echo "lynis NOT INSTALLED - run: sudo apt install lynis"
    fi
} > "$OUTPUT_DIR/04_lynis_audit.txt"
echo "   ✓ Saved to $OUTPUT_DIR/04_lynis_audit.txt"

# ─────────────────────────────────────────────────────────────────────────
# PHASE 5: GENERATE SUMMARY
# ─────────────────────────────────────────────────────────────────────────
echo ""
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  PHASE 5: Summary Generation                                │"
echo "└─────────────────────────────────────────────────────────────┘"

{
    echo "═══════════════════════════════════════════════════════════════"
    echo "   SECURITY ASSESSMENT SUMMARY"
    echo "   Generated: $(date)"
    echo "   Hostname: $(hostname)"
    echo "   Kernel: $(uname -r)"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    echo "FILES GENERATED:"
    ls -la "$OUTPUT_DIR/"
    echo ""
    
    echo "─────────────────────────────────────────────────────────────"
    echo "NETWORK FINDINGS:"
    echo "─────────────────────────────────────────────────────────────"
    echo "Listening ports: $(ss -tulnp 2>/dev/null | grep LISTEN | wc -l)"
    echo "Established connections: $(ss -tupn state established 2>/dev/null | wc -l)"
    echo ""
    
    echo "─────────────────────────────────────────────────────────────"
    echo "PROCESS FINDINGS:"
    echo "─────────────────────────────────────────────────────────────"
    echo "Total processes: $(ps aux | wc -l)"
    echo "High CPU (>50%): $(ps aux | awk 'NR>1 && $3>50' | wc -l)"
    echo "High memory (>20%): $(ps aux | awk 'NR>1 && $4>20' | wc -l)"
    echo ""
    
    echo "─────────────────────────────────────────────────────────────"
    echo "ROOTKIT WARNINGS:"
    echo "─────────────────────────────────────────────────────────────"
    grep -i "infected\|warning\|suspicious" "$OUTPUT_DIR/03_rootkit_detection.txt" 2>/dev/null | head -10 || echo "No rootkit warnings found"
    echo ""
    
    echo "─────────────────────────────────────────────────────────────"
    echo "LYNIS HARDENING SCORE:"
    echo "─────────────────────────────────────────────────────────────"
    grep -i "hardening index" "$OUTPUT_DIR/04_lynis_audit.txt" 2>/dev/null || echo "Check lynis output for score"
    echo ""
    
    echo "═══════════════════════════════════════════════════════════════"
    echo "REVIEW COMMANDS:"
    echo "═══════════════════════════════════════════════════════════════"
    echo "cat $OUTPUT_DIR/01_network_analysis.txt"
    echo "cat $OUTPUT_DIR/02_process_analysis.txt"
    echo "cat $OUTPUT_DIR/03_rootkit_detection.txt"
    echo "cat $OUTPUT_DIR/04_lynis_audit.txt"
    echo "cat $OUTPUT_DIR/05_summary.txt"
} > "$OUTPUT_DIR/05_summary.txt"
echo "   ✓ Saved to $OUTPUT_DIR/05_summary.txt"

# ─────────────────────────────────────────────────────────────────────────
# DONE
# ─────────────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "   ASSESSMENT COMPLETE"
echo "   All output saved to: $OUTPUT_DIR/"
echo "═══════════════════════════════════════════════════════════════"
echo ""
cat "$OUTPUT_DIR/05_summary.txt"
