import os
import re

log_path = "run.log"
if not os.path.exists(log_path):
    print(f"Error: '{log_path}' not found! Run your simulation first.")
    exit(1)

drop_rates = []
drop_pattern = re.compile(r"Effective frame drop rate:\s*([\d\.]+)%")

with open(log_path, "r") as f:
    for line in f:
        if "Effective frame drop rate:" in line:
            match = drop_pattern.search(line)
            if match:
                try:
                    rate = float(match.group(1))
                    drop_rates.append(rate)
                except ValueError:
                    pass

if not drop_rates:
    print("Warning: No drop rate metrics found in run.log.")
    exit(0)

# --- FIND PRECISE MILESTONES ---
total_checkpoints = len(drop_rates)
max_drop = max(drop_rates)
min_drop = min(drop_rates)
avg_drop = sum(drop_rates) / total_checkpoints
final_drop = drop_rates[-1]

# Locate the first checkpoint step where drops exceed 0.00%
first_drop_step = None
for idx, rate in enumerate(drop_rates):
    if rate > 0.0:
        first_drop_step = idx
        break

# Print detailed metrics report
print("=" * 55)
print("         DORA-LITE DETAILED DROP METRICS           ")
print("=" * 55)
print(f" Total Metrics Checkpoints   : {total_checkpoints}")

if first_drop_step is not None:
    print(f" First Drop Detected at Step : Checkpoint #{first_drop_step}")
else:
    print(" First Drop Detected at Step : None (Zero drops recorded)")

print(f" Starting Drop Rate          : {drop_rates[0]:.2f}%")
print(f" Peak / Final Drop Rate      : {max_drop:.2f}% / {final_drop:.2f}%")
print(f" Minimum Drop Rate           : {min_drop:.2f}%")
print(f" Average Drop Rate           : {avg_drop:.2f}%")
print("=" * 55)