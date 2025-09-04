// index.js
// Usage (event):
// { "algo": "mix" | "fib" | "primes" | "matmul" | "hash", "targetMs": 1500 }
// Defaults: algo="mix", targetMs=1500

const crypto = require('crypto');

module.exports.handler = async (event = {}) => {
  const algo = (event.algo || 'mix').toLowerCase();
  const targetMs = clampInt(event.targetMs ?? 1500, 50, 60_000); // 50ms..60s
  const start = Date.now();

  let stats = {
    fib: 0,
    primes: 0,
    matmul: 0,
    hash: 0,
  };

  // Run until wall clock target reached
  while ((Date.now() - start) < targetMs) {
    switch (algo) {
      case 'fib':
        stats.fib ^= fib(42); // ~heavy-ish
        break;
      case 'primes':
        stats.primes ^= sumPrimesUpTo(20000);
        break;
      case 'matmul':
        stats.matmul ^= matmulChecksum(40); // 40x40 dense multiply
        break;
      case 'hash':
        stats.hash ^= hashSpin(20000);
        break;
      case 'mix':
      default:
        // A small mix to vary CPU characteristics
        stats.fib ^= fib(41);
        stats.primes ^= sumPrimesUpTo(15000);
        stats.matmul ^= matmulChecksum(32);
        stats.hash ^= hashSpin(12000);
        break;
    }
  }

  const elapsed = Date.now() - start;
  return {
    ok: true,
    algo,
    targetMs,
    elapsedMs: elapsed,
    // Checksums make the work observable & prevent dead-code elimination
    checksums: stats,
  };
};

// -------- helpers --------

function clampInt(v, min, max) {
  v = (v | 0);
  if (v < min) return min;
  if (v > max) return max;
  return v;
}

// Iterative Fibonacci to avoid recursion overhead
function fib(n) {
  let a = 0, b = 1;
  for (let i = 0; i < n; i++) {
    const t = a + b;
    a = b; b = t;
  }
  return a;
}

// Sieve of Eratosthenes; returns sum of primes ≤ N
function sumPrimesUpTo(N) {
  const sieve = new Uint8Array(N + 1);
  let sum = 0;
  for (let i = 2; i <= N; i++) {
    if (sieve[i] === 0) {
      sum += i;
      for (let j = i * 2; j <= N; j += i) sieve[j] = 1;
    }
  }
  return sum;
}

// Deterministic PRNG for matrix fill
function lcg(seed) {
  let x = seed >>> 0;
  return () => ((x = (x * 1664525 + 1013904223) >>> 0) / 2**32);
}

// Dense matmul (n x n), return checksum of result to keep it CPU-bound
function matmulChecksum(n) {
  const rnd = lcg(n * 2654435761);
  // Allocate small to avoid huge memory; reuse arrays across loops
  const A = new Float64Array(n * n);
  const B = new Float64Array(n * n);
  const C = new Float64Array(n * n);

  for (let i = 0; i < n * n; i++) { A[i] = rnd(); B[i] = rnd(); }

  for (let i = 0; i < n; i++) {
    for (let k = 0; k < n; k++) {
      const aik = A[i * n + k];
      for (let j = 0; j < n; j++) {
        C[i * n + j] += aik * B[k * n + j];
      }
    }
  }
  // Simple checksum to force reading the result
  let s = 0;
  for (let i = 0; i < C.length; i += Math.max(1, (n >> 2))) s ^= Math.floor(C[i] * 1e6);
  return s | 0;
}

// Tight hashing loop; returns final digest nibble checksum
function hashSpin(iters) {
  let acc = Buffer.allocUnsafe(32).fill(0);
  for (let i = 0; i < iters; i++) {
    const h = crypto.createHash('sha256');
    h.update(acc);
    h.update(intToBuf(i));
    acc = h.digest();
  }
  // fold to small int
  let s = 0;
  for (let i = 0; i < acc.length; i++) s ^= acc[i];
  return s;
}

function intToBuf(i) {
  const b = Buffer.allocUnsafe(4);
  b.writeUInt32BE(i >>> 0, 0);
  return b;
}