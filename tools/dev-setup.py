#!/usr/bin/env python3
import argparse
import subprocess
import sys

def progress(msg):
    print(f"\033[1;94m==>\033[0m\033[1m {msg}\033[0m")

def error(msg):
    print(f"\033[1;91mError: {msg}\033[0m")
    sys.exit(1)

def try_version_command(argv):
    try:
        subprocess.run(argv, check=True)
    except FileNotFoundError:
        error(f"command '{argv[0]}' not found")

def main():
    parser = argparse.ArgumentParser(description="Set up a development environment")
    args = parser.parse_args()

    progress("Checking build tools")
    try_version_command(["cargo", "--version"])
    try_version_command(["rustup", "--version"])
    try_version_command(["make", "--version"])

    print()
    progress("You're all set! Try 'make run' to try out!")

if __name__ == "__main__":
    main()
