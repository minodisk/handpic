// Computes the focus peaking overlay off the main thread: a Sobel over the
// millions of device pixels of a full-window preview takes several frames.
import { peaking } from "./peaking.js";

interface PeakingRequest {
  tag: number;
  pixels: ArrayBuffer;
  width: number;
  height: number;
}

// See `worker.ts`: the worker global scope is declared by hand.
interface WorkerScope {
  addEventListener(type: "message", listener: (event: MessageEvent<PeakingRequest>) => void): void;
  postMessage(message: unknown, transfer?: Transferable[]): void;
}

const ctx = self as unknown as WorkerScope;

ctx.addEventListener("message", (event: MessageEvent<PeakingRequest>) => {
  const { tag, pixels, width, height } = event.data;
  const startedAt = performance.now();
  const overlay = peaking(new Uint8ClampedArray(pixels), width, height);
  const computeMs = performance.now() - startedAt;
  ctx.postMessage({ tag, overlay: overlay.buffer, width, height, computeMs }, [overlay.buffer]);
});
