// Decodes preview JPEG bytes into an `ImageBitmap`. `invoke` is unavailable
// here, so the main thread fetches the bytes and this worker only decodes.
import { fitWithin, jpegSize } from "./decode.js";

interface DecodeRequest {
  seq: number;
  jpeg: ArrayBuffer;
  maxPixels: number | null;
}

// The DOM lib has no worker global scope type, and pulling in the "webworker"
// lib alongside "dom" clashes on the shared globals, so declare what is used.
interface WorkerScope {
  addEventListener(type: "message", listener: (event: MessageEvent<DecodeRequest>) => void): void;
  postMessage(message: unknown, transfer?: Transferable[]): void;
}

const ctx = self as unknown as WorkerScope;

ctx.addEventListener("message", (event: MessageEvent<DecodeRequest>) => {
  const { seq, jpeg, maxPixels } = event.data;
  decode(jpeg, maxPixels)
    .then(({ bitmap, decodeMs }) => {
      ctx.postMessage({ seq, bitmap, decodeMs }, [bitmap]);
    })
    .catch((err: unknown) => {
      ctx.postMessage({ seq, error: String(err) });
    });
});

// Only a preview over `maxPixels` gets resize options; everything else takes
// the plain call, as does a JPEG whose size cannot be read. `decodeMs` is a
// duration because this worker's clock has its own origin.
async function decode(
  jpeg: ArrayBuffer,
  maxPixels: number | null,
): Promise<{ bitmap: ImageBitmap; decodeMs: number }> {
  const blob = new Blob([jpeg], { type: "image/jpeg" });
  const size = maxPixels === null ? null : jpegSize(new Uint8Array(jpeg));
  const fit =
    size === null || maxPixels === null ? null : fitWithin(size.width, size.height, maxPixels);
  const startedAt = performance.now();
  const bitmap = await (fit === null
    ? createImageBitmap(blob)
    : createImageBitmap(blob, { ...fit, resizeQuality: "high" }));
  return { bitmap, decodeMs: performance.now() - startedAt };
}
