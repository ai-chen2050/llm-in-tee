import matplotlib.pyplot as plt

# Raw data
data = [
    {"key": 1, "bincode deserialize": 14.55, "verify clock": 4773.788, "Update clock": 0.33, "Gen clock proof": 1571.576, "Total once time": 7367.268},
    {"key": 4, "bincode deserialize": 14.39, "verify clock": 4789.289, "Update clock": 0.41, "Gen clock proof": 1151.385, "Total once time": 6956.027},
    {"key": 16, "bincode deserialize": 15.14, "verify clock": 4804.879, "Update clock": 0.87, "Gen clock proof": 1356.135, "Total once time": 7200.728},
    {"key": 64, "bincode deserialize": 19.82, "verify clock": 4774.939, "Update clock": 1.45, "Gen clock proof": 1189.694, "Total once time": 7006.328},
    {"key": 256, "bincode deserialize": 30.72, "verify clock": 4830.389, "Update clock": 7.95, "Gen clock proof": 1164.215, "Total once time": 7060.418},
    {"key": 1024, "bincode deserialize": 203.421, "verify clock": 4850.049, "Update clock": 27.47, "Gen clock proof": 1198.744, "Total once time": 7390.719},
    {"key": 4096, "bincode deserialize": 465.902, "verify clock": 5068.49, "Update clock": 98.78, "Gen clock proof": 1431.905, "Total once time": 8384.523},
    {"key": 16384, "bincode deserialize": 1878.907, "verify clock": 6009.014, "Update clock": 381.702, "Gen clock proof": 2346.169, "Total once time": 12759.83},
    {"key": 65536, "bincode deserialize": 8028.811, "verify clock": 9684.848, "Update clock": 1518.026, "Gen clock proof": 6093.884, "Total once time": 30752.481},
]

keys = [item["key"] for item in data]
bincode_deserialize = [item["bincode deserialize"] for item in data]
verify_clock = [item["verify clock"] for item in data]
update_clock = [item["Update clock"] for item in data]
gen_clock_proof = [item["Gen clock proof"] for item in data]
total_once_time = [item["Total once time"] for item in data]

# Scatter diagram
plt.figure(figsize=(10, 6))
plt.scatter(keys, bincode_deserialize, label='bincode deserialize')
plt.scatter(keys, verify_clock, label='verify clock')
plt.scatter(keys, update_clock, label='Update clock')
plt.scatter(keys, gen_clock_proof, label='Gen clock proof')
plt.scatter(keys, total_once_time, label='Total once time', marker='x')
plt.xscale('log')
plt.yscale('log')
plt.xlabel('Key')
plt.ylabel('Time (µs)')
plt.legend()
plt.title('Scatter Plot of Function Execution Times')
plt.show()

# Line chart
plt.figure(figsize=(10, 6))
plt.plot(keys, bincode_deserialize, label='bincode deserialize')
plt.plot(keys, verify_clock, label='verify clock')
plt.plot(keys, update_clock, label='Update clock')
plt.plot(keys, gen_clock_proof, label='Gen clock proof')
plt.plot(keys, total_once_time, label='Total once time', marker='x')
plt.xscale('log')
plt.yscale('log')
plt.xlabel('Key')
plt.ylabel('Time (µs)')
plt.legend()
plt.title('Line Plot of Function Execution Times')
plt.show()

# Pie chart
# Clock keys
keys_to_plot = [4, 64, 1024, 4096, 16384, 65536]
data_to_plot = [entry for entry in data if entry["key"] in keys_to_plot]

fig, axes = plt.subplots(2, 3, figsize=(18, 10))

# draw six pie chart on one page
for i, entry in enumerate(data_to_plot):
    labels = ['bincode deserialize', 'verify clock', 'Update clock', 'Gen clock proof']
    sizes = [entry['bincode deserialize'], entry['verify clock'], entry['Update clock'], entry['Gen clock proof']]
    ax = axes[i//3, i%3]
    ax.pie(sizes, labels=labels, autopct='%1.1f%%', startangle=140)
    ax.set_title(f'Time Distribution for Key = {entry["key"]}')

plt.suptitle('Time Distribution for Selected Keys', fontsize=16)
plt.tight_layout(rect=[0, 0, 1, 0.96])
plt.show()