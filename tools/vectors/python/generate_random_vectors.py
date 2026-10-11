import json
import random
import struct
import sys

SEEDS = [0, 1, -1, 42, -42, 123456789, 2**31 - 1, 2**32 - 1, 2**32, 2**32 + 7, -(2**40) - 3, 2**63 - 1, 2**64 + 12345, 2**100 + 99]


def float_bits(value):
    return str(struct.unpack("<Q", struct.pack("<d", value))[0])


def random_case(seed):
    rng = random.Random(seed)
    case = {"seed": str(seed)}
    case["u32"] = [rng.getrandbits(32) for _ in range(8)]
    case["random"] = [float_bits(rng.random()) for _ in range(8)]
    case["getrandbits"] = [{"bits": bits, "value": str(rng.getrandbits(bits))} for bits in [1, 5, 31, 32, 33, 48, 63, 64]]
    case["randrange"] = [{"start": start, "stop": stop, "value": rng.randrange(start, stop)} for start, stop in [(1, 327), (0, 2), (-50, 50), (1, 268), (0, 1000000007), (5, 6)]]
    case["randint"] = [{"low": low, "high": high, "value": rng.randint(low, high)} for low, high in [(1, 493), (0, 0), (-3, 3), (1, 2**40)]]
    sequence = list(range(100, 117))
    case["choice"] = [rng.choice(sequence) for _ in range(10)]
    shuffled = list(range(25))
    rng.shuffle(shuffled)
    case["shuffle"] = shuffled
    case["sample"] = [{"population": population, "count": count, "value": rng.sample(range(population), count)} for population, count in [(267, 3), (493, 20), (826, 100), (30, 30), (50, 6), (1000, 6), (64, 22), (5, 0)]]
    case["uniform"] = [float_bits(rng.uniform(-2.5, 7.25)) for _ in range(4)]
    case["after"] = rng.getrandbits(32)
    return case


def main():
    output = {"python": sys.version.split()[0], "cases": [random_case(seed) for seed in SEEDS]}
    json.dump(output, sys.stdout, indent=1)
    sys.stdout.write("\n")


main()
