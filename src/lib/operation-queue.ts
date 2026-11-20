/** A non-cancelling FIFO queue for operations sharing one physical I²C bus. */
export class OperationQueue {
  private tail: Promise<void> = Promise.resolve();

  enqueue<T>(operation: () => Promise<T> | T): Promise<T> {
    const result = this.tail.then(operation);
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }
}

export const hardwareQueue = new OperationQueue();
