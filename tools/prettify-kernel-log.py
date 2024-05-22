#!/usr/bin/env python3
import argparse
import subprocess
import re

BACKTRACE_REGEX = re.compile(r'#(?P<index>\d+) at (?P<addr>[0-9a-f]+)')

ADDR2LINE_CACHE = {}

def addr2line(executable, addr):
    if addr in ADDR2LINE_CACHE:
        return ADDR2LINE_CACHE[addr]

    cmd = 'llvm-addr2line -e {} {}'.format(executable, hex(addr))
    stdout = subprocess.check_output(cmd, shell=True).decode('utf-8').strip()
    ADDR2LINE_CACHE[addr] = stdout
    return stdout

def prettify_line(executable, line):
    m = BACKTRACE_REGEX.search(line)
    if m:
        index = int(m.group('index'))
        addr = int(m.group('addr'), 16)
        stdout = addr2line(executable, addr)
        line = line.replace(m.group(0), f"{index}: {hex(addr)} {stdout}")
    print(line, end='')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('-e', help='The kernel executable file', default='ftl.elf')
    parser.add_argument('log_file', help='path to log file')
    args = parser.parse_args()

    with open(args.log_file, 'r') as f:
        for l in f:
            prettify_line(args.e, l)

main()
