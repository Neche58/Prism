import type { FastifyInstance } from "fastify";
import { replayQueue } from "../workers/queue";

// ---------------------------------------------------------------------------
// Request / response schemas
// ---------------------------------------------------------------------------

const createReplaySchema = {
  body: {
    type: "object",
    required: ["txHash", "network"],
    properties: {
      txHash: { type: "string", minLength: 1, maxLength: 128 },
      network: { type: "string", minLength: 1, maxLength: 32 },
    },
  },
} as const;

const jobIdParamSchema = {
  params: {
    type: "object",
    required: ["jobId"],
    properties: {
      jobId: { type: "string", minLength: 1 },
    },
  },
} as const;

interface CreateReplayBody {
  txHash: string;
  network: string;
}

interface JobIdParams {
  jobId: string;
}

// ---------------------------------------------------------------------------
// Route plugin
// ---------------------------------------------------------------------------

export async function replayRoutes(app: FastifyInstance) {
  // ------------------------------------------------------------------
  // POST /api/replay
  // Validate the transaction reference and hand it to the BullMQ queue.
  // The request returns as soon as the job is enqueued; clients then poll
  // GET /api/replay/:jobId for progress and the replay result.
  // ------------------------------------------------------------------
  app.post<{ Body: CreateReplayBody }>(
    "/replay",
    { schema: createReplaySchema },
    async (request, reply) => {
      // `minLength` rejects empty strings but not whitespace-only values, so
      // normalise before enqueueing to keep job data meaningful.
      const txHash = request.body.txHash.trim();
      const network = request.body.network.trim();

      if (!txHash || !network) {
        return reply.status(400).send({
          error: "txHash and network must be non-empty strings",
        });
      }

      const job = await replayQueue.add("replay", { txHash, network });
      const jobId = job.id;

      if (!jobId) {
        // BullMQ normally assigns an id on add; without one the client would
        // have no way to poll the job it just created.
        request.log.error({ txHash, network }, "Replay job has no id");
        return reply.status(500).send({ error: "Failed to enqueue replay job" });
      }

      request.log.info({ jobId, txHash, network }, "Replay job enqueued");

      return reply.status(202).send({ jobId, status: "queued" });
    },
  );

  // ------------------------------------------------------------------
  // GET /api/replay/:jobId
  // Report the live BullMQ state plus progress and the result/error once
  // the job has finished.
  // ------------------------------------------------------------------
  app.get<{ Params: JobIdParams }>(
    "/replay/:jobId",
    { schema: jobIdParamSchema },
    async (request, reply) => {
      const { jobId } = request.params;

      const job = await replayQueue.getJob(jobId);

      if (!job) {
        return reply.status(404).send({
          status: "not_found",
          message: `Replay job '${jobId}' does not exist or has expired.`,
        });
      }

      const status = await job.getState();

      return reply.send({
        jobId,
        status,
        progress: job.progress ?? 0,
        ...(status === "completed" ? { result: job.returnvalue } : {}),
        ...(status === "failed" ? { error: job.failedReason } : {}),
      });
    },
  );
}
