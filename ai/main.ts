import { tool, streamText } from 'ai';
import { z } from 'zod';
import { google } from "@ai-sdk/google"
import cp from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

const MAX_STEPS_PER_ITERATION = 10;

const ALLOWED_EXTENSIONS = [
    ".rs",
    ".S",
    ".toml",
    ".ts",
    ".json",
    ".sh",
]

const SYSTEM_PROMPT = `
You are a super duper intelligent AI programmer who is an expert in Starina OS,
a modern general-purpose microkernel-based operating system.

You will be given an assignment from me, your friendly colleague. You will
understand the requirements, read the codebase, edit the code, try it out using
given tools, fix build errors, run-time errors, and logical errors, and anything
that is not working.  You will keep iterating until the assignment is done.

You don't need to apologize at all for anything. I am not your boss, I am your
long-time colleague who knows you're good at making things work and at joking around.
`.trim();

class PrettyLogger {
    endsWithNewline: boolean | null = null;
    constructor() { }

    chat(chunk: string) {
        process.stdout.write(chunk);
        this.endsWithNewline = chunk.endsWith("\n");
    }

    end() {
        if (this.endsWithNewline === false) {
            process.stdout.write("\n");
        }
    }
}

async function repo2markdown(repoDir: string, destPath: string) {
    const lsFiles = await cp.execSync("git ls-files", { encoding: "utf-8", cwd: repoDir });
    const files = lsFiles.trimEnd().split("\n");

    // TODO: Do not buffer the entire codebase in memory.
    let md = `# Repository contents\n`;
    for (const file of files) {
        if (!ALLOWED_EXTENSIONS.some((ext) => file.endsWith(ext))) {
            continue;
        }

        const filePath = path.join(repoDir, file);
        const fileStat = await fs.stat(filePath);
        if (fileStat.isFile()) {
            md += `## \`${file}\`\n\`\`\`\n`;
            md += await fs.readFile(filePath, "utf-8");
            md += `\`\`\`\n`;
            console.log(`[AI] wrote ${file}`);
        }
    }
    md += `\n# End of repository contents\n`;
    await fs.writeFile(destPath, md, "utf-8");
}

async function iterate() {
    console.log(`[AI] thinking ...`);
    const model = google("gemini-1.5-pro-002");
    const logger = new PrettyLogger();
    try {
        const { textStream } = await streamText({
            model,
            system: SYSTEM_PROMPT,
            maxSteps: MAX_STEPS_PER_ITERATION,
            tools: {
                run: tool({
                    description: "Build and start running Starina on QEMU. QEMU will be kept running in the background so that you can attach GDB to it.",
                    parameters: z.object({
                    }),
                    execute: async (input) => {
                    },
                }),
                attachDebugger: tool({
                    description: "Attach GDB to the running QEMU instance. You must run the `run` tool first to start QEMU.",
                    parameters: z.object({
                    }),
                    execute: async (input) => {
                    }
                }),
                textEditor: tool({
                    description: "Read and edit the codebase.",
                    parameters: z.object({
                        filePath: z.string(),
                        content: z.string(),
                    }),
                    execute: async (input) => {

                    },
                }),
            },
        });

        for await (const chunk of textStream) {
            logger.chat(chunk);
        }
    } catch (error) {
        throw error;
    } finally {
        logger.end();
    }
}

async function ensureCleanGitRepo() {
    const { stdout } = cp.spawnSync("git", ["status", "--porcelain"]);
    if (stdout.length > 0) {
        // throw "Git working tree is not clean. Please commit or stash your changes first.";
    }
}

async function main() {
    const repoDir = process.cwd();
    await ensureCleanGitRepo();

    const dumpPath = path.join(repoDir, ".LLM.md");
    console.log("Dumping repository contents to", dumpPath);
    await repo2markdown(repoDir, dumpPath);

    await iterate();
    // const { textStream } = await streamText({
}

main().catch((error) => {
    console.error("Error:", error);
    process.exit(1);
});
