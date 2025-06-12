# Starina

## Rules

- Keep the changes minimal and concise.
- Follow the conventions and idioms of the codebase.
- Keep the code clean and easy to understand.
- Do not write code comments if it's obvious what the code does.
- Prefer short identifiers.

## Tools

- You can use ripgrep (`rg`) to search for code.

## Build

Prefer this over `cargo check`:

```
CHECK_ONLY=1 ./run.sh
```

## Run

This will start QEMU. It'll be kept running until you terminate it:

```
./run.sh
```
