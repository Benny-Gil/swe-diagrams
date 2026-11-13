#!/usr/bin/env python3
"""Write the large synthetic fixtures. The output is deterministic.

usage: python3 spec/gen_bench.py spec/fixtures
"""
import sys

def lcg(seed):
    state = seed
    while True:
        state = (state * 1103515245 + 12345) % (1 << 31)
        yield state >> 8

def arch(groups, per_group, edges, seed):
    rnd = lcg(seed)
    status = ['', ' done', ' wip', ' plan']
    kinds = ['', '', '', ' :store', ' :queue']
    out = ['arch "Synthetic system, %d nodes"' % (groups * per_group), '']
    ids = []
    for g in range(groups):
        out.append('group g%d "Domain %d"' % (g, g))
        for k in range(per_group):
            nid = 'n%d_%d' % (g, k)
            ids.append(nid)
            out.append('  %s "Service %d.%d" "pkg/%s"%s%s' % (nid, g, k, nid, kinds[next(rnd) % 5], status[next(rnd) % 4]))
    out.append('')
    for _ in range(edges):
        a = ids[next(rnd) % len(ids)]
        b = ids[next(rnd) % len(ids)]
        if a == b:
            continue
        arrow = '-->' if next(rnd) % 4 == 0 else '->'
        label = ' "call %d"' % (next(rnd) % 90) if next(rnd) % 2 == 0 else ''
        out.append('%s %s %s%s%s' % (a, arrow, b, label, status[next(rnd) % 4]))
    return '\n'.join(out) + '\n'

def seq(parts, messages, seed):
    rnd = lcg(seed)
    out = ['seq "Synthetic sequence, %d messages"' % messages, '']
    for p in range(parts):
        out.append('p%d "Participant %d"' % (p, p))
    for m in range(messages):
        a = next(rnd) % parts
        b = next(rnd) % parts
        arrow = '-->' if next(rnd) % 3 == 0 else '->'
        out.append('p%d %s p%d "message %d"' % (a, arrow, b, m))
    return '\n'.join(out) + '\n'

root = sys.argv[1]
open(root + '/bench-arch-medium.sd', 'w').write(arch(6, 6, 50, 7))
open(root + '/bench-arch-large.sd', 'w').write(arch(20, 10, 320, 11))
open(root + '/bench-seq-large.sd', 'w').write(seq(12, 200, 3))
