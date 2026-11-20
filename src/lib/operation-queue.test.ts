import { describe, expect, it, vi } from 'vitest';
import { OperationQueue } from './operation-queue';

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('OperationQueue', () => {
  it('runs synchronous and asynchronous operations in FIFO order', async () => {
    const queue = new OperationQueue();
    const calls: string[] = [];

    const first = queue.enqueue(() => {
      calls.push('first:start');
      return 1;
    });
    const second = queue.enqueue(async () => {
      calls.push('second:start');
      await Promise.resolve();
      calls.push('second:end');
      return 2;
    });
    const third = queue.enqueue(() => {
      calls.push('third:start');
      return 3;
    });

    await expect(Promise.all([first, second, third])).resolves.toEqual([1, 2, 3]);
    expect(calls).toEqual(['first:start', 'second:start', 'second:end', 'third:start']);
  });

  it('rejects the failed operation without blocking subsequent operations', async () => {
    const queue = new OperationQueue();
    const calls: string[] = [];
    const failure = new Error('I2C failed');

    const first = queue.enqueue(async () => {
      calls.push('first:start');
      await Promise.resolve();
      calls.push('first:throw');
      throw failure;
    });
    const second = queue.enqueue(() => {
      calls.push('second:run');
      return 'recovered';
    });

    await expect(first).rejects.toBe(failure);
    await expect(second).resolves.toBe('recovered');
    expect(calls).toEqual(['first:start', 'first:throw', 'second:run']);

    await expect(queue.enqueue(() => 'after recovery')).resolves.toBe('after recovery');
  });

  it('does not run the next operation before a pending predecessor settles', async () => {
    const queue = new OperationQueue();
    const first = deferred<string>();
    const second = deferred<string>();
    const calls: string[] = [];

    const firstResult = queue.enqueue(() => {
      calls.push('first:start');
      return first.promise;
    });
    const secondResult = queue.enqueue(() => {
      calls.push('second:start');
      return second.promise;
    });
    const thirdResult = queue.enqueue(() => {
      calls.push('third:start');
      return 'third:end';
    });

    await Promise.resolve();
    expect(calls).toEqual(['first:start']);

    second.resolve('second:end');
    await Promise.resolve();
    expect(calls).toEqual(['first:start']);

    first.resolve('first:end');
    await expect(firstResult).resolves.toBe('first:end');
    await expect(secondResult).resolves.toBe('second:end');
    await expect(thirdResult).resolves.toBe('third:end');
    expect(calls).toEqual(['first:start', 'second:start', 'third:start']);
  });

  it('keeps rejections local while many operations are queued concurrently', async () => {
    const queue = new OperationQueue();
    const events: number[] = [];

    const results = Array.from({ length: 12 }, (_, index) => queue.enqueue(async () => {
      await Promise.resolve();
      events.push(index);
      if (index % 3 === 0) throw new Error(`failed ${index}`);
      return index;
    }));

    await expect(Promise.allSettled(results)).resolves.toEqual(
      Array.from({ length: 12 }, (_, index) => index % 3 === 0
        ? { status: 'rejected', reason: expect.objectContaining({ message: `failed ${index}` }) }
        : { status: 'fulfilled', value: index }),
    );
    expect(events).toEqual(Array.from({ length: 12 }, (_, index) => index));
  });

  it('propagates synchronous throws and still starts the following operation', async () => {
    const queue = new OperationQueue();
    const operation = vi.fn(() => 42);

    const failed = queue.enqueue(() => {
      throw new Error('synchronous failure');
    });
    const next = queue.enqueue(operation);

    await expect(failed).rejects.toThrow('synchronous failure');
    await expect(next).resolves.toBe(42);
    expect(operation).toHaveBeenCalledTimes(1);
  });
});
