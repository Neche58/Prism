import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import Fastify, { type FastifyInstance } from "fastify";

// ---------------------------------------------------------------------------
// Mock the BullMQ queue — no Redis instance is required to test the routes.
// ---------------------------------------------------------------------------
vi.mock("../workers/queue", () => ({
  replayQueue: {
    add: vi.fn(),
    getJob: vi.fn(),
  },
}));

const { replayQueue } = await import("../workers/queue");
const { replayRoutes } = await import("../routes/replay");

const queue = replayQueue as unknown as {
  add: ReturnType<typeof vi.fn>;
  getJob: ReturnType<typeof vi.fn>;
};

/** Minimal stand-in for a BullMQ `Job` returned by `queue.getJob()`. */
function fakeJob(overrides: {
  id?: string;
  state?: string;
  progress?: number | Record<string, unknown>;
  returnvalue?: unknown;
  failedReason?: string;
} = {}) {
  const {
    id = "42",
    state = "waiting",
    progress = 0,
    returnvalue,
    failedReason = "",
  } = overrides;

  return {
    id,
    progress,
    returnvalue,
    failedReason,
    getState: vi.fn(async () => state),
  };
}

let app: FastifyInstance;

beforeEach(async () => {
  queue.add.mockReset();
  queue.getJob.mockReset();

  app = Fastify();
  await app.register(replayRoutes, { prefix: "/api" });
  await app.ready();
});

afterEach(async () => {
  await app.close();
});

describe("Replay routes — POST /api/replay", () => {
  it("enqueues the transaction and returns the job id", async () => {
    queue.add.mockResolvedValue({ id: "1" });

    const res = await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { txHash: "abc123", network: "testnet" },
    });

    expect(res.statusCode).toBe(202);
    expect(res.json()).toEqual({ jobId: "1", status: "queued" });
    expect(queue.add).toHaveBeenCalledTimes(1);
    expect(queue.add).toHaveBeenCalledWith("replay", {
      txHash: "abc123",
      network: "testnet",
    });
  });

  it("trims whitespace from the payload before enqueueing", async () => {
    queue.add.mockResolvedValue({ id: "2" });

    await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { txHash: "  abc123  ", network: " mainnet " },
    });

    expect(queue.add).toHaveBeenCalledWith("replay", {
      txHash: "abc123",
      network: "mainnet",
    });
  });

  it("rejects a payload without a txHash", async () => {
    const res = await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { network: "testnet" },
    });

    expect(res.statusCode).toBe(400);
    expect(queue.add).not.toHaveBeenCalled();
  });

  it("rejects a payload without a network", async () => {
    const res = await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { txHash: "abc123" },
    });

    expect(res.statusCode).toBe(400);
    expect(queue.add).not.toHaveBeenCalled();
  });

  it("rejects whitespace-only values that pass the schema", async () => {
    const res = await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { txHash: "   ", network: "testnet" },
    });

    expect(res.statusCode).toBe(400);
    expect(res.json()).toEqual({
      error: "txHash and network must be non-empty strings",
    });
    expect(queue.add).not.toHaveBeenCalled();
  });

  it("does not report success when BullMQ returns a job without an id", async () => {
    queue.add.mockResolvedValue({ id: undefined });

    const res = await app.inject({
      method: "POST",
      url: "/api/replay",
      payload: { txHash: "abc123", network: "testnet" },
    });

    expect(res.statusCode).toBe(500);
    expect(res.json()).toEqual({ error: "Failed to enqueue replay job" });
  });
});

describe("Replay routes — GET /api/replay/:jobId", () => {
  it("returns 404 when the job does not exist", async () => {
    queue.getJob.mockResolvedValue(undefined);

    const res = await app.inject({
      method: "GET",
      url: "/api/replay/missing",
    });

    expect(queue.getJob).toHaveBeenCalledWith("missing");
    expect(res.statusCode).toBe(404);
    expect(res.json()).toEqual({
      status: "not_found",
      message: "Replay job 'missing' does not exist or has expired.",
    });
  });

  it("returns the waiting state while the job is queued", async () => {
    queue.getJob.mockResolvedValue(fakeJob({ state: "waiting" }));

    const res = await app.inject({ method: "GET", url: "/api/replay/42" });
    const body = res.json();

    expect(res.statusCode).toBe(200);
    expect(body).toEqual({ jobId: "42", status: "waiting", progress: 0 });
    expect(body).not.toHaveProperty("result");
    expect(body).not.toHaveProperty("error");
  });

  it("returns the active state with progress", async () => {
    queue.getJob.mockResolvedValue(fakeJob({ state: "active", progress: 60 }));

    const res = await app.inject({ method: "GET", url: "/api/replay/42" });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toEqual({
      jobId: "42",
      status: "active",
      progress: 60,
    });
  });

  it("returns the replay result once the job is completed", async () => {
    const returnvalue = {
      report: { error_code: 2, error_name: "ExceededLimit" },
      explanation: { summary: "Storage limit exceeded" },
    };
    queue.getJob.mockResolvedValue(
      fakeJob({ state: "completed", progress: 100, returnvalue }),
    );

    const res = await app.inject({ method: "GET", url: "/api/replay/42" });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toEqual({
      jobId: "42",
      status: "completed",
      progress: 100,
      result: returnvalue,
    });
  });

  it("returns the failure reason when the job failed", async () => {
    queue.getJob.mockResolvedValue(
      fakeJob({ state: "failed", failedReason: "replay crashed" }),
    );

    const res = await app.inject({ method: "GET", url: "/api/replay/42" });

    expect(res.statusCode).toBe(200);
    expect(res.json()).toEqual({
      jobId: "42",
      status: "failed",
      progress: 0,
      error: "replay crashed",
    });
  });

  it("reports the queue state rather than assuming pending", async () => {
    const job = fakeJob({ state: "delayed" });
    queue.getJob.mockResolvedValue(job);

    const res = await app.inject({ method: "GET", url: "/api/replay/42" });

    expect(job.getState).toHaveBeenCalledTimes(1);
    expect(res.json().status).toBe("delayed");
  });
});
