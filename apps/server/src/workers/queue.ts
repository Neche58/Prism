import { Queue } from "bullmq";
import { config } from "../config";

/**
 * Payload accepted by the replay worker.
 *
 * `report` is optional because the route only knows the transaction to replay;
 * it is populated upstream when a diagnostic report already exists.
 */
export interface ReplayJobData {
  txHash: string;
  network: string;
  report?: unknown;
}

/** Value a replay job resolves to once BullMQ marks it `completed`. */
export interface ReplayJobResult {
  report: unknown;
  explanation?: unknown;
}

export const replayQueue = new Queue<ReplayJobData, ReplayJobResult, "replay">(
  "replay",
  {
    connection: { url: config.redisUrl },
  },
);
