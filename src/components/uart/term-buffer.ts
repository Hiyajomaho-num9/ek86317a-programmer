/**
 * 极简终端缓冲区。
 *
 * 为什么需要它：串口控制台不是纯文本流。shell 删一个字是靠发 `\b \b`，
 * 重画一行是靠 `\r` 回到行首再覆盖，清行是靠 `ESC[K`。
 *
 * 关键点：**光标位置必须单独记录，而且渲染时要插在文本中间**。
 * 如果只把字符堆进 <pre>、光标画在末尾，那么 `\b \b` 擦掉的字会变成一个
 * 留在光标前面的空格 —— 看起来就是"按了退格，光标没退"。
 *
 * 这里实现够用的一小撮 VT100 行为：光标移动、覆盖写、擦除。
 * 不解析颜色（SGR 直接忽略），调试终端不需要。
 */

const MAX_LINES = 2000;
export const MAX_CHARS = 60_000;

/** `ESC [ params final` */
const CSI_RE = /^\u001b\[([0-9;?]*)([@-~])/;

export interface RenderedView {
  /** 光标之前的全部内容 */
  before: string;
  /** 光标之后的内容（含光标所在行的剩余部分） */
  after: string;
  /** 光标位置，方便测试 */
  row: number;
  col: number;
}

export class TermBuffer {
  private lines: string[] = [''];
  private row = 0;
  private col = 0;
  /** 上一条 chunk 结尾半个转义序列，等下一次拼上 */
  private pending = '';

  reset(): void {
    this.lines = [''];
    this.row = 0;
    this.col = 0;
    this.pending = '';
  }

  write(chunk: string): void {
    if (chunk === '') return;
    const text = this.pending + chunk;
    this.pending = '';

    let i = 0;
    while (i < text.length) {
      const ch = text[i];

      if (ch === '\u001b') {
        if (i + 1 >= text.length) {
          this.pending = text.slice(i);
          return;
        }
        if (text[i + 1] === '[') {
          const rest = text.slice(i);
          const match = CSI_RE.exec(rest);
          if (match == null) {
            this.pending = rest;
            return;
          }
          this.applyCsi(match[1], match[2]);
          i += match[0].length;
          continue;
        }
        i += 2;
        continue;
      }

      if (ch === '\n') { this.newline(); i += 1; continue; }
      if (ch === '\r') { this.col = 0; i += 1; continue; }
      if (ch === '\b') {
        // 退格只移动光标。真正把字擦掉的是后面那个覆盖写的空格
        if (this.col > 0) this.col -= 1;
        i += 1;
        continue;
      }
      if (ch === '\u0007') { i += 1; continue; }
      if (ch === '\t') {
        const next = (Math.floor(this.col / 8) + 1) * 8;
        while (this.col < next) this.put(' ');
        i += 1;
        continue;
      }
      if (ch < ' ') { i += 1; continue; }

      this.put(ch);
      i += 1;
    }

    this.trim();
  }

  /** 纯文本（不含光标），测试和日志用。 */
  toString(): string {
    return this.lines.join('\n');
  }

  /**
   * 按真实光标位置把内容切成两段，UI 在中间插光标。
   *
   * 不能用「光标画在末尾」那种偷懒写法 —— 退格之后擦出来的空格会留在
   * 光标前面，看起来就是光标没回退。
   */
  renderParts(): RenderedView {
    const lines = this.lines.slice();
    const curLine = lines[this.row] ?? '';
    // 光标可能停在行尾之外（写过又擦掉的情况），补空格对齐
    const padded = curLine.length < this.col
      ? curLine + ' '.repeat(this.col - curLine.length)
      : curLine;
    lines[this.row] = padded;

    const col = Math.min(this.col, padded.length);
    const beforeLines = lines.slice(0, this.row);
    const before = [...beforeLines, padded.slice(0, col)].join('\n');
    const rest = padded.slice(col);
    const tailLines = lines.slice(this.row + 1);
    const after = tailLines.length === 0 ? rest : `${rest}\n${tailLines.join('\n')}`;

    return { before, after, row: this.row, col };
  }

  private put(ch: string): void {
    const line = this.lines[this.row] ?? '';
    if (this.col < line.length) {
      this.lines[this.row] = line.slice(0, this.col) + ch + line.slice(this.col + 1);
    } else {
      this.lines[this.row] = line + ' '.repeat(this.col - line.length) + ch;
    }
    this.col += 1;
  }

  private newline(): void {
    this.row += 1;
    this.col = 0;
    if (this.lines[this.row] == null) this.lines.push('');
  }

  /** 行数 / 字符数超限时从最前面丢，同时把 row 跟着挪。 */
  private trim(): void {
    while (this.lines.length > MAX_LINES) {
      this.lines.shift();
      this.row = Math.max(0, this.row - 1);
    }
    let total = 0;
    for (const line of this.lines) total += line.length + 1;
    while (total > MAX_CHARS && this.lines.length > 1) {
      total -= this.lines[0].length + 1;
      this.lines.shift();
      this.row = Math.max(0, this.row - 1);
    }
  }

  private applyCsi(rawParams: string, final: string): void {
    const first = rawParams.split(';')[0];
    const count = first === '' ? 0 : Number.parseInt(first, 10);

    switch (final) {
      case 'K': {
        const line = this.lines[this.row] ?? '';
        if (first === '2') {
          this.lines[this.row] = '';
        } else if (first === '1') {
          this.lines[this.row] = ' '.repeat(this.col) + line.slice(this.col);
        } else {
          this.lines[this.row] = line.slice(0, this.col);
        }
        break;
      }
      case 'J': {
        if (first === '2' || first === '3') {
          this.lines = [''];
          this.row = 0;
          this.col = 0;
        } else {
          this.lines = this.lines.slice(0, this.row + 1);
          const line = this.lines[this.row] ?? '';
          this.lines[this.row] = line.slice(0, this.col);
        }
        break;
      }
      case 'D': this.col = Math.max(0, this.col - (count || 1)); break;
      case 'C': this.col += count || 1; break;
      case 'G': this.col = Math.max(0, (count || 1) - 1); break;
      case 'A': this.row = Math.max(0, this.row - (count || 1)); break;
      case 'B': this.row = Math.min(this.lines.length - 1, this.row + (count || 1)); break;
      default:
        // m / h / l / H / r / ? 全部忽略
        break;
    }
  }
}
