#!/bin/bash
# 24-Hour Stress Test for BTPC Testnet
# Monitors node health every 5 minutes and logs metrics

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="$SCRIPT_DIR/logs/stress-test-24hr.log"
MONITOR_SCRIPT="$SCRIPT_DIR/monitor-testnet.sh"

# Test duration (24 hours = 86400 seconds, check every 5 minutes = 300 seconds)
DURATION=$((24 * 60 * 60))
INTERVAL=300
ITERATIONS=$((DURATION / INTERVAL))

echo "=============================================" | tee -a "$LOG_FILE"
echo "BTPC 24-Hour Stress Test Started" | tee -a "$LOG_FILE"
echo "Start Time: $(date '+%Y-%m-%d %H:%M:%S')" | tee -a "$LOG_FILE"
echo "Duration: 24 hours ($ITERATIONS checks at ${INTERVAL}s intervals)" | tee -a "$LOG_FILE"
echo "Log File: $LOG_FILE" | tee -a "$LOG_FILE"
echo "=============================================" | tee -a "$LOG_FILE"
echo "" | tee -a "$LOG_FILE"

for i in $(seq 1 $ITERATIONS); do
    echo "--- Check $i of $ITERATIONS ($(date '+%Y-%m-%d %H:%M:%S')) ---" >> "$LOG_FILE"
    $MONITOR_SCRIPT >> "$LOG_FILE" 2>&1

    # Check if node is still running
    NODE_COUNT=$(ps aux | grep btpc_node | grep -v grep | wc -l)
    if [ "$NODE_COUNT" -lt 1 ]; then
        echo "⚠️  CRITICAL: Node stopped running!" | tee -a "$LOG_FILE"
        exit 1
    fi

    # Sleep until next check
    if [ $i -lt $ITERATIONS ]; then
        sleep $INTERVAL
    fi
done

echo "" | tee -a "$LOG_FILE"
echo "=============================================" | tee -a "$LOG_FILE"
echo "BTPC 24-Hour Stress Test Completed" | tee -a "$LOG_FILE"
echo "End Time: $(date '+%Y-%m-%d %H:%M:%S')" | tee -a "$LOG_FILE"
echo "=============================================" | tee -a "$LOG_FILE"

# Final summary
echo "" | tee -a "$LOG_FILE"
echo "=== FINAL SUMMARY ===" | tee -a "$LOG_FILE"
$MONITOR_SCRIPT | tee -a "$LOG_FILE"