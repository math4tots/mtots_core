"""
For benchmarking against foo.u

On my mbp,

time python3 samples/foo.py gives me:

total = 49999995000000

real	0m0.601s
user	0m0.585s
sys	0m0.010s
"""

def main():
    total = 0
    for i in range(10 ** 7):
        total = total + i
    print(f'total = {total}')

main()
