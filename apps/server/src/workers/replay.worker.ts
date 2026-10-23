import { Worker } from "bullmq";
import { config } from "../config";
import { explainError } from "../ai";
import type { ReplayJobData, ReplayJobResult } from "./queue";

// The third type parameter pins the worker to the same queue name the producer
// uses, so a rename on either side becomes a compile error.
const worker = new Worker<ReplayJobData, ReplayJobResult, "replay">(
  "replay",
  async (job) => {
    console.log(`Processing replay job ${job.id}: ${job.data.txHash}`);

    // After replay produces a diagnostic report, optionally generate an
    // AI explanation.  `job.data.report` is populated by the replay engine
    // once the trace is complete.
    if (job.data.report && config.ai.apiKey) {
      try {
        const explanation = await explainError(
          JSON.stringify(job.data.report),
          job.data.txHash,
          job.data.network || "mainnet",
        );
        console.log(
          `AI explanation generated for job ${job.id} (${explanation.tokens_used} tokens, cached=${explanation.cached})`,
        );
        return { report: job.data.report, explanation };
      } catch (err) {
        // AI failure is non-fatal — the replay result is still valid
        console.warn(`AI explanation failed for job ${job.id}:`, err);
        return { report: job.data.report, explanation: null };
      }
    }

    return { report: job.data.report ?? null };
  },
  { connection: { url: config.redisUrl } },
);

export default worker;
