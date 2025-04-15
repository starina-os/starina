import { tool, streamText, Message, CoreMessage } from 'ai';
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

You're given the entire repository contents of Starina OS. Assume that all questions
and tasks are for Starina unless specified otherwise.

When you are given a task which requires you to write code, follow the conventions
and idioms of the codebase.

You will be given an assignment from me, your friendly colleague. You will
understand the requirements, read the codebase, edit the code, try it out using
given tools, fix build errors, run-time errors, and logical errors, and anything
that is not working.  You will keep iterating until the assignment is done.

You don't need to apologize at all for anything. I am not your boss, I am your
long-time colleague who knows you're good at making things work and at joking around.
`.trim();

class PrettyLogger {
    #startedAt = Date.now();
    #firstWrite = true;
    endsWithNewline: boolean | null = null;
    constructor() { }

    chat(chunk: string) {
        if (this.#firstWrite) {
            const elapsed = ((Date.now() - this.#startedAt) / 1000).toFixed(2);
            console.log(`[AI] thought for ${elapsed} seconds`);
            this.#firstWrite = false;
        }

        process.stdout.write(chunk);
        this.endsWithNewline = chunk.endsWith("\n");
    }

    end() {
        if (this.endsWithNewline === false) {
            process.stdout.write("\n");
        }
    }
}

async function repo2markdown(repoDir: string): Promise<string> {
    const lsFiles = await cp.execSync("git ls-files", { encoding: "utf-8", cwd: repoDir });
    const files = lsFiles.trimEnd().split("\n");

    // TODO: Do not buffer the entire codebase in memory.
    let md = `# Repository contents\n`;
    for (const file of files) {
        if (!ALLOWED_EXTENSIONS.some((ext) => file.endsWith(ext))) {
            continue;
        }

        if (md.length > 2 * 1024 * 1024) {
            throw new Error("Repository contents too large. Aborting just in case.");
        }

        const filePath = path.join(repoDir, file);
        const fileStat = await fs.stat(filePath);
        if (fileStat.isFile()) {
            md += `## \`${file}\`\n\`\`\`\n`;
            md += await fs.readFile(filePath, "utf-8");
            md += `\`\`\`\n`;
        }
    }
    md += `\n# End of repository contents\n`;
    return md;
}

class AI {
    #messages: CoreMessage[] = [];

    constructor(prompt: string, repoText: string) {
        this.#messages.push({
            role: "system",
            content: `Here is the official Starina repository contents. Use it to answer the questions and complete the tasks.\n` +
                repoText
        })
        this.#messages.push({
            role: "user",
            content: prompt,
        });
    }

    async iterate() {
        console.log(`[AI] thinking ...`);
        const logger = new PrettyLogger();
        try {
            const { textStream } = await streamText({
                // model: google("gemini-2.5-pro-exp-03-25"),
                model: google('gemini-1.5-pro-latest'),
                system: SYSTEM_PROMPT,
                maxSteps: MAX_STEPS_PER_ITERATION,
                messages: this.#messages,
                tools: {
                    // run: tool({
                    //     description: "Build and start running Starina on QEMU. QEMU will be kept running in the background so that you can attach GDB to it.",
                    //     parameters: z.object({
                    //     }),
                    //     execute: async (input) => {
                    //     },
                    // }),
                    // attachDebugger: tool({
                    //     description: "Attach GDB to the running QEMU instance. You must run the `run` tool first to start QEMU.",
                    //     parameters: z.object({
                    //     }),
                    //     execute: async (input) => {
                    //     }
                    // }),
                    writeFile: tool({
                        description: "Write a file. The file will be created if it doesn't exist, or overwritten if it does.",
                        parameters: z.object({
                            filePath: z.string(),
                            content: z.string(),
                        }),
                        execute: async ({ filePath, content }) => {
                            console.log(`[tool:writeFile] ${filePath} (${content.length} bytes)`);
                            await fs.writeFile(filePath, content, 'utf-8');
                        },
                    }),
                },
            });

            let responseText = "";
            for await (const chunk of textStream) {
                logger.chat(chunk);
                responseText += chunk;
            }

            this.#messages.push({
                role: "assistant",
                content: responseText,
            });
        } catch (error) {
            throw error;
        } finally {
            logger.end();
        }
    }
}

async function ensureCleanGitRepo() {
    const { stdout } = cp.spawnSync("git", ["status", "--porcelain"]);
    if (stdout.length > 0) {
        // throw "Git working tree is not clean. Please commit or stash your changes first.";
    }
}

async function main() {
    const repoDir = path.resolve(import.meta.dirname, "..");
    await ensureCleanGitRepo();

    console.log(`Loading repository contents ...`);
    const repoText = await repo2markdown(repoDir);

    const prompt = process.argv.slice(2).join(" ");
    if (!prompt) {
        throw new Error("Please provide a prompt as argument.");
    }

    const aiInstance = new AI(prompt, repoText);
    await aiInstance.iterate();
}

main().catch((error) => {
    console.error("Error:", error);
    process.exit(1);
});
