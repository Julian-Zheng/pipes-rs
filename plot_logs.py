import os
import matplotlib.pyplot as plt

if not os.path.exists("run.log"):
    print("Error: 'run.log' not found!")
    exit(1)

drop_rates = []

with open("run.log", "r") as f:
    for line in f:
        if "Effective frame drop rate:" in line:
            parts = line.split("Effective frame drop rate:")
            if len(parts) > 1:
                rate_str = parts[1].strip().replace("%", "")
                try:
                    drop_rates.append(float(rate_str))
                except ValueError:
                    pass

print(f"Parsed {len(drop_rates)} valid drop rate points.")

if drop_rates:
    plt.figure(figsize=(9, 5))
    plt.plot(drop_rates, marker="o", linestyle="-", color="#2563eb", linewidth=2, markersize=4)
    plt.title("dora-lite Simulation: Frame Drop Rate Over Time")
    plt.xlabel("Checkpoint Step")
    plt.ylabel("Effective Frame Drop Rate (%)")
    plt.grid(True, linestyle="--", alpha=0.6)
    plt.tight_layout()
    plt.savefig("drop_rate_graph.png", dpi=300)
    print("Success! Graph saved as 'drop_rate_graph.png'.")
else:
    print("No drop rates found.")