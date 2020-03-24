#!/usr/bin/env python3

import matplotlib.pyplot as plt
import numpy as np
import sys

filename = sys.argv[1]

# [[(addr, count, age)]]
data = []
per_addr = {}

with open(filename, "r") as f:
    tmp = []
    for line in f.readlines():
        if line.strip() == "===":
            data.append(tmp)
            tmp = []
        else:
            split = line.split()
            tmp.append((split[0], int(split[1]), int(split[2])))

            if split[0] not in per_addr:
                per_addr[split[0]] = [(0, None)]

# Collect info over time per-address
for dump in data:
    visited = set()
    for (addr, count, age) in dump:
        per_addr[addr].append((count, age))
        visited.add(addr)

    # fill in blanks for the others
    for not_visited in set(per_addr.keys()) - visited:
        per_addr[not_visited].append((0, None))

#data = np.array(data)
#print(per_addr)

for (addr, data) in per_addr.items():
    counts = [d[0] for d in data]
    counts = np.diff(counts)
    plt.plot(counts, label=addr)

plt.yscale('symlog')
plt.ylabel('Derivative of #accesses to huge page')
plt.xlabel('Total number of memory accesses (# periods)')

plt.show()
